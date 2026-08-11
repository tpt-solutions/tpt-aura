//! The WASM bootstrap: a self-describing decoder key embedded in the file header.
//!
//! Per RFC 001 §7, the first section of an AURA file is a compiled WebAssembly
//! binary containing the exact math required to decode the rest of the file. If
//! the AURA standard ever disappears, a runtime can compile this blob on the fly
//! and still read the container.
//!
//! The bytes below are a *valid, minimal* WASM module (exports an `add`
//! function) used as a placeholder decoder key. A production build would embed
//! the real AURA decoding primitives here (see `tpt-aura/bootstrap/decoder.wat`).
//!
//! Equivalent WAT:
//! ```wat
//! (module
//!   (func (export "add") (param $a i32) (param $b i32) (result i32)
//!     local.get 0
//!     local.get 1
//!     i32.add))
//! ```

use crate::codec::{Reader, Writer};

/// Magic bytes of any WASM binary: `"\0asm"`.
pub const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];
/// WASM binary format version (1).
pub const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

/// A minimal but valid WASM decoder-key module embedded in the crate.
pub const AURA_BOOTSTRAP_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, // magic "\0asm"
    0x01, 0x00, 0x00, 0x00, // version 1
    0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, // type section
    0x03, 0x02, 0x01, 0x00, // function section
    0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00, // export "add"
    0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b, // code
];

/// The embedded WASM bootstrap blob.
#[derive(Debug, Clone, PartialEq)]
pub struct Bootstrap {
    /// Raw WASM bytes.
    pub bytes: Vec<u8>,
}

impl Bootstrap {
    /// A bootstrap carrying the default embedded decoder key.
    pub fn with_default_wasm() -> Self {
        Bootstrap {
            bytes: AURA_BOOTSTRAP_WASM.to_vec(),
        }
    }

    /// Check the WASM magic + version prefix.
    pub fn validate(&self) -> bool {
        self.bytes.len() >= 8 && self.bytes[0..4] == WASM_MAGIC && self.bytes[4..8] == WASM_VERSION
    }

    /// Write the bootstrap into a section writer.
    pub fn write(&self, w: &mut Writer) {
        w.put_bytes(&self.bytes);
    }

    /// Read a bootstrap from a section reader.
    pub fn read(r: &mut Reader) -> Result<Self, crate::error::AuraError> {
        let bytes = r.bytes()?;
        Ok(Bootstrap { bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Writer;

    #[test]
    fn embedded_wasm_is_valid() {
        let b = Bootstrap::with_default_wasm();
        assert!(b.validate());
    }

    #[test]
    fn write_read_round_trip() {
        let b = Bootstrap::with_default_wasm();
        let mut w = Writer::new();
        b.write(&mut w);
        let bytes = w.into_inner();
        let mut r = Reader::new(&bytes);
        let back = Bootstrap::read(&mut r).unwrap();
        assert_eq!(back, b);
        assert_eq!(back.bytes, AURA_BOOTSTRAP_WASM);
    }

    #[test]
    fn tampered_bootstrap_fails_validation() {
        let mut b = Bootstrap::with_default_wasm();
        b.bytes[0] = 0xFF;
        assert!(!b.validate());
    }
}
