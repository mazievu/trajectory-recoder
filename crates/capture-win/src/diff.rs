use core_types::metadata::BoundingBox;
use serde::{Deserialize, Serialize};

/// Visual diff result between two screen captures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualDiffResult {
    pub changed_pixel_count: usize,
    pub total_pixel_count: usize,
    pub change_ratio: f32,   // 0.0 .. 1.0 (e.g. 0.005 = 0.5%)
    pub is_stabilized: bool, // True if change_ratio < threshold
    pub changed_bounding_box: Option<BoundingBox>,
}

/// Compute perceptual difference between two 32-bit BGRA/RGBA image buffers.
pub fn compute_visual_diff(
    before_buf: &[u8],
    after_buf: &[u8],
    width: u32,
    height: u32,
    stabilization_threshold: f32, // e.g. 0.005 (0.5%)
) -> VisualDiffResult {
    let total_pixels = (width * height) as usize;
    if before_buf.len() < total_pixels * 4
        || after_buf.len() < total_pixels * 4
        || total_pixels == 0
    {
        return VisualDiffResult {
            changed_pixel_count: 0,
            total_pixel_count: total_pixels,
            change_ratio: 0.0,
            is_stabilized: true,
            changed_bounding_box: None,
        };
    }

    let mut changed_pixels = 0usize;
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0u32;
    let mut max_y = 0u32;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let b1 = before_buf[idx];
            let g1 = before_buf[idx + 1];
            let r1 = before_buf[idx + 2];

            let b2 = after_buf[idx];
            let g2 = after_buf[idx + 1];
            let r2 = after_buf[idx + 2];

            // Calculate color distance
            let dr = (r1 as i32 - r2 as i32).abs();
            let dg = (g1 as i32 - g2 as i32).abs();
            let db = (b1 as i32 - b2 as i32).abs();

            if dr > 10 || dg > 10 || db > 10 {
                changed_pixels += 1;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    let change_ratio = (changed_pixels as f32) / (total_pixels as f32);
    let is_stabilized = change_ratio <= stabilization_threshold;

    let changed_bounding_box = if changed_pixels > 0 {
        Some(BoundingBox::new(
            min_x as i32,
            min_y as i32,
            max_x.saturating_sub(min_x) + 1,
            max_y.saturating_sub(min_y) + 1,
        ))
    } else {
        None
    };

    VisualDiffResult {
        changed_pixel_count: changed_pixels,
        total_pixel_count: total_pixels,
        change_ratio,
        is_stabilized,
        changed_bounding_box,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_diff() {
        let buf = vec![255u8; 100 * 100 * 4];
        let diff = compute_visual_diff(&buf, &buf, 100, 100, 0.005);
        assert_eq!(diff.changed_pixel_count, 0);
        assert_eq!(diff.change_ratio, 0.0);
        assert!(diff.is_stabilized);
        assert!(diff.changed_bounding_box.is_none());
    }

    #[test]
    fn test_region_diff() {
        let buf1 = vec![0u8; 100 * 100 * 4];
        let mut buf2 = buf1.clone();

        // Change a 10x10 block at (20, 30)
        for y in 30..40 {
            for x in 20..30 {
                let idx = ((y * 100 + x) * 4) as usize;
                buf2[idx] = 255; // B
                buf2[idx + 1] = 255; // G
                buf2[idx + 2] = 255; // R
            }
        }

        let diff = compute_visual_diff(&buf1, &buf2, 100, 100, 0.005);
        assert_eq!(diff.changed_pixel_count, 100);
        assert_eq!(diff.total_pixel_count, 10000);
        assert!((diff.change_ratio - 0.01).abs() < 0.0001);
        assert!(!diff.is_stabilized); // 1.0% > 0.5% threshold

        let bbox = diff.changed_bounding_box.expect("Bounding box found");
        assert_eq!(bbox.x, 20);
        assert_eq!(bbox.y, 30);
        assert_eq!(bbox.width, 10);
        assert_eq!(bbox.height, 10);
    }
}
