use e2e_runner::verifiers::upload_verifier::UploadVerifier;

#[test]
fn test_f25_chunk_slicing_and_hashing_plan() {
    let dummy_data = vec![7u8; 100 * 1024]; // 100 KiB
    let chunk_size = 32 * 1024; // 32 KiB chunks -> 4 chunks (32, 32, 32, 4)

    let plan = UploadVerifier::plan_chunks("sess_test_100", &dummy_data, chunk_size);
    assert_eq!(plan.chunks.len(), 4);
    assert_eq!(plan.chunks[0].size_bytes, 32 * 1024);
    assert_eq!(plan.chunks[3].size_bytes, 4 * 1024);
    assert_eq!(plan.chunks[0].chunk_index, 0);
    assert_eq!(plan.chunks[3].chunk_index, 3);
}
