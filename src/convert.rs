

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use image::{Rgba, RgbaImage, imageops};
use resvg::{tiny_skia, usvg};



pub const ICON_SIZES: [u32; 10] = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256];


pub const DEFAULT_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

const MAX_PNG_PIXELS: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceFormat {
    Svg,
    Png,
}

impl SourceFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Svg => "SVG",
            Self::Png => "PNG",
        }
    }
}

impl fmt::Display for SourceFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

enum SourceData {
    Svg(Box<usvg::Tree>),
    Png(Arc<RgbaImage>),
}


pub struct IconSource {
    pub path: PathBuf,
    pub file_name: String,
    pub format: SourceFormat,
    pub width: f32,
    pub height: f32,
    pub file_size: u64,
    data: SourceData,
}

impl IconSource {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .map_err(|err| format!("Could not read {}: {err}", path.display()))?;
        let file_size = std::fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(bytes.len() as u64);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
            .to_owned();

        if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
            Self::from_png(path, file_name, bytes, file_size)
        } else if looks_like_svg(&bytes) {
            Self::from_svg(path, file_name, bytes, file_size)
        } else {
            Err("That file is not an SVG or PNG image.".to_owned())
        }
    }

    fn from_png(
        path: &Path,
        file_name: String,
        bytes: Vec<u8>,
        file_size: u64,
    ) -> Result<Self, String> {
        validate_png_dimensions(&bytes)?;
        let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .map_err(|err| format!("Could not decode this PNG: {err}"))?
            .to_rgba8();
        if image.width() == 0 || image.height() == 0 {
            return Err("The PNG has no pixels.".to_owned());
        }

        Ok(Self {
            path: path.to_owned(),
            file_name,
            format: SourceFormat::Png,
            width: image.width() as f32,
            height: image.height() as f32,
            file_size,
            data: SourceData::Png(Arc::new(image)),
        })
    }

    fn from_svg(
        path: &Path,
        file_name: String,
        bytes: Vec<u8>,
        file_size: u64,
    ) -> Result<Self, String> {
        let mut options = usvg::Options::default();
        options.resources_dir = path.parent().map(Path::to_path_buf);
        options.font_family = "Segoe UI".to_owned();
        options.fontdb_mut().load_system_fonts();

        let tree = usvg::Tree::from_data(&bytes, &options)
            .map_err(|err| format!("Could not parse this SVG: {err}"))?;
        let size = tree.size();
        if !size.width().is_finite()
            || !size.height().is_finite()
            || size.width() <= 0.0
            || size.height() <= 0.0
        {
            return Err("The SVG has an invalid canvas size.".to_owned());
        }

        Ok(Self {
            path: path.to_owned(),
            file_name,
            format: SourceFormat::Svg,
            width: size.width(),
            height: size.height(),
            file_size,
            data: SourceData::Svg(Box::new(tree)),
        })
    }





    pub fn render(&self, size: u32, padding: f32) -> Result<RgbaImage, String> {
        if size == 0 || size > 512 {
            return Err(format!("Unsupported frame size: {size} px"));
        }

        let padding_fraction = (padding / 100.0).clamp(0.0, 0.45);
        let content_size =
            ((size as f32) * (1.0 - padding_fraction * 2.0)).round().max(1.0) as u32;

        match &self.data {
            SourceData::Svg(tree) => render_svg(tree, size, content_size),
            SourceData::Png(image) => render_png(image, size, content_size),
        }
    }

    pub fn native_size_label(&self) -> String {
        format_dimension_pair(self.width, self.height)
    }

    pub fn is_low_resolution(&self) -> bool {
        self.format == SourceFormat::Png && (self.width < 128.0 || self.height < 128.0)
    }
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let prefix_len = bytes.len().min(4096);
    let Ok(text) = std::str::from_utf8(&bytes[..prefix_len]) else {
        return false;
    };
    let text = text.trim_start_matches('\u{feff}').trim_start();
    text.starts_with('<') && text.to_ascii_lowercase().contains("<svg")
}

fn validate_png_dimensions(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
        return Err("This PNG is missing its image header.".to_owned());
    }

    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if width == 0 || height == 0 {
        return Err("The PNG has an invalid size.".to_owned());
    }
    if u64::from(width) * u64::from(height) > MAX_PNG_PIXELS {
        return Err("This PNG is too large. Please use an image under 64 megapixels.".to_owned());
    }
    Ok(())
}

fn render_svg(tree: &usvg::Tree, size: u32, content_size: u32) -> Result<RgbaImage, String> {
    let source = tree.size();
    let scale =
        (content_size as f32 / source.width()).min(content_size as f32 / source.height());
    let drawn_width = source.width() * scale;
    let drawn_height = source.height() * scale;
    let offset_x = (size as f32 - drawn_width) * 0.5;
    let offset_y = (size as f32 - drawn_height) * 0.5;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| "Could not allocate the SVG preview.".to_owned())?;
    let transform = tiny_skia::Transform::from_translate(offset_x, offset_y)
        .pre_scale(scale, scale);
    resvg::render(tree, transform, &mut pixmap.as_mut());



    let bytes = pixmap.take_demultiplied();
    RgbaImage::from_raw(size, size, bytes)
        .ok_or_else(|| "Could not create the rendered SVG frame.".to_owned())
}

