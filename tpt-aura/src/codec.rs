//! Low-level little-endian binary read/write helpers and a CRC-32 implementation.
//!
//! The AURA container is a flat byte stream. [`Writer`] accumulates bytes; [`Reader`]
//! consumes them with bounds checking that yields [`AuraError::UnexpectedEof`].

use crate::error::AuraError;

/// Accumulates little-endian primitives into a byte buffer.
#[derive(Debug, Default, Clone)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    /// Create an empty writer.
    pub fn new() -> Self {
        Writer { buf: Vec::new() }
    }

    /// Number of bytes accumulated so far.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether no bytes have been written.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Consume the writer and return its bytes.
    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    pub fn put_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn put_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn put_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn put_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn put_f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn put_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Length-prefixed (`u32`) raw bytes.
    pub fn put_bytes(&mut self, b: &[u8]) {
        self.put_u32(b.len() as u32);
        self.buf.extend_from_slice(b);
    }

    /// Length-prefixed UTF-8 string.
    pub fn put_str(&mut self, s: &str) {
        self.put_bytes(s.as_bytes());
    }

    /// Append a raw, unprefixed byte slice.
    pub fn put_raw(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
}

/// Consumes a byte slice with bounds-checked little-endian reads.
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Wrap a slice for reading from the start.
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    /// Bytes remaining unread.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], AuraError> {
        if self.pos + n > self.data.len() {
            return Err(AuraError::UnexpectedEof);
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn u8(&mut self) -> Result<u8, AuraError> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, AuraError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Result<u32, AuraError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u64(&mut self) -> Result<u64, AuraError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn f32(&mut self) -> Result<f32, AuraError> {
        let b = self.take(4)?;
        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn f64(&mut self) -> Result<f64, AuraError> {
        let b = self.take(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    /// Read a length-prefixed (`u32`) byte vector.
    pub fn bytes(&mut self) -> Result<Vec<u8>, AuraError> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }

    /// Read a length-prefixed UTF-8 string.
    pub fn str(&mut self) -> Result<String, AuraError> {
        let b = self.bytes()?;
        Ok(std::str::from_utf8(&b)?.to_owned())
    }

    /// Read exactly `n` raw bytes as a vector.
    pub fn raw(&mut self, n: usize) -> Result<Vec<u8>, AuraError> {
        Ok(self.take(n)?.to_vec())
    }

    /// Read a fixed-size array of `N` bytes.
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], AuraError> {
        let b = self.take(N)?;
        let mut a = [0u8; N];
        a.copy_from_slice(b);
        Ok(a)
    }

    /// Skip `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<(), AuraError> {
        self.take(n)?;
        Ok(())
    }
}

/// IEEE 802.3 CRC-32 (polynomial `0xEDB8_8320`, reflected).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Marker trait so concrete record types can be downcast from `Box<dyn Record>`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_primitives() {
        let mut w = Writer::new();
        w.put_u8(0xAB);
        w.put_u16(0x1234);
        w.put_u32(0xDEAD_BEEF);
        w.put_u64(0x0102_0304_0506_0708);
        w.put_f32(3.5);
        w.put_str("hello");
        w.put_bytes(&[1, 2, 3]);
        let bytes = w.into_inner();

        let mut r = Reader::new(&bytes);
        assert_eq!(r.u8().unwrap(), 0xAB);
        assert_eq!(r.u16().unwrap(), 0x1234);
        assert_eq!(r.u32().unwrap(), 0xDEAD_BEEF);
        assert_eq!(r.u64().unwrap(), 0x0102_0304_0506_0708);
        assert_eq!(r.f32().unwrap(), 3.5);
        assert_eq!(r.str().unwrap(), "hello");
        assert_eq!(r.bytes().unwrap(), vec![1, 2, 3]);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn crc_is_stable() {
        assert_eq!(crc32(b""), 0x0000_0000);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
