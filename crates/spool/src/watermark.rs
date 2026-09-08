use serde::{Deserialize, Serialize};

/// 4-Tier Disk Protection Watermarks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiskWatermarkLevel {
    /// Normal operation (< 70% disk usage)
    Normal,
    /// Warning / Proactive upload (70% - 85% disk usage)
    LowWater,
    /// Throttle capture / Shed P4/P3 events (85% - 92% disk usage)
    HighWater,
    /// Critical emergency stop / purge old uploads (> 92% disk usage)
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskWatermarkConfig {
    pub low_water_percent: f64,  // 70.0
    pub high_water_percent: f64, // 85.0
    pub critical_percent: f64,   // 92.0
}

impl Default for DiskWatermarkConfig {
    fn default() -> Self {
        Self {
            low_water_percent: 70.0,
            high_water_percent: 85.0,
            critical_percent: 92.0,
        }
    }
}

/// Evaluates disk watermarks from total and available bytes.
pub fn evaluate_disk_level(
    total_bytes: u64,
    available_bytes: u64,
    config: &DiskWatermarkConfig,
) -> DiskWatermarkLevel {
    if total_bytes == 0 {
        return DiskWatermarkLevel::Normal;
    }

    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let usage_pct = ((used_bytes as f64) / (total_bytes as f64)) * 100.0;

    if usage_pct >= config.critical_percent {
        DiskWatermarkLevel::Critical
    } else if usage_pct >= config.high_water_percent {
        DiskWatermarkLevel::HighWater
    } else if usage_pct >= config.low_water_percent {
        DiskWatermarkLevel::LowWater
    } else {
        DiskWatermarkLevel::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_watermark_thresholds() {
        let config = DiskWatermarkConfig::default();
        let total = 1000u64;

        // 50% used -> Normal
        assert_eq!(
            evaluate_disk_level(total, 500, &config),
            DiskWatermarkLevel::Normal
        );

        // 75% used -> LowWater
        assert_eq!(
            evaluate_disk_level(total, 250, &config),
            DiskWatermarkLevel::LowWater
        );

        // 88% used -> HighWater
        assert_eq!(
            evaluate_disk_level(total, 120, &config),
            DiskWatermarkLevel::HighWater
        );

        // 95% used -> Critical
        assert_eq!(
            evaluate_disk_level(total, 50, &config),
            DiskWatermarkLevel::Critical
        );
    }
}
