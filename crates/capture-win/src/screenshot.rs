use crate::diff::{compute_visual_diff, VisualDiffResult};
use core_types::metadata::{BoundingBox, BoundingRect};
use image::{codecs::webp::WebPEncoder, ExtendedColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Captured screen frame in memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturedFrame {
    pub monitor_id: u32,
    pub width: u32,
    pub height: u32,
    pub format: &'static str, // "webp", "png", "bgra"
    pub data: Vec<u8>,
    pub timestamp_ns: u64,
    pub bounds: BoundingRect,
}

impl CapturedFrame {
    pub fn new(
        monitor_id: u32,
        width: u32,
        height: u32,
        format: &'static str,
        data: Vec<u8>,
        timestamp_ns: u64,
        bounds: BoundingRect,
    ) -> Self {
        Self {
            monitor_id,
            width,
            height,
            format,
            data,
            timestamp_ns,
            bounds,
        }
    }
}

/// Result of a stabilized or single-shot screenshot capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub monitor_id: u32,
    pub width: u32,
    pub height: u32,
    pub format: String, // "webp"
    pub webp_data: Vec<u8>,
    pub raw_bgra: Vec<u8>,
    pub timestamp_ns: u64,
    pub diff_ratio: f32,
    pub is_stabilized: bool,
    pub bounds: BoundingRect,
    pub changed_bounding_box: Option<BoundingBox>,
}

/// Convert 32-bit BGRA buffer to 32-bit RGBA buffer.
pub fn bgra_to_rgba(bgra_data: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgra_data.len());
    for chunk in bgra_data.chunks_exact(4) {
        rgba.push(chunk[2]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[0]); // B
        rgba.push(chunk[3]); // A
    }
    rgba
}

/// Encode 32-bit BGRA image data into WebP format bytes.
pub fn encode_webp(bgra_data: &[u8], width: u32, height: u32, _quality: u8) -> Option<Vec<u8>> {
    let total_pixels = (width * height) as usize;
    if width == 0 || height == 0 || bgra_data.len() < total_pixels * 4 {
        return None;
    }

    let rgba_data = bgra_to_rgba(&bgra_data[..total_pixels * 4]);
    let mut output = Vec::new();
    let encoder = WebPEncoder::new_lossless(&mut output);
    match encoder.write_image(&rgba_data, width, height, ExtendedColorType::Rgba8) {
        Ok(_) => Some(output),
        Err(e) => {
            tracing::error!("WebP encoding failed: {}", e);
            None
        }
    }
}

/// Calculate perceptual/pixel difference ratio between two 32-bit BGRA/RGBA image buffers.
pub fn pixel_diff_ratio(a: &[u8], b: &[u8]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return if a.is_empty() && b.is_empty() { 0.0 } else { 1.0 };
    }

    let pixel_count = a.len() / 4;
    if pixel_count == 0 {
        return 0.0;
    }

    let mut changed = 0usize;
    for (px_a, px_b) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let db = (px_a[0] as i32 - px_b[0] as i32).abs();
        let dg = (px_a[1] as i32 - px_b[1] as i32).abs();
        let dr = (px_a[2] as i32 - px_b[2] as i32).abs();
        if dr > 10 || dg > 10 || db > 10 {
            changed += 1;
        }
    }

    changed as f32 / pixel_count as f32
}

/// Abstract backend interface for capturing screen frames.
pub trait ScreenCaptureBackend: Send + Sync {
    fn capture(&self, monitor_id: u32) -> Option<CapturedFrame>;
}

