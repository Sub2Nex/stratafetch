//! Shared artwork for native windows, installers and browser extensions.
pub fn icon() -> eframe::egui::IconData {
    icon_at_size(128)
}
pub fn icon_at_size(size: u32) -> eframe::egui::IconData {
    use std::sync::OnceLock;
    static MASTER: OnceLock<image::RgbaImage> = OnceLock::new();
    assert!((1..=1024).contains(&size));
    let master = MASTER.get_or_init(|| {
        image::load_from_memory(include_bytes!("../../../assets/branding/icon-512.png"))
            .expect("validated bundled icon")
            .into_rgba8()
    });
    let image = image::imageops::resize(master, size, size, image::imageops::FilterType::Lanczos3);
    eframe::egui::IconData {
        rgba: image.into_raw(),
        width: size,
        height: size,
    }
}
