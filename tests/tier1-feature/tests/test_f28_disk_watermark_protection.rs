use spool::{evaluate_disk_level, DiskWatermarkConfig, DiskWatermarkLevel};

#[test]
fn test_f28_four_tier_disk_watermark_evaluations() {
    let config = DiskWatermarkConfig {
        low_water_percent: 70.0,
        high_water_percent: 85.0,
        critical_percent: 92.0,
    };

    let total = 1_000_000_000u64; // 1GB

    // Normal: 50% usage
    let lvl1 = evaluate_disk_level(total, 500_000_000, &config);
    assert_eq!(lvl1, DiskWatermarkLevel::Normal);

    // LowWater: 75% usage
    let lvl2 = evaluate_disk_level(total, 250_000_000, &config);
    assert_eq!(lvl2, DiskWatermarkLevel::LowWater);

    // HighWater: 88% usage
    let lvl3 = evaluate_disk_level(total, 120_000_000, &config);
    assert_eq!(lvl3, DiskWatermarkLevel::HighWater);

    // Critical: 96% usage
    let lvl4 = evaluate_disk_level(total, 40_000_000, &config);
    assert_eq!(lvl4, DiskWatermarkLevel::Critical);
}
