//! Windows Graphics Capture, DDA WebP capture, perceptual visual diffs, and Media Foundation video pipeline.

pub mod diff;
pub mod mock;
pub mod screenshot;
pub mod video;

pub use diff::{compute_visual_diff, VisualDiffResult};
pub use mock::MockScreenCapturer;
pub use screenshot::{
    bgra_to_rgba, encode_webp, pixel_diff_ratio, CapturedFrame, ScreenCaptureBackend,
    ScreenshotPipeline, ScreenshotResult,
};
pub use video::{
    VideoFrameIndexEntry, VideoPipelineConfig, VideoRecorder, VideoSegmentMetadata,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_capture_and_diff() {
        let capturer = MockScreenCapturer::new(1920, 1080);
        let frame1 = capturer.capture_frame(1, 1_000_000_000, 100);
        let frame2 = capturer.capture_frame(1, 1_200_000_000, 100);

        let diff = compute_visual_diff(&frame1.data, &frame2.data, frame1.width, frame1.height, 0.005);
        assert!(diff.is_stabilized);
        assert_eq!(diff.changed_pixel_count, 0);
    }

    #[test]
    fn test_video_segment_indexing() {
        let capturer = MockScreenCapturer::new(1920, 1080);
        let segment = capturer.generate_mock_video_segment(1, 10_000_000_000, 60, 10);

        assert_eq!(segment.segment_id, 1);
        assert_eq!(segment.frame_count, 600); // 60s * 10fps
        assert_eq!(segment.fps, 10);
        assert_eq!(segment.frame_index.len(), 600);
        assert!(segment.frame_index[0].is_keyframe);
        assert!(segment.frame_index[20].is_keyframe); // 2s GOP = every 20 frames
        assert!(!segment.frame_index[1].is_keyframe);
    }
}
