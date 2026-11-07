//! Reads live power sensors from the SMC (AppleSMC IOKit service). No root needed.

use anyhow::{Result, anyhow, bail};
use std::ffi::c_void;

type KernReturn = i32;
type MachPort = u32;

/// Selector for `IOConnectCallStructMethod` that talks to the SMC.
const SMC_SELECTOR: u32 = 2;
const CMD_READ_KEY: u8 = 5;
const CMD_READ_KEY_INFO: u8 = 9;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Version {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct PLimitData {
    version: u16,
    length: u16,
    cpu: u32,
    gpu: u32,
    mem: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct KeyInfo {
    data_size: u32,
    data_type: u32,
    attributes: u8,
}

/// Mirrors the C `SMCKeyData_t` struct the AppleSMC user client expects.
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct KeyData {
    key: u32,
    vers: Version,
    p_limit: PLimitData,
    key_info: KeyInfo,
    result: u8,
    status: u8,
    data8: u8,
    data32: u32,
    bytes: [u8; 32],
}

// The AppleSMC user client reads this exact layout, so the size must match.
const _: () = assert!(std::mem::size_of::<KeyData>() == 80);

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    static mach_task_self_: MachPort;
    fn IOServiceMatching(name: *const std::ffi::c_char) -> *mut c_void;
    /// Consumes `matching`. Returns 0 when nothing matches.
    fn IOServiceGetMatchingService(main_port: MachPort, matching: *mut c_void) -> u32;
    fn IOServiceOpen(service: u32, owning_task: MachPort, kind: u32, connect: *mut u32) -> KernReturn;
    fn IOServiceClose(connect: u32) -> KernReturn;
    fn IOObjectRelease(object: u32) -> KernReturn;
    fn IOConnectCallStructMethod(
        connect: u32,
        selector: u32,
        input: *const c_void,
        input_size: usize,
        output: *mut c_void,
        output_size: *mut usize,
    ) -> KernReturn;
}

/// An open connection to the AppleSMC service. Closed on drop.
pub struct Smc {
    conn: u32,
}

impl Smc {
    pub fn open() -> Result<Smc> {
        // SAFETY: plain IOKit calls. `matching` is consumed by the Get call,
        // the service handle is released before returning, and `conn` is a
        // valid out-pointer.
        unsafe {
            let matching = IOServiceMatching(c"AppleSMC".as_ptr());
            if matching.is_null() {
                bail!("IOServiceMatching(AppleSMC) failed");
            }
            let service = IOServiceGetMatchingService(0, matching);
            if service == 0 {
                bail!("AppleSMC service not found");
            }
            let mut conn = 0;
            let kr = IOServiceOpen(service, mach_task_self_, 0, &mut conn);
            IOObjectRelease(service);
            if kr != 0 {
                bail!("IOServiceOpen(AppleSMC) failed: {kr:#x}");
            }
            Ok(Smc { conn })
        }
    }

    /// Reads a 4-char key whose type is "flt " (f32, little-endian).
    pub fn read_f32(&self, key: &str) -> Result<f32> {
        let code = key_code(key)?;
        let info = self.call(KeyData { key: code, data8: CMD_READ_KEY_INFO, ..Default::default() })?;
        let size = info.key_info.data_size;
        let out = self.call(KeyData {
            key: code,
            key_info: KeyInfo { data_size: size, ..Default::default() },
            data8: CMD_READ_KEY,
            ..Default::default()
        })?;
        let n = (size as usize).min(out.bytes.len());
        decode_f32(info.key_info.data_type, &out.bytes[..n]).map_err(|e| anyhow!("{key}: {e}"))
    }

    fn call(&self, input: KeyData) -> Result<KeyData> {
        let mut out = KeyData::default();
        let mut out_size = std::mem::size_of::<KeyData>();
        // SAFETY: input/output are repr(C) structs of exactly `size_of::<KeyData>()`
        // bytes, and `out_size` holds that size for the call.
        let kr = unsafe {
            IOConnectCallStructMethod(
                self.conn,
                SMC_SELECTOR,
                &input as *const KeyData as *const c_void,
                std::mem::size_of::<KeyData>(),
                &mut out as *mut KeyData as *mut c_void,
                &mut out_size,
            )
        };
        if kr != 0 {
            bail!("SMC call failed: {kr:#x}");
        }
        if out.result != 0 {
            bail!("SMC result code {}", out.result);
        }
        Ok(out)
    }
}

impl Drop for Smc {
    fn drop(&mut self) {
        // SAFETY: `conn` came from a successful IOServiceOpen and is closed once.
        unsafe {
            IOServiceClose(self.conn);
        }
    }
}

/// Packs a 4-char ASCII key into the big-endian u32 the SMC expects.
fn key_code(key: &str) -> Result<u32> {
    let b: [u8; 4] = key.as_bytes().try_into().map_err(|_| anyhow!("SMC key must be 4 bytes: {key}"))?;
    Ok(u32::from_be_bytes(b))
}

/// Decodes an SMC "flt " value. Any other type is an error.
fn decode_f32(data_type: u32, bytes: &[u8]) -> Result<f32> {
    if data_type != u32::from_be_bytes(*b"flt ") {
        bail!("unsupported SMC type {:?}", data_type.to_be_bytes());
    }
    let b = <[u8; 4]>::try_from(bytes).map_err(|_| anyhow!("flt needs 4 bytes, got {}", bytes.len()))?;
    Ok(f32::from_le_bytes(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_flt_little_endian() {
        let t = u32::from_be_bytes(*b"flt ");
        assert_eq!(decode_f32(t, &58.5f32.to_le_bytes()).unwrap(), 58.5);
        assert!(decode_f32(u32::from_be_bytes(*b"ui32"), &[0; 4]).is_err());
        assert!(decode_f32(t, &[0; 3]).is_err());
    }
}
