use std::path::Path;

#[test]
fn test_f13_file_event_filtering_user_directories() {
    let file_path = Path::new("C:/Users/alice/Documents/Reports/Financial_Q3.xlsx");
    let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");

    assert_eq!(ext, "xlsx");
    assert!(file_path.starts_with("C:/Users/alice/Documents"));
}