#[cfg(windows)]
pub mod native {
    use super::CapturedFrame;
    use core_types::metadata::BoundingRect;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HBITMAP, HDC, SRCCOPY,
    };

    pub fn capture_screen_gdi(
        monitor_id: u32,
        left: i32,
        top: i32,
        width: u32,
        height: u32,
        timestamp_ns: u64,
    ) -> Option<CapturedFrame> {
        unsafe {
            let hdc_screen: HDC = GetDC(HWND(0 as _));
            if hdc_screen.is_invalid() {
                return None;
            }

            let hdc_mem: HDC = CreateCompatibleDC(hdc_screen);
            let hbm: HBITMAP = CreateCompatibleBitmap(hdc_screen, width as i32, height as i32);
            let old_bm = SelectObject(hdc_mem, hbm);

            let _ = BitBlt(
                hdc_mem,
                0,
                0,
                width as i32,
                height as i32,
                hdc_screen,
                left,
                top,
                SRCCOPY,
            );

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32), // Top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: width * height * 4,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut buffer = vec![0u8; (width * height * 4) as usize];
            let lines = GetDIBits(
                hdc_mem,
                hbm,
                0,
                height,
                Some(buffer.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            SelectObject(hdc_mem, old_bm);
            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(HWND(0 as _), hdc_screen);

            if lines > 0 {
                Some(CapturedFrame {
                    monitor_id,
                    width,
                    height,
                    format: "bgra",
                    data: buffer,
                    timestamp_ns,
                    bounds: BoundingRect::new(left, top, left + width as i32, top + height as i32),
                })
            } else {
                None
            }
        }
    }
}

/// Default GDI backend for Windows.
#[cfg(windows)]
pub struct NativeGdiBackend {
    pub width: u32,
    pub height: u32,
}

#[cfg(windows)]
impl Default for NativeGdiBackend {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

#[cfg(windows)]
impl ScreenCaptureBackend for NativeGdiBackend {
    fn capture(&self, monitor_id: u32) -> Option<CapturedFrame> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        native::capture_screen_gdi(monitor_id, 0, 0, self.width, self.height, ts)
    }
}

/// Pipeline manager for capturing before-action and stabilized after-action screenshots in WebP format.
pub struct ScreenshotPipeline {
    pub quality: u8,
    pub default_delays_ms: Vec<u64>,
    pub default_diff_threshold: f32,
    backend: Option<Arc<dyn ScreenCaptureBackend + Send + Sync>>,
}

impl Default for ScreenshotPipeline {
    fn default() -> Self {
        Self::new(80)
    }
}

impl ScreenshotPipeline {
    pub fn new(quality: u8) -> Self {
        #[cfg(windows)]
        let backend: Option<Arc<dyn ScreenCaptureBackend + Send + Sync>> =
            Some(Arc::new(NativeGdiBackend::default()));
        #[cfg(not(windows))]
        let backend: Option<Arc<dyn ScreenCaptureBackend + Send + Sync>> = None;

        Self {
            quality,
            default_delays_ms: vec![200, 500, 1000],
            default_diff_threshold: 0.005,
            backend,
        }
    }

    pub fn with_backend(
        quality: u8,
        backend: Arc<dyn ScreenCaptureBackend + Send + Sync>,
    ) -> Self {
        Self {
            quality,
            default_delays_ms: vec![200, 500, 1000],
            default_diff_threshold: 0.005,
            backend: Some(backend),
        }
    }

    /// Capture a single before-action screenshot and encode to WebP.
    pub fn capture_before(&self, monitor_id: u32) -> Option<ScreenshotResult> {
        let frame = self.backend.as_ref()?.capture(monitor_id)?;
        let webp_data = encode_webp(&frame.data, frame.width, frame.height, self.quality)?;

        Some(ScreenshotResult {
            monitor_id: frame.monitor_id,
            width: frame.width,
            height: frame.height,
            format: "webp".to_string(),
            webp_data,
            raw_bgra: frame.data,
            timestamp_ns: frame.timestamp_ns,
            diff_ratio: 0.0,
            is_stabilized: true,
            bounds: frame.bounds,
            changed_bounding_box: None,
        })
    }

