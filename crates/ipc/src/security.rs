use crate::error::IpcError;
use std::ptr;

#[cfg(windows)]
use windows::{
    Win32::Foundation::{HLOCAL, LocalFree},
    Win32::Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
    },
    core::PCWSTR,
};

/// SDDL granting Generic All (GA) to SYSTEM (SY), Builtin Administrators (BA), and Interactive Users (IU).
pub const DEFAULT_PIPE_SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;IU)";

pub struct PipeSecurityAttributes {
    #[cfg(windows)]
    security_descriptor: PSECURITY_DESCRIPTOR,
    #[cfg(windows)]
    attributes: SECURITY_ATTRIBUTES,
}

impl PipeSecurityAttributes {
    /// Compiles an SDDL string into a Win32 SECURITY_ATTRIBUTES struct for NamedPipe creation.
    pub fn from_sddl(sddl: &str) -> Result<Self, IpcError> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let wide_sddl: Vec<u16> = std::ffi::OsStr::new(sddl)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR(ptr::null_mut());

            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    PCWSTR(wide_sddl.as_ptr()),
                    SDDL_REVISION_1,
                    &mut p_sd,
                    None,
                )
                .map_err(|e| {
                    IpcError::SecurityDescriptorError(format!("SDDL convert failed: {e}"))
                })?;
            }

            let attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: p_sd.0,
                bInheritHandle: false.into(),
            };

            Ok(Self {
                security_descriptor: p_sd,
                attributes,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = sddl;
            Ok(Self {})
        }
    }

    #[cfg(windows)]
    pub fn as_raw_ptr(&self) -> *const SECURITY_ATTRIBUTES {
        &self.attributes as *const _
    }
}

unsafe impl Send for PipeSecurityAttributes {}
unsafe impl Sync for PipeSecurityAttributes {}

#[cfg(windows)]
impl Drop for PipeSecurityAttributes {
    fn drop(&mut self) {
        if !self.security_descriptor.0.is_null() {
            unsafe {
                let _ = LocalFree(HLOCAL(self.security_descriptor.0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sddl_security_descriptor_generation() {
        let sec_attrs = PipeSecurityAttributes::from_sddl(DEFAULT_PIPE_SDDL);
        assert!(sec_attrs.is_ok());
    }
}
