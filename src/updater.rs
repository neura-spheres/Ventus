use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use std::io::Write;

use crate::version::{APP_VERSION, USER_AGENT};

pub const CURRENT_VERSION: &str = APP_VERSION;
const GITHUB_OWNER: &str = "neura-spheres";
const GITHUB_REPO: &str = "Ventus";

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub notes: String,
    pub download_url: String,
}

pub async fn check_latest() -> Result<Option<ReleaseInfo>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let resp: serde_json::Value = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?
        .get(&url)
        .send()
        .await?
        .json()
        .await?;

    let tag = resp["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();

    if tag.is_empty() {
        return Err(anyhow!("No releases found on GitHub."));
    }

    let current = semver::Version::parse(CURRENT_VERSION)?;
    let latest = semver::Version::parse(&tag)
        .map_err(|_| anyhow!("Could not parse release version '{}'", tag))?;

    if latest <= current {
        return Ok(None);
    }

    let download_url = resp["assets"]
        .as_array()
        .and_then(|assets| {
            assets
                .iter()
                .find(|a| a["name"].as_str() == Some("neura-search.exe"))
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .unwrap_or("")
        .to_string();

    if download_url.is_empty() {
        return Err(anyhow!(
            "Release v{} has no neura-search.exe asset attached.",
            tag
        ));
    }

    Ok(Some(ReleaseInfo {
        version: latest.to_string(),
        notes: resp["body"].as_str().unwrap_or("").to_string(),
        download_url,
    }))
}

pub async fn download_update(
    url: &str,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<std::path::PathBuf> {
    let resp = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?
        .get(url)
        .send()
        .await?;

    let total = resp.content_length().unwrap_or(0);
    let tmp = std::env::temp_dir().join("ventus-update.exe");
    let mut file = std::fs::File::create(&tmp)?;
    let mut received: u64 = 0;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        received += chunk.len() as u64;
        on_progress(received, total);
    }

    Ok(tmp)
}

pub fn apply_update(new_exe: &std::path::Path) -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let pid = std::process::id();

    let bat_content = format!(
        "@echo off\r\n\
        :WAIT\r\n\
        tasklist /FI \"PID eq {pid}\" 2>NUL | find /I \"{pid}\" >NUL\r\n\
        if not errorlevel 1 (\r\n    timeout /t 1 /nobreak >nul\r\n    goto WAIT\r\n)\r\n\
        copy /y \"{new}\" \"{exe}\"\r\n\
        if errorlevel 1 goto WAIT\r\n\
        start \"\" \"{exe}\"\r\n\
        del \"%~f0\"\r\n",
        pid = pid,
        new = new_exe.to_string_lossy().replace('/', "\\"),
        exe = current_exe.to_string_lossy().replace('/', "\\"),
    );

    let bat_path = std::env::temp_dir().join("ventus-update.bat");
    std::fs::write(&bat_path, bat_content)?;

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("cmd")
            .args(["/c", bat_path.to_str().unwrap_or("")])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
    }
    #[cfg(not(windows))]
    {
        let new_s = new_exe.to_string_lossy();
        let exe_s = current_exe.to_string_lossy();
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "sleep 2 && cp '{}' '{}' && open '{}'",
                new_s, exe_s, exe_s
            ))
            .spawn()?;
    }

    std::process::exit(0);
}
