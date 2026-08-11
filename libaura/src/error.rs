//! Error types for the AURA core library.

use thiserror::Error;

/// All fallible operations in `libaura` surface an [`AuraError`].
#[derive(Error, Debug)]
pub enum AuraError {
    /// Wraps a `std::io::Error` raised while reading or writing bytes.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The file does not begin with the `AURA` magic bytes.
    #[error("bad magic: expected {expected:?}, found {found:?}")]
    BadMagic { expected: [u8; 4], found: [u8; 4] },

    /// The file uses a major version this implementation cannot read.
    #[error("unsupported version {0}.{1}")]
    UnsupportedVersion(u16, u16),

    /// A record section referenced an unknown type tag.
    #[error("unknown record type 0x{0:02x}")]
    UnknownRecord(u8),

    /// A required section was absent from the file header.
    #[error("missing section 0x{0:02x}")]
    MissingSection(u8),

    /// The trailing CRC-32 did not match the computed checksum.
    #[error("checksum mismatch: stored {stored:#010x}, computed {computed:#010x}")]
    ChecksumMismatch { stored: u32, computed: u32 },

    /// The SHA-3 content hash did not match the stored footer hash.
    #[error("content hash mismatch")]
    HashMismatch,

    /// The cryptographic trust seal was broken (chain verification failed).
    #[error("trust seal broken: {0}")]
    TrustSealBroken(String),

    /// An ed25519 signature failed to verify.
    #[error("signature verification failed")]
    SignatureInvalid,

    /// A key or signature buffer had the wrong length.
    #[error("invalid key/signature length")]
    InvalidLength,

    /// A semantic graph contained a cycle where a DAG was required.
    #[error("semantic graph contains a cycle")]
    CycleDetected,

    /// The reader ran past the end of the buffer.
    #[error("unexpected end of data")]
    UnexpectedEof,

    /// A UTF-8 string could not be decoded.
    #[error("utf-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// A feature-gated capability (e.g. ONNX Tier-1 reconstruction) is unavailable.
    #[error("unsupported operation: {0}")]
    Unsupported(String),
}
