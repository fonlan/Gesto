use std::{env, error::Error, fs::File, path::PathBuf};

use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::imageops::FilterType;

const ICON_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=logo.png");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }

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
