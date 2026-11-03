#![cfg_attr(not(feature = "console"), windows_subsystem = "windows")]

mod app;
mod convert;
mod theme;

use std::path::PathBuf;

use eframe::egui;


const DESIGN_WIDTH: f32 = 1180.0;

const DESIGN_HEIGHT: f32 = 700.0;

const SCREEN_MARGIN: f32 = 24.0;

fn main() -> eframe::Result<()> {
    let initial_source = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|path| path.is_file());

    let (work_left, work_top, work_width, work_height, dpi) = desktop_metrics();
    let scale = (dpi / 96.0).max(0.5);
    let available_width = (work_width - SCREEN_MARGIN) / scale;
    let available_height = (work_height - SCREEN_MARGIN) / scale;



    let zoom = (available_width / DESIGN_WIDTH)
        .min(available_height / DESIGN_HEIGHT)
        .clamp(0.60, 1.0);
    let size = egui::vec2(DESIGN_WIDTH * zoom, DESIGN_HEIGHT * zoom);
    let position_x = (work_left / scale + (work_width / scale - size.x) * 0.5).max(0.0);
    let position_y = (work_top / scale + (work_height / scale - size.y) * 0.5).max(0.0);
    let position = egui::pos2(position_x, position_y);

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("ICONBV \u{2014} Windows Icon Studio")
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_position(position)
            .with_icon(build_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "iconbv",
        options,
        Box::new(move |creation| {
            Ok(Box::new(app::IconApp::new(creation, zoom, initial_source)))
        }),
    )
}



#[cfg(target_os = "windows")]
fn desktop_metrics() -> (f32, f32, f32, f32, f32) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETWORKAREA, SystemParametersInfoW};

    unsafe {
        let mut rect = RECT::default();
        let ok = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut rect as *mut RECT as *mut core::ffi::c_void,
            0,
        );
        let dpi = GetDpiForSystem() as f32;
        let dpi = if dpi > 0.0 { dpi } else { 96.0 };
        if ok != 0 && rect.right > rect.left && rect.bottom > rect.top {
            return (
                rect.left as f32,
                rect.top as f32,
                (rect.right - rect.left) as f32,
                (rect.bottom - rect.top) as f32,
                dpi,
            );
        }
        (0.0, 0.0, 1280.0, 720.0, dpi)
    }
}

#[cfg(not(target_os = "windows"))]
fn desktop_metrics() -> (f32, f32, f32, f32, f32) {
    (0.0, 0.0, 1600.0, 1000.0, 96.0)
}


fn build_icon() -> egui::IconData {
    const SIZE: usize = 256;
    const OUTER: f32 = 10.0;
    const RADIUS: f32 = 58.0;

    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let gradient_span = SIZE as f32 - OUTER * 2.0;


    let tile_area = 62.0;
    let tile_size = 58.0;
    let tile_gap = (tile_area * 2.0 - tile_size * 2.0) / 1.0;
    let origin = (SIZE as f32 - (tile_size * 2.0 + tile_gap)) * 0.5;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let coverage = rounded_rect_coverage(
                px,
                py,
                OUTER,
                OUTER,
                SIZE as f32 - OUTER,
                SIZE as f32 - OUTER,
                RADIUS,
            );
            if coverage <= 0.0 {
                continue;
            }

            let t = ((px - OUTER) / gradient_span).clamp(0.0, 1.0);
            let mut color = [
                lerp(242.0, 126.0, t),
                lerp(243.0, 130.0, t),
                lerp(246.0, 137.0, t),
            ];

            for row in 0..2 {
                for column in 0..2 {
                    let left = origin + column as f32 * (tile_size + tile_gap);
                    let top = origin + row as f32 * (tile_size + tile_gap);
                    let tile_coverage = rounded_rect_coverage(
                        px,
                        py,
                        left,
                        top,
                        left + tile_size,
                        top + tile_size,
                        12.0,
                    );
                    if tile_coverage > 0.0 {
                        let alpha = if row == 1 && column == 1 { 0.72 } else { 0.94 };
                        let blend = tile_coverage * alpha;
                        for channel in 0..3 {
                            color[channel] = lerp(color[channel], 255.0, blend);
                        }
                    }
                }
            }

            let alpha = (coverage * 255.0).round() as u8;
            let offset = (y * SIZE + x) * 4;
            rgba[offset] = color[0].round() as u8;
            rgba[offset + 1] = color[1].round() as u8;
            rgba[offset + 2] = color[2].round() as u8;
            rgba[offset + 3] = alpha;
        }
    }

    egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}


fn rounded_rect_coverage(
    px: f32,
    py: f32,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    radius: f32,
) -> f32 {
    let half_width = (right - left) * 0.5;
    let half_height = (bottom - top) * 0.5;
    let center_x = (left + right) * 0.5;
    let center_y = (top + bottom) * 0.5;
    let radius = radius.min(half_width).min(half_height);

    let dx = (px - center_x).abs() - (half_width - radius);
    let dy = (py - center_y).abs() - (half_height - radius);
    let outside = dx.max(0.0).hypot(dy.max(0.0));
    let distance = outside + dx.max(dy).min(0.0) - radius;

    (0.5 - distance).clamp(0.0, 1.0)
}
