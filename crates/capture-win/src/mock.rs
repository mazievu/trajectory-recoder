use crate::screenshot::CapturedFrame;
use crate::video::{VideoFrameIndexEntry, VideoSegmentMetadata};
use core_types::metadata::BoundingRect;

/// In-memory mock screen capturer for headless tests and CI.
#[derive(Clone, Default)]
pub struct MockScreenCapturer {
    default_width: u32,
    default_height: u32,
}

impl MockScreenCapturer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            default_width: width,
            default_height: height,
        }
    }

    pub fn capture_frame(&self, monitor_id: u32, timestamp_ns: u64, color: u8) -> CapturedFrame {
        let size = (self.default_width * self.default_height * 4) as usize;
        let data = vec![color; size];
        CapturedFrame {
            monitor_id,
            width: self.default_width,
            height: self.default_height,
            format: "webp",
            data,
            timestamp_ns,
            bounds: BoundingRect::new(
                0,
                0,
                self.default_width as i32,
                self.default_height as i32,
            ),
        }
    }

    pub fn generate_mock_video_segment(
        &self,
        segment_id: u32,
        start_ns: u64,
        duration_secs: u32,
        fps: u32,
    ) -> VideoSegmentMetadata {
        let total_frames = duration_secs * fps;
        let mut index = Vec::new();
        let frame_interval_ns = 1_000_000_000u64 / (fps as u64);

        for i in 0..total_frames {
            let pts = start_ns + (i as u64) * frame_interval_ns;
            let is_keyframe = i % (fps * 2) == 0; // Keyframe every 2 seconds
            index.push(VideoFrameIndexEntry {
                frame_number: i,
                pts_ns: pts,
                is_keyframe,
                byte_offset: (i as u64) * 15_000,
                byte_size: 15_000,
            });
        }

        VideoSegmentMetadata {
            segment_id,
            file_name: format!("video_{:04}.mp4", segment_id),
            start_monotonic_ns: start_ns,
            end_monotonic_ns: start_ns + (duration_secs as u64) * 1_000_000_000,
            frame_count: total_frames,
            fps,
            bitrate_kbps: 1500,
            gop_size_secs: 2.0,
            width: self.default_width,
            height: self.default_height,
            frame_index: index,
        }
    }
}
