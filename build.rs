fn main() {
    let version = app_version();
    println!("cargo:rerun-if-changed=config.yaml");
    println!("cargo:rustc-env=NEURA_APP_VERSION={version}");

    emit_cloud_env();

    let png_path = std::path::Path::new("public/ventus.png");
    if !png_path.exists() {
        return;
    }
    let png_bytes = match std::fs::read(png_path) {
        Ok(b) => b,
        Err(_) => return,
    };

    let assets_dir = std::path::Path::new("assets");
    std::fs::create_dir_all(assets_dir).ok();
    let ico_path = assets_dir.join("logo.ico");
    let resized = resize_png_256(&png_bytes);
    std::fs::write(&ico_path, build_png_ico(&resized)).ok();

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon(ico_path.to_string_lossy().as_ref());
        res.set("ProductName", "Ventus");
        res.set("FileDescription", "Ventus");
        res.set("ProductVersion", &version);
        res.set("FileVersion", &version);
        if let Some(v) = version_info(&version) {
            res.set_version_info(winres::VersionInfo::FILEVERSION, v);
            res.set_version_info(winres::VersionInfo::PRODUCTVERSION, v);
        }
        if let Err(e) = res.compile() {
            println!("cargo:warning=winres compile failed: {e}");
        }
    }

    println!("cargo:rerun-if-changed=public/ventus.png");
}

fn emit_cloud_env() {
    println!("cargo:rerun-if-changed=cloud.env");
    let Ok(contents) = std::fs::read_to_string("cloud.env") else {
        return;
    };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            continue;
        }
        println!("cargo:rustc-env=VENTUS_{key}={value}");
    }
}

fn app_version() -> String {
    let cfg = std::fs::read_to_string("config.yaml").expect("read config.yaml");
    for line in cfg.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("version:") else {
            continue;
        };
        let version = rest.trim().trim_matches('"').trim_matches('\'');
        if !version.is_empty() {
            return version.to_string();
        }
    }
    panic!("config.yaml must contain version: x.y.z");
}

fn version_info(version: &str) -> Option<u64> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    Some((major << 48) | (minor << 32) | (patch << 16))
}

fn resize_png_256(png: &[u8]) -> Vec<u8> {
    use image::imageops::FilterType;
    let img = match image::load_from_memory(png) {
        Ok(i) => i,
        Err(_) => return png.to_vec(),
    };
    let resized = img.resize_exact(256, 256, FilterType::Lanczos3);
    let mut buf = Vec::new();
    resized
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .ok();
    buf
}

fn build_png_ico(png: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(22 + png.len());
    out.extend_from_slice(&[0u8, 0, 1, 0, 1, 0]);
    out.push(0);
    out.push(0);
    out.push(0);
    out.push(0);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&(png.len() as u32).to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes());
    out.extend_from_slice(png);
    out
}
