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

pub async fn check_latest(beta: bool) -> Result<Option<ReleaseInfo>> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(12))
        .build()?;

    let current = semver::Version::parse(CURRENT_VERSION)?;

    if !beta {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            GITHUB_OWNER, GITHUB_REPO
        );
        let resp: serde_json::Value = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        return resolve(&resp, &current);
    }

    let url = format!(
        "https://api.github.com/repos/{}/{}/releases?per_page=30",
        GITHUB_OWNER, GITHUB_REPO
    );
    let releases: serde_json::Value = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let Some(list) = releases.as_array() else {
        return Ok(None);
    };

    let newest = list
        .iter()
        .filter(|r| r["draft"].as_bool() != Some(true))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?.trim_start_matches('v');
            semver::Version::parse(tag).ok().map(|v| (v, r))
        })
        .max_by(|a, b| a.0.cmp(&b.0));

    let Some((_, release)) = newest else {
        return Ok(None);
    };

    resolve(release, &current)
}

fn resolve(release: &serde_json::Value, current: &semver::Version) -> Result<Option<ReleaseInfo>> {
    let tag = release["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();

    if tag.is_empty() {
        return Err(anyhow!("No releases found on GitHub."));
    }

    let latest = semver::Version::parse(&tag)
        .map_err(|_| anyhow!("Could not parse release version '{}'", tag))?;

    if latest <= *current {
        return Ok(None);
    }

    let download_url = pick_asset(&release["assets"], &latest).unwrap_or_default();
    if download_url.is_empty() {
        return Err(anyhow!(
            "Release v{} has no Ventus installer or ventus.exe asset attached.",
            tag
        ));
    }

    Ok(Some(ReleaseInfo {
        version: latest.to_string(),
        notes: release["body"].as_str().unwrap_or("").to_string(),
        download_url,
    }))
}

fn pick_asset(assets: &serde_json::Value, version: &semver::Version) -> Option<String> {
    let assets = assets.as_array()?;
    let installer_name = format!("Ventus-Setup-{}.exe", version);
    assets
        .iter()
        .find(|a| a["name"].as_str() == Some(&installer_name))
        .or_else(|| {
            assets.iter().find(|a| {
                let name = a["name"].as_str().unwrap_or("");
                name.starts_with("Ventus-Setup-") && name.ends_with(".exe")
            })
        })
        .or_else(|| {
            assets
                .iter()
                .find(|a| a["name"].as_str() == Some("ventus.exe"))
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .map(|s| s.to_string())
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
    let file_name = update_file_name(url);
    let tmp = std::env::temp_dir().join(file_name);
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

    let new = new_exe.to_string_lossy().replace('/', "\\");
    let exe = current_exe.to_string_lossy().replace('/', "\\");
    let is_installer = new_exe
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_ascii_lowercase().contains("setup"))
        .unwrap_or(false);

    let bat_content = if is_installer {
        format!(
            "@echo off\r\n\
            :WAIT\r\n\
            tasklist /FI \"PID eq {pid}\" 2>NUL | find /I \"{pid}\" >NUL\r\n\
            if not errorlevel 1 (\r\n    timeout /t 1 /nobreak >nul\r\n    goto WAIT\r\n)\r\n\
            \"{new}\" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART\r\n\
            if errorlevel 1 goto WAIT\r\n\
            start \"\" \"{exe}\" --restore-session\r\n\
            del \"{new}\"\r\n\
            del \"%~f0\"\r\n",
            pid = pid,
            new = new,
            exe = exe,
        )
    } else {
        format!(
            "@echo off\r\n\
            :WAIT\r\n\
            tasklist /FI \"PID eq {pid}\" 2>NUL | find /I \"{pid}\" >NUL\r\n\
            if not errorlevel 1 (\r\n    timeout /t 1 /nobreak >nul\r\n    goto WAIT\r\n)\r\n\
            copy /y \"{new}\" \"{exe}\"\r\n\
            if errorlevel 1 goto WAIT\r\n\
            start \"\" \"{exe}\" --restore-session\r\n\
            del \"{new}\"\r\n\
            del \"%~f0\"\r\n",
            pid = pid,
            new = new,
            exe = exe,
        )
    };

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
    Ok(())
}

fn update_file_name(url: &str) -> String {
    let name = reqwest::Url::parse(url)
        .ok()
        .and_then(|url| {
            url.path_segments()
                .and_then(|mut parts| parts.next_back().map(|v| v.to_string()))
        })
        .filter(|name| name.to_ascii_lowercase().ends_with(".exe"))
        .unwrap_or_else(|| "ventus-update.exe".to_string());

    let clean: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();

    if clean.is_empty() {
        "ventus-update.exe".to_string()
    } else {
        clean
    }
}

#[cfg(test)]
mod tests {
    use semver::Version;

    fn v(s: &str) -> Version {
        Version::parse(s).expect("valid version")
    }

    #[test]
    fn prerelease_chain_orders_chronologically() {
        let chain = [
            "1.0.33",
            "1.0.34-pre26062201",
            "1.0.34-pre26062202",
            "1.0.34-pre26070101",
            "1.0.34",
        ];
        for pair in chain.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} should be < {}", pair[0], pair[1]);
        }
    }

    #[test]
    fn stable_beats_its_prerelease() {
        assert!(v("1.0.34") > v("1.0.34-pre26070101"));
    }

    #[test]
    fn picks_highest_when_releases_unsorted() {
        let mut versions = [
            v("1.0.34-pre26062202"),
            v("1.0.34"),
            v("1.0.33"),
            v("1.0.34-pre26062201"),
        ];
        versions.sort();
        assert_eq!(versions.last().unwrap(), &v("1.0.34"));
    }

    #[test]
    fn pick_asset_prefers_exact_installer() {
        let assets = serde_json::json!([
            {"name": "ventus.exe", "browser_download_url": "u-raw"},
            {"name": "Ventus-Setup-1.0.34-pre26062201.exe", "browser_download_url": "u-exact"}
        ]);
        let url = super::pick_asset(&assets, &v("1.0.34-pre26062201"));
        assert_eq!(url.as_deref(), Some("u-exact"));
    }
}
