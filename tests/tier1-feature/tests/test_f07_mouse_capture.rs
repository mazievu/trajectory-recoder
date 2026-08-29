#[test]
fn test_f07_coordinate_normalization() {
    let physical_x: f64 = 960.0;
    let physical_y: f64 = 540.0;
    let screen_w: f64 = 1920.0;
    let screen_h: f64 = 1080.0;

    let norm_x: f64 = physical_x / screen_w;
    let norm_y: f64 = physical_y / screen_h;

    assert!((norm_x - 0.5f64).abs() < 1e-5);
    assert!((norm_y - 0.5f64).abs() < 1e-5);
}
