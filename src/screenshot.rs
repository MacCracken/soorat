//! Screenshot capture from GPU render targets via selah.
//!
//! Bridges soorat's [`RenderTarget`] GPU readback with selah's screenshot
//! processing pipeline — annotation, redaction, file saving, and clipboard.
//!
//! Requires the `screenshot` feature.

use crate::error::{RenderError, Result};
use crate::render_target::RenderTarget;
use std::io::Cursor;
use std::path::Path;

/// Supported output formats for screenshot encoding.
///
/// Maps to both `image` crate codecs and `selah::ImageFormat`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScreenshotFormat {
    #[default]
    Png,
    Jpeg,
    Bmp,
}

impl ScreenshotFormat {
    /// File extension for this format.
    #[must_use]
    #[inline]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Bmp => "bmp",
        }
    }

    #[inline]
    fn to_image_format(self) -> image::ImageFormat {
        match self {
            Self::Png => image::ImageFormat::Png,
            Self::Jpeg => image::ImageFormat::Jpeg,
            Self::Bmp => image::ImageFormat::Bmp,
        }
    }
}

/// Encode raw RGBA8 pixels into an image format.
///
/// This is the core bridge between soorat's GPU readback (`Vec<u8>` RGBA)
/// and selah's image-bytes input.
#[must_use = "encoded bytes are returned, not written anywhere"]
pub fn encode_pixels(
    width: u32,
    height: u32,
    rgba: &[u8],
    format: ScreenshotFormat,
) -> Result<Vec<u8>> {
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(RenderError::Screenshot(format!(
            "pixel buffer size mismatch: expected {}x{}x4={expected}, got {}",
            width,
            height,
            rgba.len()
        )));
    }

    let mut buf = Cursor::new(Vec::with_capacity(rgba.len() / 2));

    match format {
        // JPEG doesn't support alpha — strip to RGB
        ScreenshotFormat::Jpeg => {
            let rgb: Vec<u8> = rgba
                .chunks_exact(4)
                .flat_map(|px| [px[0], px[1], px[2]])
                .collect();
            let img = image::RgbImage::from_raw(width, height, rgb)
                .ok_or_else(|| RenderError::Screenshot("RGB conversion failed".into()))?;
            img.write_to(&mut buf, image::ImageFormat::Jpeg)
                .map_err(|e| RenderError::Screenshot(format!("encode failed: {e}")))?;
        }
        _ => {
            let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
                .ok_or_else(|| RenderError::Screenshot("RGBA buffer construction failed".into()))?;
            img.write_to(&mut buf, format.to_image_format())
                .map_err(|e| RenderError::Screenshot(format!("encode failed: {e}")))?;
        }
    }

    Ok(buf.into_inner())
}

/// Capture a render target as encoded image bytes.
///
/// Performs blocking GPU readback, then encodes to the requested format.
/// Use for tools, tests, and one-shot captures — not in game loops.
#[inline]
pub fn capture_render_target(
    target: &RenderTarget,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: ScreenshotFormat,
) -> Result<Vec<u8>> {
    let rgba = target.read_pixels(device, queue)?;
    encode_pixels(target.width, target.height, &rgba, format)
}

/// Save encoded image bytes to a file.
pub fn save_to_file(encoded: &[u8], path: &Path) -> Result<()> {
    std::fs::write(path, encoded).map_err(|e| RenderError::Screenshot(format!("save failed: {e}")))
}

// ── selah integration (feature-gated) ──────────────────────────────────────

/// Convert a [`ScreenshotFormat`] to a [`selah::ImageFormat`].
#[cfg(feature = "screenshot")]
#[must_use]
#[inline]
pub fn to_selah_format(format: ScreenshotFormat) -> selah::ImageFormat {
    match format {
        ScreenshotFormat::Png => selah::ImageFormat::Png,
        ScreenshotFormat::Jpeg => selah::ImageFormat::Jpeg,
        ScreenshotFormat::Bmp => selah::ImageFormat::Bmp,
    }
}

/// Capture a render target as a [`selah::Screenshot`].
///
/// Returns a complete `Screenshot` with encoded image data, dimensions,
/// timestamp, and capture source metadata.
#[cfg(feature = "screenshot")]
pub fn capture_screenshot(
    target: &RenderTarget,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: ScreenshotFormat,
) -> Result<selah::Screenshot> {
    let encoded = capture_render_target(target, device, queue, format)?;

    Ok(selah::Screenshot {
        id: uuid::Uuid::new_v4(),
        width: target.width,
        height: target.height,
        data: encoded,
        timestamp: chrono::Utc::now(),
        source: selah::CaptureSource::FullScreen,
        format: to_selah_format(format),
    })
}

/// Capture a render target region as a [`selah::Screenshot`].
///
/// The region is specified as pixel coordinates `(x, y, width, height)`.
#[cfg(feature = "screenshot")]
pub fn capture_screenshot_region(
    target: &RenderTarget,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    region: (u32, u32, u32, u32),
    format: ScreenshotFormat,
) -> Result<selah::Screenshot> {
    let rgba = target.read_pixels(device, queue)?;
    let (rx, ry, rw, rh) = region;

    if rx + rw > target.width || ry + rh > target.height {
        return Err(RenderError::Screenshot(format!(
            "region ({rx},{ry},{rw}x{rh}) exceeds target ({}x{})",
            target.width, target.height
        )));
    }

    // Extract the sub-rectangle from the full RGBA buffer
    let stride = (target.width * 4) as usize;
    let row_bytes = (rw * 4) as usize;
    let mut cropped = Vec::with_capacity((rw * rh * 4) as usize);
    for row in ry..(ry + rh) {
        let start = row as usize * stride + (rx * 4) as usize;
        cropped.extend_from_slice(&rgba[start..start + row_bytes]);
    }

    let encoded = encode_pixels(rw, rh, &cropped, format)?;
    let selah_rect = selah::Rect::new(rx as f32, ry as f32, rw as f32, rh as f32);

    Ok(selah::Screenshot {
        id: uuid::Uuid::new_v4(),
        width: rw,
        height: rh,
        data: encoded,
        timestamp: chrono::Utc::now(),
        source: selah::CaptureSource::Region(selah_rect),
        format: to_selah_format(format),
    })
}

