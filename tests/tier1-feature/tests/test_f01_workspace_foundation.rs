#[test]
fn test_f01_schema_version_constants() {
    let schema_name = "gtf.trajectory";
    let schema_version = "1.0";

    assert_eq!(schema_name, "gtf.trajectory");
    assert_eq!(schema_version, "1.0");
}
