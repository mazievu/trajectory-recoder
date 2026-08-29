use e2e_runner::verifiers::crypto_verifier::CryptoVerifier;

#[test]
fn test_f24_xchacha20_aead_authenticated_crypto() {
    let key = CryptoVerifier::generate_key();
    let nonce = CryptoVerifier::generate_nonce();
    let plaintext = b"Confidential Trajectory Archive Tar Stream";

    let ciphertext = CryptoVerifier::encrypt_payload(&key, &nonce, plaintext).unwrap();
    let decrypted = CryptoVerifier::decrypt_payload(&key, &nonce, &ciphertext).unwrap();

    assert_eq!(decrypted, plaintext);
}
