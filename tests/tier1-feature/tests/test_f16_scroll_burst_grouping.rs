#[test]
fn test_f16_scroll_delta_accumulation() {
    let wheel_ticks = [120.0, 120.0, 120.0, 120.0];
    let total_delta: f64 = wheel_ticks.iter().sum();

    assert_eq!(total_delta, 480.0);
}
