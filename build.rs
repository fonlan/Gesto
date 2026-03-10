use std::{
    env,
    error::Error,
    ffi::OsString,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::imageops::FilterType;

const ICON_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];
const WEB_INPUTS: [&str; 8] = [
    "web/src",
    "web/index.html",
    "web/package.json",
    "web/package-lock.json",
    "web/postcss.config.js",
    "web/tailwind.config.ts",
    "web/tsconfig.json",
    "web/tsconfig.node.json",
];
const WEB_OPTIONAL_INPUTS: [&str; 1] = ["web/vite.config.ts"];

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=logo.png");
    println!("cargo:rerun-if-changed=web/dist");
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=NPM");

    for input in WEB_INPUTS {
        println!("cargo:rerun-if-changed={input}");
    }

    for input in WEB_OPTIONAL_INPUTS {
        if Path::new(input).exists() {
            println!("cargo:rerun-if-changed={input}");
        }
    }

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    ensure_web_dist(&manifest_dir)?;

    let source = image::open("logo.png")?.into_rgba8();
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let icon_path = out_dir.join("logo.ico");

    let mut icon_dir = IconDir::new(ResourceType::Icon);
    for size in ICON_SIZES {
        let resized = image::imageops::resize(&source, size, size, FilterType::Lanczos3);
        let icon_image = IconImage::from_rgba_data(size, size, resized.into_raw());
        icon_dir.add_entry(IconDirEntry::encode(&icon_image)?);
    }

    let mut icon_file = File::create(&icon_path)?;
    icon_dir.write(&mut icon_file)?;

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(icon_path.to_string_lossy().as_ref());
    resource.compile()?;

    Ok(())
}

fn ensure_web_dist(manifest_dir: &Path) -> Result<(), Box<dyn Error>> {
    let web_dir = manifest_dir.join("web");
    let dist_dir = web_dir.join("dist");

    if !frontend_build_needed(&web_dir, &dist_dir)? {
        return Ok(());
    }

    if frontend_install_needed(&web_dir)? {
        println!("cargo:warning=Installing frontend dependencies...");
        run_npm(manifest_dir, ["--prefix", "web", "install"])?;
    }

    println!("cargo:warning=Building frontend assets...");
    run_npm(manifest_dir, ["--prefix", "web", "run", "build"])?;
    Ok(())
}

fn frontend_build_needed(web_dir: &Path, dist_dir: &Path) -> Result<bool, Box<dyn Error>> {
    if !dist_dir.exists() {
        return Ok(true);
    }

    let latest_input = latest_frontend_input_time(web_dir)?;
    let latest_output = latest_modified_recursive(dist_dir)?;

    Ok(match (latest_input, latest_output) {
        (_, None) => true,
        (Some(input_time), Some(output_time)) => input_time > output_time,
        (None, Some(_)) => false,
    })
}

fn frontend_install_needed(web_dir: &Path) -> Result<bool, Box<dyn Error>> {
    let node_modules_dir = web_dir.join("node_modules");
    if !node_modules_dir.exists() {
        return Ok(true);
    }

    let latest_package_input = latest_modified(&[
        web_dir.join("package.json"),
        web_dir.join("package-lock.json"),
    ])?;
    let node_modules_time = modified_time(&node_modules_dir)?;

    Ok(match (latest_package_input, node_modules_time) {
        (_, None) => true,
        (Some(package_time), Some(node_modules_time)) => package_time > node_modules_time,
        (None, Some(_)) => false,
    })
}

fn latest_frontend_input_time(web_dir: &Path) -> Result<Option<SystemTime>, Box<dyn Error>> {
    let mut paths = vec![
        web_dir.join("src"),
        web_dir.join("index.html"),
        web_dir.join("package.json"),
        web_dir.join("package-lock.json"),
        web_dir.join("postcss.config.js"),
        web_dir.join("tailwind.config.ts"),
        web_dir.join("tsconfig.json"),
        web_dir.join("tsconfig.node.json"),
    ];

    let vite_config = web_dir.join("vite.config.ts");
    if vite_config.exists() {
        paths.push(vite_config);
    }

    latest_modified(&paths)
}

fn latest_modified(paths: &[PathBuf]) -> Result<Option<SystemTime>, Box<dyn Error>> {
    let mut latest = None;

    for path in paths {
        latest = max_time(latest, latest_modified_recursive(path)?);
    }

    Ok(latest)
}

fn latest_modified_recursive(path: &Path) -> Result<Option<SystemTime>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(None);
    }

    let metadata = fs::metadata(path)?;
    let mut latest = Some(metadata.modified()?);

    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            latest = max_time(latest, latest_modified_recursive(&entry.path())?);
        }
    }

    Ok(latest)
}

fn modified_time(path: &Path) -> Result<Option<SystemTime>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(None);
    }

    Ok(Some(fs::metadata(path)?.modified()?))
}

fn max_time(current: Option<SystemTime>, next: Option<SystemTime>) -> Option<SystemTime> {
    match (current, next) {
        (Some(current), Some(next)) => Some(current.max(next)),
        (Some(current), None) => Some(current),
        (None, Some(next)) => Some(next),
        (None, None) => None,
    }
}

fn run_npm<I, S>(manifest_dir: &Path, args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let npm = env::var_os("NPM").unwrap_or_else(|| {
        if cfg!(windows) {
            OsString::from("npm.cmd")
        } else {
            OsString::from("npm")
        }
    });

    let status = Command::new(&npm)
        .args(args.into_iter().map(Into::into))
        .current_dir(manifest_dir)
        .status()
        .map_err(|error| {
            format!(
                "failed to start {:?}: {error}. Install Node.js/npm or set the NPM environment variable",
                npm
            )
        })?;

    if !status.success() {
        return Err(format!("npm command exited with status {status}").into());
    }

    Ok(())
}
