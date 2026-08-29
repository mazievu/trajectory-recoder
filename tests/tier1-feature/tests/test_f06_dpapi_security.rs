use sha2::{Digest, Sha256};

#[test]
fn test_f06_machine_token_derivation_hash() {
    let secret = b"dpapi_protected_machine_secret_token_entropy";
    let mut hasher = Sha256::new();
    hasher.update(secret);
    let hash = format!("{:x}", hasher.finalize());

    assert_eq!(hash.len(), 64);
}
