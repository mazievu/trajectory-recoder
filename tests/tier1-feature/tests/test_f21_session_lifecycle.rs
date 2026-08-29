#[test]
fn test_f21_session_id_format_and_parsing() {
    let machine_id = "ws-dev-01";
    let date_str = "20260829";
    let hour_str = "090000";
    let short_uuid = "a1b2c3d4";

    let session_id = format!("{}_{}_{}_{}", machine_id, date_str, hour_str, short_uuid);
    assert_eq!(session_id, "ws-dev-01_20260829_090000_a1b2c3d4");

    let parts: Vec<&str> = session_id.split('_').collect();
    assert_eq!(parts.len(), 4);
}
