//! Rasterization of the original repository SVG geometry for the native window icon.
pub fn icon() -> eframe::egui::IconData {
    icon_at_size(128)
}
/// Rasterize original vector geometry directly at the requested DPI.
pub fn icon_at_size(size: u32) -> eframe::egui::IconData {
    assert!((1..=1024).contains(&size));
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    let distance = |x: f32, y: f32, a: (f32, f32), b: (f32, f32)| {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let t = ((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy);
        let t = t.clamp(0.0, 1.0);
        ((x - a.0 - t * dx).powi(2) + (y - a.1 - t * dy).powi(2)).sqrt()
    };
    for y in 0..size {
        for x in 0..size {
            let (x, y) = (
                (x as f32 + 0.5) * 128.0 / size as f32,
                (y as f32 + 0.5) * 128.0 / size as f32,
            );
            let cx = x.clamp(28.0, 100.0);
            let cy = y.clamp(28.0, 100.0);
            let mut color = if (x - cx).hypot(y - cy) <= 28.0 {
                [21, 32, 44, 255]
            } else {
                [0, 0, 0, 0]
            };
            for (a, b) in [
                ((28.0, 32.0), (100.0, 32.0)),
                ((28.0, 50.0), (100.0, 50.0)),
                ((28.0, 68.0), (50.0, 68.0)),
                ((78.0, 68.0), (100.0, 68.0)),
            ] {
                if distance(x, y, a, b) <= 4.0 {
                    color = [73, 204, 194, 255];
                }
            }
            for (a, b) in [
                ((64.0, 58.0), (64.0, 98.0)),
                ((48.0, 82.0), (64.0, 98.0)),
                ((64.0, 98.0), (80.0, 82.0)),
            ] {
                if distance(x, y, a, b) <= 4.0 {
                    color = [255, 255, 255, 255];
                }
            }
            rgba.extend_from_slice(&color);
        }
    }
    eframe::egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}
