#[test]
fn test_f05_named_pipe_framing_length_prefix() {
    let payload = b"Hello Trajectory IPC Bridge";
    let len = payload.len() as u32;
    let mut framed = Vec::new();
    framed.extend_from_slice(&len.to_be_bytes());
    framed.extend_from_slice(payload);

    assert_eq!(framed.len(), 4 + payload.len());
    let read_len = u32::from_be_bytes(framed[0..4].try_into().unwrap()) as usize;
    assert_eq!(read_len, payload.len());
    assert_eq!(&framed[4..4 + read_len], payload);
}