    /// Capture an after-action screenshot, sampling after incremental delays until visual stabilization occurs.
    pub fn capture_after_stable(
        &self,
        monitor_id: u32,
        base_image: &[u8],
        delays_ms: &[u64],
        diff_threshold: f32,
    ) -> Option<ScreenshotResult> {
        let backend = self.backend.as_ref()?;
        let delays = if delays_ms.is_empty() {
            &self.default_delays_ms[..]
        } else {
            delays_ms
        };

        let mut last_capture: Option<(CapturedFrame, VisualDiffResult)> = None;

        for delay in delays {
            if *delay > 0 {
                std::thread::sleep(Duration::from_millis(*delay));
            }

            if let Some(frame) = backend.capture(monitor_id) {
                let diff = compute_visual_diff(
                    base_image,
                    &frame.data,
                    frame.width,
                    frame.height,
                    diff_threshold,
                );

                if diff.is_stabilized {
                    let webp_data = encode_webp(&frame.data, frame.width, frame.height, self.quality)?;
                    return Some(ScreenshotResult {
                        monitor_id: frame.monitor_id,
                        width: frame.width,
                        height: frame.height,
                        format: "webp".to_string(),
                        webp_data,
                        raw_bgra: frame.data,
                        timestamp_ns: frame.timestamp_ns,
                        diff_ratio: diff.change_ratio,
                        is_stabilized: true,
                        bounds: frame.bounds,
                        changed_bounding_box: diff.changed_bounding_box,
                    });
                }

                last_capture = Some((frame, diff));
            }
        }

        // If not stabilized within budget, return the last frame marked as unstabilized
        if let Some((frame, diff)) = last_capture {
            let webp_data = encode_webp(&frame.data, frame.width, frame.height, self.quality)?;
            Some(ScreenshotResult {
                monitor_id: frame.monitor_id,
                width: frame.width,
                height: frame.height,
                format: "webp".to_string(),
                webp_data,
                raw_bgra: frame.data,
                timestamp_ns: frame.timestamp_ns,
                diff_ratio: diff.change_ratio,
                is_stabilized: false,
                bounds: frame.bounds,
                changed_bounding_box: diff.changed_bounding_box,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBackend {
        frames: std::sync::Mutex<Vec<CapturedFrame>>,
    }

    impl ScreenCaptureBackend for MockBackend {
        fn capture(&self, _monitor_id: u32) -> Option<CapturedFrame> {
            let mut guard = self.frames.lock().unwrap();
            if guard.is_empty() {
                None
            } else {
                Some(guard.remove(0))
            }
        }
    }

    #[test]
    fn test_encode_webp_valid() {
        let width = 64u32;
        let height = 64u32;
        // 64x64 solid blue (BGRA = 255, 0, 0, 255)
        let mut bgra = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            bgra.extend_from_slice(&[255, 0, 0, 255]);
        }

        let webp_bytes = encode_webp(&bgra, width, height, 80).expect("WebP encode should succeed");
        assert!(!webp_bytes.is_empty());
        // WebP header contains "RIFF" and "WEBP"
        assert!(webp_bytes.starts_with(b"RIFF"));
        assert_eq!(&webp_bytes[8..12], b"WEBP");
    }

    #[test]
    fn test_pixel_diff_ratio() {
        let size = 100 * 100 * 4;
        let a = vec![0u8; size];
        let mut b = vec![0u8; size];

        assert_eq!(pixel_diff_ratio(&a, &b), 0.0);

        // Change 1000 out of 10000 pixels (10%)
        for i in 0..1000 {
            b[i * 4] = 200;
        }
        let ratio = pixel_diff_ratio(&a, &b);
        assert!((ratio - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_screenshot_pipeline_before_and_after_stable() {
        let width = 32u32;
        let height = 32u32;
        let base_bgra = vec![100u8; (width * height * 4) as usize];
        let stable_bgra = base_bgra.clone(); // identical

        let frame1 = CapturedFrame::new(
            1,
            width,
            height,
            "bgra",
            base_bgra.clone(),
            1_000_000,
            BoundingRect::new(0, 0, width as i32, height as i32),
        );
        let frame2 = CapturedFrame::new(
            1,
            width,
            height,
            "bgra",
            stable_bgra,
            2_000_000,
            BoundingRect::new(0, 0, width as i32, height as i32),
        );

        let backend = Arc::new(MockBackend {
            frames: std::sync::Mutex::new(vec![frame1, frame2]),
        });

        let pipeline = ScreenshotPipeline::with_backend(80, backend);

        let before = pipeline.capture_before(1).expect("capture_before succeeded");
        assert_eq!(before.monitor_id, 1);
        assert_eq!(before.width, 32);
        assert!(before.webp_data.starts_with(b"RIFF"));

        let after = pipeline
            .capture_after_stable(1, &base_bgra, &[0, 0], 0.005)
            .expect("capture_after_stable succeeded");
        assert!(after.is_stabilized);
        assert_eq!(after.diff_ratio, 0.0);
    }
}