fn render_png(source: &RgbaImage, size: u32, content_size: u32) -> Result<RgbaImage, String> {
    let source_width = source.width() as f32;
    let source_height = source.height() as f32;
    let scale = (content_size as f32 / source_width)
        .min(content_size as f32 / source_height);
    let drawn_width = (source_width * scale).round().max(1.0) as u32;
    let drawn_height = (source_height * scale).round().max(1.0) as u32;

    let resized = resize_premultiplied(source, drawn_width, drawn_height);
    let mut canvas = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    let x = ((size - drawn_width) / 2) as i64;
    let y = ((size - drawn_height) / 2) as i64;
    imageops::overlay(&mut canvas, &resized, x, y);
    Ok(canvas)
}




fn resize_premultiplied(source: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let mut premultiplied = RgbaImage::new(source.width(), source.height());
    for (destination, pixel) in premultiplied.pixels_mut().zip(source.pixels()) {
        let alpha = u16::from(pixel[3]);
        *destination = Rgba([
            ((u16::from(pixel[0]) * alpha + 127) / 255) as u8,
            ((u16::from(pixel[1]) * alpha + 127) / 255) as u8,
            ((u16::from(pixel[2]) * alpha + 127) / 255) as u8,
            pixel[3],
        ]);
    }

    let mut resized = imageops::resize(
        &premultiplied,
        width,
        height,
        imageops::FilterType::Lanczos3,
    );
    for pixel in resized.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 {
            *pixel = Rgba([0, 0, 0, 0]);
        } else {
            pixel[0] = ((u16::from(pixel[0]) * 255 + alpha / 2) / alpha).min(255) as u8;
            pixel[1] = ((u16::from(pixel[1]) * 255 + alpha / 2) / alpha).min(255) as u8;
            pixel[2] = ((u16::from(pixel[2]) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
    resized
}





pub fn export_ico(
    source: &IconSource,
    sizes: &[u32],
    padding: f32,
    destination: &Path,
) -> Result<u64, String> {
    let mut unique_sizes = sizes.to_vec();
    unique_sizes.sort_unstable();
    unique_sizes.dedup();
    unique_sizes.retain(|size| (1..=256).contains(size));

    if unique_sizes.is_empty() {
        return Err("Select at least one icon size first.".to_owned());
    }

    let mut directory = ico::IconDir::new(ico::ResourceType::Icon);
    for size in &unique_sizes {
        let image = source.render(*size, padding)?;
        let icon = ico::IconImage::from_rgba_data(*size, *size, image.into_raw());
        let entry = if *size >= 64 {
            ico::IconDirEntry::encode_as_png(&icon)
        } else {
            ico::IconDirEntry::encode_as_bmp(&icon)
        }
        .map_err(|err| format!("Could not encode the {size} px frame: {err}"))?;
        directory.add_entry(entry);
    }

    let mut encoded = Vec::new();
    directory
        .write(&mut encoded)
        .map_err(|err| format!("Could not assemble the ICO: {err}"))?;

    if let Some(parent) = destination.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|err| {
                format!("Could not create {}: {err}", parent.display())
            })?;
        }
    }

    std::fs::write(destination, &encoded)
        .map_err(|err| format!("Could not save {}: {err}", destination.display()))?;
    Ok(encoded.len() as u64)
}

pub fn format_dimension_pair(width: f32, height: f32) -> String {
    format!(
        "{} x {} px",
        format_dimension(width),
        format_dimension(height)
    )
}

pub fn format_dimension(value: f32) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{:.0}", value)
    } else {
        format!("{value:.1}")
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}

/* #[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
        <defs>
            <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0" stop-color="#a78bfa"/>
                <stop offset="1" stop-color="#22d3ee"/>
            </linearGradient>
        </defs>
        <rect x="36" y="36" width="440" height="440" rx="104" fill="url(#g)"/>
        <circle cx="256" cy="224" r="92" fill="#fff" fill-opacity=".92"/>
    </svg>"##;

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("iconbv-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn renders_svg_at_multiple_sizes() {
        let path = temp_file("render.svg");
        std::fs::write(&path, SAMPLE_SVG).unwrap();
        let source = IconSource::load(&path).unwrap();
        for size in ICON_SIZES {
            let image = source.render(size, 8.0).unwrap();
            assert_eq!(image.dimensions(), (size, size));
            assert!(image.pixels().any(|pixel| pixel[3] > 0));
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn writes_a_valid_multi_size_ico() {
        let svg_path = temp_file("icon.svg");
        let ico_path = temp_file("icon.ico");
        std::fs::write(&svg_path, SAMPLE_SVG).unwrap();
        let source = IconSource::load(&svg_path).unwrap();
        let bytes = export_ico(&source, &DEFAULT_SIZES, 8.0, &ico_path).unwrap();
        assert!(bytes > 0);

        let file = std::fs::File::open(&ico_path).unwrap();
        let icon = ico::IconDir::read(file).unwrap();
        assert_eq!(icon.entries().len(), DEFAULT_SIZES.len());
        let sizes: Vec<_> = icon.entries().iter().map(ico::IconDirEntry::width).collect();
        assert_eq!(sizes, DEFAULT_SIZES);

        let _ = std::fs::remove_file(svg_path);
        let _ = std::fs::remove_file(ico_path);
    }

    #[test]
    fn png_frames_keep_transparency() {
        let mut png = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 16..48 {
            for x in 16..48 {
                png.put_pixel(x, y, Rgba([255, 80, 160, 255]));
            }
        }
        let png_path = temp_file("pixels.png");
        png.save(&png_path).unwrap();
        let source = IconSource::load(&png_path).unwrap();
        let rendered = source.render(32, 0.0).unwrap();
        assert_eq!(rendered.get_pixel(0, 0)[3], 0);
        assert!(rendered.get_pixel(16, 16)[3] > 0);

        let _ = std::fs::remove_file(png_path);
    }
}
 */