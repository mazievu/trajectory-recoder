use crate::error::CryptoError;
use std::ptr;

#[cfg(windows)]
use windows::{
    Win32::Foundation::{GetLastError, HLOCAL, LocalFree},
    Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_LOCAL_MACHINE, CRYPTPROTECT_UI_FORBIDDEN,
        CryptProtectData, CryptUnprotectData,
    },
    core::PCWSTR,
};

pub struct Dpapi;

impl Dpapi {
    /// Encrypts plaintext using Windows DPAPI with CRYPTPROTECT_LOCAL_MACHINE | CRYPTPROTECT_UI_FORBIDDEN.
    /// This allows any process (Session 0 Service, Session 1 Agent) on the machine to decrypt.
    pub fn protect_machine_secret(
        plaintext: &[u8],
        optional_entropy: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        Self::protect_internal(plaintext, optional_entropy, true)
    }

    /// Encrypts plaintext scoped to the current user with CRYPTPROTECT_UI_FORBIDDEN.
    pub fn protect_user_secret(
        plaintext: &[u8],
        optional_entropy: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        Self::protect_internal(plaintext, optional_entropy, false)
    }

    /// Decrypts DPAPI ciphertext into a vector.
    pub fn unprotect(
        ciphertext: &[u8],
        optional_entropy: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        #[cfg(windows)]
        {
            if ciphertext.is_empty() {
                return Err(CryptoError::InvalidPayloadLength { actual: 0 });
            }

            let in_blob = CRYPT_INTEGER_BLOB {
                cbData: ciphertext.len() as u32,
                pbData: ciphertext.as_ptr() as *mut u8,
            };

            let mut entropy_blob = CRYPT_INTEGER_BLOB {
                cbData: 0,
                pbData: ptr::null_mut(),
            };
            let p_entropy = if let Some(entropy) = optional_entropy {
                entropy_blob.cbData = entropy.len() as u32;
                entropy_blob.pbData = entropy.as_ptr() as *mut u8;
                &entropy_blob as *const _
            } else {
                ptr::null()
            };

            let mut out_blob = CRYPT_INTEGER_BLOB {
                cbData: 0,
                pbData: ptr::null_mut(),
            };

            let success = unsafe {
                CryptUnprotectData(
                    &in_blob,
                    None,
                    if optional_entropy.is_some() {
                        Some(p_entropy)
                    } else {
                        None
                    },
                    None,
                    None,
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut out_blob,
                )
            };

            if success.is_err() {
                let code = unsafe { GetLastError().0 };
                return Err(CryptoError::DpapiUnprotectFailed {
                    error_code: code,
                    message: format!("CryptUnprotectData failed with error code {code}"),
                });
            }

            // Copy out data and free Win32 allocated buffer
            let result = unsafe {
                let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
                let vec = slice.to_vec();
                let _ = LocalFree(HLOCAL(out_blob.pbData as *mut std::ffi::c_void));
                vec
            };

            Ok(result)
        }
        #[cfg(not(windows))]
        {
            // Software fallback for non-Windows test environments
            if ciphertext.starts_with(b"MOCK_DPAPI:") {
                Ok(ciphertext[11..].to_vec())
            } else {
                Err(CryptoError::UnsupportedPlatform)
            }
        }
    }

    #[cfg(windows)]
    fn protect_internal(
        plaintext: &[u8],
        optional_entropy: Option<&[u8]>,
        local_machine: bool,
    ) -> Result<Vec<u8>, CryptoError> {
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: plaintext.len() as u32,
            pbData: plaintext.as_ptr() as *mut u8,
        };

        let mut entropy_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        let p_entropy = if let Some(entropy) = optional_entropy {
            entropy_blob.cbData = entropy.len() as u32;
            entropy_blob.pbData = entropy.as_ptr() as *mut u8;
            &entropy_blob as *const _
        } else {
            ptr::null()
        };

        let mut flags = CRYPTPROTECT_UI_FORBIDDEN;
        if local_machine {
            flags |= CRYPTPROTECT_LOCAL_MACHINE;
        }

        let mut out_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };

        let success = unsafe {
            CryptProtectData(
                &in_blob,
                PCWSTR(ptr::null()),
                if optional_entropy.is_some() {
                    Some(p_entropy)
                } else {
                    None
                },
                None,
                None,
                flags,
                &mut out_blob,
            )
        };

        if success.is_err() {
            let code = unsafe { GetLastError().0 };
            return Err(CryptoError::DpapiProtectFailed {
                error_code: code,
                message: format!("CryptProtectData failed with error code {code}"),
            });
        }

        let result = unsafe {
            let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
            let vec = slice.to_vec();
            let _ = LocalFree(HLOCAL(out_blob.pbData as *mut std::ffi::c_void));
            vec
        };

        Ok(result)
    }

    #[cfg(not(windows))]
    fn protect_internal(
        plaintext: &[u8],
        _optional_entropy: Option<&[u8]>,
        _local_machine: bool,
    ) -> Result<Vec<u8>, CryptoError> {
        // Mock fallback for non-Windows CI
        let mut out = b"MOCK_DPAPI:".to_vec();
        out.extend_from_slice(plaintext);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpapi_protect_unprotect_roundtrip() {
        let secret = b"super-secret-machine-key-12345678";
        let protected = Dpapi::protect_machine_secret(secret, None).unwrap();
        assert_ne!(protected, secret);

        let unprotected = Dpapi::unprotect(&protected, None).unwrap();
        assert_eq!(unprotected, secret);
    }
}
