use capture_win::diff::compute_visual_diff;
use capture_win::screenshot::{
    CapturedFrame, ScreenCaptureBackend, ScreenshotPipeline, bgra_to_rgba, encode_webp,
    pixel_diff_ratio,
};
use core_types::metadata::BoundingRect;
use std::sync::{Arc, Mutex};

struct MultiStageMockBackend {
    frames: Mutex<Vec<CapturedFrame>>,
}

impl ScreenCaptureBackend for MultiStageMockBackend {
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
fn test_encode_webp_edge_cases() {
    // Zero dimensions
    assert!(encode_webp(&[], 0, 0, 80).is_none());
    assert!(encode_webp(&[255; 400], 0, 10, 80).is_none());
    assert!(encode_webp(&[255; 400], 10, 0, 80).is_none());

    // Truncated buffer
    let truncated_buf = vec![0u8; 10 * 10 * 4 - 1];
    assert!(encode_webp(&truncated_buf, 10, 10, 80).is_none());

    // 1x1 Pixel
    let one_px = vec![128, 64, 32, 255]; // BGRA
    let webp_1px = encode_webp(&one_px, 1, 1, 80).expect("1x1 WebP encode");
    assert!(webp_1px.starts_with(b"RIFF"));
    assert_eq!(&webp_1px[8..12], b"WEBP");

    // 4K Frame (3840 x 2160)
    let w_4k = 384u32; // Scaled for fast unit test
    let h_4k = 216u32;
    let mut bgra_4k = vec![0u8; (w_4k * h_4k * 4) as usize];
    for chunk in bgra_4k.chunks_exact_mut(4) {
        chunk[0] = 200; // B
        chunk[1] = 100; // G
        chunk[2] = 50; // R
        chunk[3] = 255; // A
    }
    let webp_4k = encode_webp(&bgra_4k, w_4k, h_4k, 80).expect("4K WebP encode");
    assert!(webp_4k.starts_with(b"RIFF"));
    assert_eq!(&webp_4k[8..12], b"WEBP");
}

#[test]
fn test_bgra_to_rgba_channel_swapping() {
    let bgra = vec![
        10, 20, 30, 255, // Pixel 1: B=10, G=20, R=30, A=255
        40, 50, 60, 128, // Pixel 2: B=40, G=50, R=60, A=128
    ];
    let rgba = bgra_to_rgba(&bgra);
    assert_eq!(rgba.len(), 8);
    assert_eq!(&rgba[0..4], &[30, 20, 10, 255]); // R=30, G=20, B=10, A=255
    assert_eq!(&rgba[4..8], &[60, 50, 40, 128]); // R=60, G=50, B=40, A=128
}

#[test]
fn test_pixel_diff_ratio_thresholds_and_boundaries() {
    // Empty buffers
    assert_eq!(pixel_diff_ratio(&[], &[]), 0.0);
    assert_eq!(pixel_diff_ratio(&[1, 2, 3, 4], &[]), 1.0);
    assert_eq!(pixel_diff_ratio(&[1, 2, 3, 4], &[1, 2, 3]), 1.0);

    // Exact same
    let base = vec![100u8; 400]; // 100 pixels
    assert_eq!(pixel_diff_ratio(&base, &base), 0.0);

    // Delta <= 10 should NOT count as changed
    let mut small_diff = base.clone();
    for i in 0..100 {
        small_diff[i * 4] = 110; // +10
        small_diff[i * 4 + 1] = 90; // -10
        small_diff[i * 4 + 2] = 105; // +5
    }
    assert_eq!(pixel_diff_ratio(&base, &small_diff), 0.0);

    // Delta == 11 on just ONE channel should count as changed
    let mut single_diff = base.clone();
    single_diff[0] = 111; // +11
    assert_eq!(pixel_diff_ratio(&base, &single_diff), 0.01); // 1 out of 100 = 0.01

    // Delta on Green channel == 11
    let mut green_diff = base.clone();
    green_diff[5] = 111; // +11 on G of pixel 1
    assert_eq!(pixel_diff_ratio(&base, &green_diff), 0.01);

    // Delta on Red channel == 11
    let mut red_diff = base.clone();
    red_diff[10] = 111; // +11 on R of pixel 2
    assert_eq!(pixel_diff_ratio(&base, &red_diff), 0.01);
}

#[test]
fn test_compute_visual_diff_bounding_box_adversarial() {
    let width = 50u32;
    let height = 50u32;
    let total_bytes = (width * height * 4) as usize;
    let base = vec![0u8; total_bytes];

    // Case 1: Top-Left pixel only (0, 0)
    let mut tl = base.clone();
    tl[0] = 255;
    let diff_tl = compute_visual_diff(&base, &tl, width, height, 0.005);
    assert_eq!(diff_tl.changed_pixel_count, 1);
    let bbox_tl = diff_tl.changed_bounding_box.expect("BBox top-left");
    assert_eq!(bbox_tl.x, 0);
    assert_eq!(bbox_tl.y, 0);
    assert_eq!(bbox_tl.width, 1);
    assert_eq!(bbox_tl.height, 1);

    // Case 2: Bottom-Right pixel only (49, 49)
    let mut br = base.clone();
    let br_idx = ((49 * width + 49) * 4) as usize;
    br[br_idx] = 255;
    let diff_br = compute_visual_diff(&base, &br, width, height, 0.005);
    assert_eq!(diff_br.changed_pixel_count, 1);
    let bbox_br = diff_br.changed_bounding_box.expect("BBox bottom-right");
    assert_eq!(bbox_br.x, 49);
    assert_eq!(bbox_br.y, 49);
    assert_eq!(bbox_br.width, 1);
    assert_eq!(bbox_br.height, 1);

    // Case 3: Four corners changed: (0,0), (49,0), (0,49), (49,49) -> BBox must span 50x50
    let mut corners = base.clone();
    corners[0] = 255; // (0,0)
    corners[((0 * width + 49) * 4) as usize] = 255; // (49,0)
    corners[((49 * width + 0) * 4) as usize] = 255; // (0,49)
    corners[((49 * width + 49) * 4) as usize] = 255; // (49,49)

    let diff_corners = compute_visual_diff(&base, &corners, width, height, 0.005);
    assert_eq!(diff_corners.changed_pixel_count, 4);
    let bbox_c = diff_corners.changed_bounding_box.expect("BBox 4 corners");
    assert_eq!(bbox_c.x, 0);
    assert_eq!(bbox_c.y, 0);
    assert_eq!(bbox_c.width, 50);
    assert_eq!(bbox_c.height, 50);
}

#[test]
fn test_screenshot_pipeline_stabilization_delayed_convergence() {
    let width = 20u32;
    let height = 20u32;
    let total_bytes = (width * height * 4) as usize;

    let base_bgra = vec![50u8; total_bytes];

    // Frame 0: heavily shifting (50% changed)
    let mut shifting_bgra = base_bgra.clone();
    for i in 0..200 {
        shifting_bgra[i * 4] = 200;
    }

    // Frame 1: settled (0% changed)
    let settled_bgra = base_bgra.clone();

    let f0 = CapturedFrame::new(
        1,
        width,
        height,
        "bgra",
        shifting_bgra,
        1_000_000,
        BoundingRect::new(0, 0, width as i32, height as i32),
    );
    let f1 = CapturedFrame::new(
        1,
        width,
        height,
        "bgra",
        settled_bgra,
        2_000_000,
        BoundingRect::new(0, 0, width as i32, height as i32),
    );

    let backend = Arc::new(MultiStageMockBackend {
        frames: Mutex::new(vec![f0, f1]),
    });

    let pipeline = ScreenshotPipeline::with_backend(80, backend);

    // Pass delays: [0, 0] so it samples f0 on attempt 1, f1 on attempt 2
    let result = pipeline
        .capture_after_stable(1, &base_bgra, &[0, 0], 0.005)
        .expect("capture_after_stable");

    assert!(result.is_stabilized);
    assert_eq!(result.diff_ratio, 0.0);
    assert_eq!(result.format, "webp");
    assert!(result.webp_data.starts_with(b"RIFF"));
}

#[test]
fn test_screenshot_pipeline_unstable_timeout_fallback() {
    let width = 10u32;
    let height = 10u32;
    let total_bytes = (width * height * 4) as usize;

    let base_bgra = vec![10u8; total_bytes];

    // Both frames remain violently changing (100% changed)
    let unstable_bgra1 = vec![255u8; total_bytes];
    let unstable_bgra2 = vec![200u8; total_bytes];

    let f0 = CapturedFrame::new(
        1,
        width,
        height,
        "bgra",
        unstable_bgra1,
        1_000_000,
        BoundingRect::new(0, 0, width as i32, height as i32),
    );
    let f1 = CapturedFrame::new(
        1,
        width,
        height,
        "bgra",
        unstable_bgra2,
        2_000_000,
        BoundingRect::new(0, 0, width as i32, height as i32),
    );

    let backend = Arc::new(MultiStageMockBackend {
        frames: Mutex::new(vec![f0, f1]),
    });

    let pipeline = ScreenshotPipeline::with_backend(80, backend);

    let result = pipeline
        .capture_after_stable(1, &base_bgra, &[0, 0], 0.005)
        .expect("capture_after_stable timeout fallback");

    // Must be marked as NOT stabilized
    assert!(!result.is_stabilized);
    assert_eq!(result.diff_ratio, 1.0);
    assert_eq!(result.format, "webp");
    assert!(result.webp_data.starts_with(b"RIFF"));
}
