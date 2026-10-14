//! Create a macOS icon container from the original window-icon vector geometry.
use std::io::Write;
fn main() -> anyhow::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Usage: macos_icon OUTPUT.icns"))?;
    let mut payload = Vec::new();
    for (size, kind) in [
        (128, b"ic07"),
        (256, b"ic08"),
        (512, b"ic09"),
        (1024, b"ic10"),
    ] {
        let icon = strata_app::branding::icon_at_size(size);
        let scaled = image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(size, size, icon.rgba).unwrap(),
        );
        let mut png = std::io::Cursor::new(Vec::new());
        scaled.write_to(&mut png, image::ImageFormat::Png)?;
        let png = png.into_inner();
        payload.extend(kind);
        payload.extend(((png.len() + 8) as u32).to_be_bytes());
        payload.extend(png);
    }
    let mut output = std::fs::File::create(path)?;
    output.write_all(b"icns")?;
    output.write_all(&((payload.len() + 8) as u32).to_be_bytes())?;
    output.write_all(&payload)?;
    output.sync_all()?;
    Ok(())
}
