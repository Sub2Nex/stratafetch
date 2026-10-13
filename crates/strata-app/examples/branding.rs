//! Rebuild native installer assets from the same geometry as the desktop window icon.
fn main() -> anyhow::Result<()> {
    let icon = strata_app::branding::icon();
    image::save_buffer(
        "assets/stratafetch.png",
        &icon.rgba,
        128,
        128,
        image::ColorType::Rgba8,
    )?;
    let mut bitmap = Vec::new();
    for value in [40u32, 128, 256] {
        bitmap.extend(value.to_le_bytes());
    }
    bitmap.extend(1u16.to_le_bytes());
    bitmap.extend(32u16.to_le_bytes());
    for value in [0u32, 128 * 128 * 4, 0, 0, 0, 0] {
        bitmap.extend(value.to_le_bytes());
    }
    for y in (0..128).rev() {
        for x in 0..128 {
            let pixel = &icon.rgba[(y * 128 + x) * 4..][..4];
            bitmap.extend([pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    for y in (0..128).rev() {
        for byte in 0..16 {
            let mut mask = 0u8;
            for bit in 0..8 {
                if icon.rgba[(y * 128 + byte * 8 + bit) * 4 + 3] == 0 {
                    mask |= 1 << (7 - bit);
                }
            }
            bitmap.push(mask);
        }
    }
    let mut ico = vec![0, 0, 1, 0, 1, 0, 128, 128, 0, 0, 1, 0, 32, 0];
    ico.extend((bitmap.len() as u32).to_le_bytes());
    ico.extend(22u32.to_le_bytes());
    ico.extend(bitmap);
    std::fs::write("assets/stratafetch.ico", ico)?;
    Ok(())
}