/// Annotate a captured screenshot's image data.
///
/// Thin wrapper around [`selah::annotate_image`] that accepts soorat's
/// [`ScreenshotFormat`] and maps errors into [`RenderError`].
#[cfg(feature = "screenshot")]
pub fn annotate_capture(
    encoded: &[u8],
    annotations: &[selah::Annotation],
    format: ScreenshotFormat,
) -> Result<Vec<u8>> {
    selah::annotate_image(encoded, annotations, to_selah_format(format))
        .map_err(|e| RenderError::Screenshot(format!("annotation failed: {e}")))
}

/// Redact PII from a captured screenshot's image data.
///
/// Returns the redacted image bytes and a list of detected targets.
#[cfg(feature = "screenshot")]
pub fn redact_capture(
    encoded: &[u8],
    targets: Option<&[selah::RedactionTarget]>,
    format: ScreenshotFormat,
) -> Result<(Vec<u8>, Vec<selah::RedactionSuggestion>)> {
    selah::redact_image(encoded, targets, to_selah_format(format))
        .map_err(|e| RenderError::Screenshot(format!("redaction failed: {e}")))
}

/// Copy encoded image bytes to the system clipboard.
///
/// Delegates to selah's clipboard support (Wayland via `wl-copy`, X11 via `xclip`).
#[cfg(feature = "screenshot")]
pub fn copy_to_clipboard(encoded: &[u8]) -> Result<()> {
    selah::CaptureClient::copy_to_clipboard(encoded)
        .map_err(|e| RenderError::Screenshot(format!("clipboard failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_pixels_roundtrip_png() {
        // 2x2 red square
        let rgba = vec![
            255, 0, 0, 255, 255, 0, 0, 255, //
            255, 0, 0, 255, 255, 0, 0, 255,
        ];
        let encoded = encode_pixels(2, 2, &rgba, ScreenshotFormat::Png).unwrap();
        // PNG magic bytes
        assert_eq!(&encoded[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn encode_pixels_jpeg() {
        let rgba = vec![0u8; 4 * 4 * 4]; // 4x4 black
        let encoded = encode_pixels(4, 4, &rgba, ScreenshotFormat::Jpeg).unwrap();
        // JPEG magic bytes
        assert_eq!(&encoded[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn encode_pixels_size_mismatch() {
        let rgba = vec![0u8; 10]; // wrong size for any image
        let err = encode_pixels(2, 2, &rgba, ScreenshotFormat::Png);
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("size mismatch"));
    }

    #[test]
    fn screenshot_format_extension() {
        assert_eq!(ScreenshotFormat::Png.extension(), "png");
        assert_eq!(ScreenshotFormat::Jpeg.extension(), "jpg");
        assert_eq!(ScreenshotFormat::Bmp.extension(), "bmp");
    }

    #[test]
    fn screenshot_format_default_is_png() {
        assert_eq!(ScreenshotFormat::default(), ScreenshotFormat::Png);
    }

    #[test]
    fn save_to_file_roundtrip() {
        let data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic
        let path = std::env::temp_dir().join("soorat_screenshot_test.png");
        save_to_file(&data, &path).unwrap();
        let read_back = std::fs::read(&path).unwrap();
        assert_eq!(read_back, data);
        std::fs::remove_file(&path).ok();
    }

    #[cfg(feature = "screenshot")]
    #[test]
    fn to_selah_format_mapping() {
        assert_eq!(
            to_selah_format(ScreenshotFormat::Png),
            selah::ImageFormat::Png
        );
        assert_eq!(
            to_selah_format(ScreenshotFormat::Jpeg),
            selah::ImageFormat::Jpeg
        );
        assert_eq!(
            to_selah_format(ScreenshotFormat::Bmp),
            selah::ImageFormat::Bmp
        );
    }

    #[cfg(feature = "screenshot")]
    #[test]
    fn annotate_capture_empty_annotations() {
        // Create a minimal 2x2 PNG
        let rgba = vec![0u8; 2 * 2 * 4];
        let encoded = encode_pixels(2, 2, &rgba, ScreenshotFormat::Png).unwrap();
        let result = annotate_capture(&encoded, &[], ScreenshotFormat::Png).unwrap();
        assert!(!result.is_empty());
    }

    #[cfg(feature = "screenshot")]
    #[test]
    fn redact_capture_no_targets() {
        let rgba = vec![0u8; 2 * 2 * 4];
        let encoded = encode_pixels(2, 2, &rgba, ScreenshotFormat::Png).unwrap();
        let (redacted, suggestions) =
            redact_capture(&encoded, None, ScreenshotFormat::Png).unwrap();
        assert!(!redacted.is_empty());
        assert!(suggestions.is_empty()); // no text in a 2x2 black image
    }
}
