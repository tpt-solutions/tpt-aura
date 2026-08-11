//! The polymorphic AURA container: a typed, chunk-based record format.
//!
//! Layout (see RFC 001 §8):
//! ```text
//! [ MAGIC + VERSION + SECTION TABLE ]
//! [ BOOTSTRAP ] [ GENESIS ] [ RECORDS ] [ SEMANTIC ] [ LEDGER ]
//! [ FOOTER: CRC-32 + SHA3-256 ]
//! ```
//!
//! The header carries absolute byte offsets to every section, so a reader can
//! jump straight to the records it supports. The footer validates the whole
//! stream with a CRC-32 (integrity) and a SHA-3-256 (tamper-evidence).

use crate::bootstrap::Bootstrap;
use crate::codec::{crc32, Reader, Writer};
use crate::error::AuraError;
use crate::provenance::{sha3_256, GenesisBlock, ProvenanceLedger};
use crate::semantic::SemanticDAG;

/// File magic: ASCII `"AURA"`.
pub const AURA_MAGIC: [u8; 4] = *b"AURA";
/// Current major version.
pub const VERSION_MAJOR: u16 = 1;
/// Current minor version.
pub const VERSION_MINOR: u16 = 0;

/// Section identifiers used in the header table.
pub const SECTION_BOOTSTRAP: u8 = 1;
/// Provenance genesis block.
pub const SECTION_GENESIS: u8 = 2;
/// Root `SCENE_RECORD` (polymorphic records).
pub const SECTION_RECORDS: u8 = 3;
/// Semantic DAG.
pub const SECTION_SEMANTIC: u8 = 4;
/// Provenance ledger.
pub const SECTION_LEDGER: u8 = 5;

/// Size of the trailing footer (CRC-32 + SHA-3-256).
const FOOTER_SIZE: usize = 4 + 32;

/// Record type tags.
pub const REC_LUMINANCE_CHROMA: u8 = 0x10;
/// Spatial depth / Z-buffer record.
pub const REC_SPATIAL_DEPTH: u8 = 0x11;
/// Spatial (ambisonic) audio record.
pub const REC_SPATIAL_AUDIO: u8 = 0x12;
/// Temporal frame-timing / motion-vector record.
pub const REC_TEMPORAL: u8 = 0x13;
/// Neural Tier-1 reconstruction payload record.
pub const REC_NEURAL_PAYLOAD: u8 = 0x14;
/// Root scene record.
pub const REC_SCENE: u8 = 0x1F;

/// A single typed section entry in the header table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionEntry {
    /// Section kind (see `SECTION_*` constants).
    pub section_type: u8,
    /// Absolute byte offset of the section within the file.
    pub offset: u64,
    /// Byte length of the section.
    pub length: u64,
}

/// A polymorphic record within the AURA container.
pub trait Record: std::fmt::Debug {
    /// The record's type tag.
    fn type_tag(&self) -> u8;
    /// Serialize the record into a writer.
    fn encode(&self, w: &mut Writer);
    /// Return `self` as `&dyn Any` for downcasting.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Standard 2D color data (YCbCr or RGB).
#[derive(Debug, Clone, PartialEq)]
pub struct LuminanceChromaRecord {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bits per sample (typically 8).
    pub depth: u8,
    /// Sampling: `0` = YCbCr 4:2:0, `1` = RGB.
    pub sampling: u8,
    /// Packed sample bytes.
    pub data: Vec<u8>,
}

impl LuminanceChromaRecord {
    fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let width = r.u32()?;
        let height = r.u32()?;
        let depth = r.u8()?;
        let sampling = r.u8()?;
        let data = r.bytes()?;
        Ok(LuminanceChromaRecord {
            width,
            height,
            depth,
            sampling,
            data,
        })
    }
}

impl Record for LuminanceChromaRecord {
    fn type_tag(&self) -> u8 {
        REC_LUMINANCE_CHROMA
    }
    fn encode(&self, w: &mut Writer) {
        w.put_u32(self.width);
        w.put_u32(self.height);
        w.put_u8(self.depth);
        w.put_u8(self.sampling);
        w.put_bytes(&self.data);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// LiDAR / depth map with camera intrinsics `[fx, fy, cx, cy]`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialDepthRecord {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Camera intrinsics `[fx, fy, cx, cy]`.
    pub intrinsics: [f32; 4],
    /// Z-buffer values in millimetres (one per pixel).
    pub depth: Vec<u16>,
}

impl SpatialDepthRecord {
    fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let width = r.u32()?;
        let height = r.u32()?;
        let mut intrinsics = [0f32; 4];
        for x in intrinsics.iter_mut() {
            *x = r.f32()?;
        }
        let n = r.u32()? as usize;
        let mut depth = Vec::with_capacity(n);
        for _ in 0..n {
            depth.push(r.u16()?);
        }
        Ok(SpatialDepthRecord {
            width,
            height,
            intrinsics,
            depth,
        })
    }
}

impl Record for SpatialDepthRecord {
    fn type_tag(&self) -> u8 {
        REC_SPATIAL_DEPTH
    }
    fn encode(&self, w: &mut Writer) {
        w.put_u32(self.width);
        w.put_u32(self.height);
        for x in self.intrinsics.iter() {
            w.put_f32(*x);
        }
        w.put_u32(self.depth.len() as u32);
        for d in &self.depth {
            w.put_u16(*d);
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Ambisonic / object-based 3D audio track.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialAudioRecord {
    /// Channel count.
    pub channels: u8,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Frame count.
    pub frames: u32,
    /// Encoded audio bytes.
    pub data: Vec<u8>,
}

impl SpatialAudioRecord {
    fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let channels = r.u8()?;
        let sample_rate = r.u32()?;
        let frames = r.u32()?;
        let data = r.bytes()?;
        Ok(SpatialAudioRecord {
            channels,
            sample_rate,
            frames,
            data,
        })
    }
}

impl Record for SpatialAudioRecord {
    fn type_tag(&self) -> u8 {
        REC_SPATIAL_AUDIO
    }
    fn encode(&self, w: &mut Writer) {
        w.put_u8(self.channels);
        w.put_u32(self.sample_rate);
        w.put_u32(self.frames);
        w.put_bytes(&self.data);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Frame timing + delta-motion vectors for video captures.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalRecord {
    /// Number of frames.
    pub frame_count: u32,
    /// Frames per second.
    pub fps: f32,
    /// Per-macroblock motion vectors `(dx, dy)`.
    pub motion_vectors: Vec<(i16, i16)>,
}

impl TemporalRecord {
    fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let frame_count = r.u32()?;
        let fps = r.f32()?;
        let n = r.u32()? as usize;
        let mut motion_vectors = Vec::with_capacity(n);
        for _ in 0..n {
            motion_vectors.push((r.i16()?, r.i16()?));
        }
        Ok(TemporalRecord {
            frame_count,
            fps,
            motion_vectors,
        })
    }
}

impl Record for TemporalRecord {
    fn type_tag(&self) -> u8 {
        REC_TEMPORAL
    }
    fn encode(&self, w: &mut Writer) {
        w.put_u32(self.frame_count);
        w.put_f32(self.fps);
        w.put_u32(self.motion_vectors.len() as u32);
        for &(dx, dy) in &self.motion_vectors {
            w.put_i16(dx);
            w.put_i16(dy);
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Writer {
    fn put_i16(&mut self, v: i16) {
        self.put_u16(v as u16);
    }
}

impl Reader<'_> {
    fn i16(&mut self) -> Result<i16, AuraError> {
        Ok(self.u16()? as i16)
    }
}

/// The root container; wraps an ordered list of child records.
#[derive(Debug, Default)]
pub struct SceneRecord {
    /// Child records (leaves such as luminance, depth, audio, temporal).
    pub children: Vec<Box<dyn Record>>,
}

impl SceneRecord {
    /// An empty scene.
    pub fn new() -> Self {
        SceneRecord::default()
    }

    /// Push a child record.
    pub fn push(&mut self, record: Box<dyn Record>) {
        self.children.push(record);
    }

    fn encode(&self, w: &mut Writer) {
        w.put_u32(self.children.len() as u32);
        for c in &self.children {
            w.put_u8(c.type_tag());
            c.encode(w);
        }
    }

    fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let n = r.u32()? as usize;
        let mut children = Vec::with_capacity(n);
        for _ in 0..n {
            let tag = r.u8()?;
            children.push(decode_record(tag, r)?);
        }
        Ok(SceneRecord { children })
    }
}

fn decode_record(tag: u8, r: &mut Reader) -> Result<Box<dyn Record>, AuraError> {
    match tag {
        REC_LUMINANCE_CHROMA => Ok(Box::new(LuminanceChromaRecord::decode(r)?)),
        REC_SPATIAL_DEPTH => Ok(Box::new(SpatialDepthRecord::decode(r)?)),
        REC_SPATIAL_AUDIO => Ok(Box::new(SpatialAudioRecord::decode(r)?)),
        REC_TEMPORAL => Ok(Box::new(TemporalRecord::decode(r)?)),
        other => Err(AuraError::UnknownRecord(other)),
    }
}

/// Parsed file header (magic, version, and section table).
#[derive(Debug, Clone)]
pub struct AuraHeader {
    /// Major version.
    pub version_major: u16,
    /// Minor version.
    pub version_minor: u16,
    /// Section table.
    pub sections: Vec<SectionEntry>,
}

/// A fully parsed AURA file.
#[derive(Debug)]
pub struct AuraFile {
    /// Header.
    pub header: AuraHeader,
    /// WASM bootstrap (decoder key).
    pub bootstrap: Bootstrap,
    /// Provenance genesis block.
    pub genesis: GenesisBlock,
    /// Root scene record.
    pub scene: SceneRecord,
    /// Semantic DAG.
    pub dag: SemanticDAG,
    /// Provenance ledger.
    pub ledger: ProvenanceLedger,
}

impl AuraFile {
    /// Verify the trust chain (ledger) and the bootstrap validity.
    pub fn verify(&self) -> Result<(), AuraError> {
        if !self.bootstrap.validate() {
            return Err(AuraError::TrustSealBroken(
                "bootstrap WASM failed validation".into(),
            ));
        }
        self.ledger.verify()
    }
}

/// Builder that accumulates all sections and serializes a complete AURA file.
pub struct AuraBuilder {
    /// WASM bootstrap.
    pub bootstrap: Bootstrap,
    /// Provenance genesis.
    pub genesis: GenesisBlock,
    /// Root scene.
    pub scene: SceneRecord,
    /// Semantic DAG.
    pub dag: SemanticDAG,
    /// Provenance ledger.
    pub ledger: ProvenanceLedger,
}

impl AuraBuilder {
    /// Start from the required components.
    pub fn new(
        bootstrap: Bootstrap,
        genesis: GenesisBlock,
        scene: SceneRecord,
        dag: SemanticDAG,
        ledger: ProvenanceLedger,
    ) -> Self {
        AuraBuilder {
            bootstrap,
            genesis,
            scene,
            dag,
            ledger,
        }
    }

    /// Serialize the builder into a complete AURA byte stream.
    pub fn build(&self) -> Result<Vec<u8>, AuraError> {
        let mut boot = Writer::new();
        self.bootstrap.write(&mut boot);
        let mut gen = Writer::new();
        self.genesis.encode(&mut gen);
        let mut rec = Writer::new();
        self.scene.encode(&mut rec);
        let mut sem = Writer::new();
        self.dag.encode(&mut sem);
        let mut led = Writer::new();
        self.ledger.encode(&mut led);

        let sections: [(u8, Vec<u8>); 5] = [
            (SECTION_BOOTSTRAP, boot.into_inner()),
            (SECTION_GENESIS, gen.into_inner()),
            (SECTION_RECORDS, rec.into_inner()),
            (SECTION_SEMANTIC, sem.into_inner()),
            (SECTION_LEDGER, led.into_inner()),
        ];

        let count = sections.len();
        let header_size = (4 + 2 + 2 + 2) + count * (1 + 8 + 8);
        let mut cursor = header_size as u64;
        let mut offsets = vec![0u64; count];
        for (i, (_, buf)) in sections.iter().enumerate() {
            offsets[i] = cursor;
            cursor += buf.len() as u64;
        }

        let mut out = Vec::with_capacity(cursor as usize + FOOTER_SIZE);
        out.extend_from_slice(&AURA_MAGIC);
        out.extend_from_slice(&VERSION_MAJOR.to_le_bytes());
        out.extend_from_slice(&VERSION_MINOR.to_le_bytes());
        out.extend_from_slice(&(count as u16).to_le_bytes());
        for i in 0..count {
            out.push(sections[i].0);
            out.extend_from_slice(&offsets[i].to_le_bytes());
            out.extend_from_slice(&(sections[i].1.len() as u64).to_le_bytes());
        }
        for (_, buf) in &sections {
            out.extend_from_slice(buf);
        }

        let crc = crc32(&out);
        let hash = sha3_256(&out);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&hash);
        Ok(out)
    }
}

/// Parse an AURA byte stream into a typed [`AuraFile`].
pub fn open(data: &[u8]) -> Result<AuraFile, AuraError> {
    if data.len() < 10 + FOOTER_SIZE {
        return Err(AuraError::UnexpectedEof);
    }
    let mut found = [0u8; 4];
    found.copy_from_slice(&data[0..4]);
    if found != AURA_MAGIC {
        return Err(AuraError::BadMagic {
            expected: AURA_MAGIC,
            found,
        });
    }
    let version_major = u16::from_le_bytes([data[4], data[5]]);
    let version_minor = u16::from_le_bytes([data[6], data[7]]);
    if version_major > VERSION_MAJOR {
        return Err(AuraError::UnsupportedVersion(version_major, version_minor));
    }
    let count = u16::from_le_bytes([data[8], data[9]]) as usize;
    let mut pos = 10usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if pos + 17 > data.len() {
            return Err(AuraError::UnexpectedEof);
        }
        let section_type = data[pos];
        pos += 1;
        let offset = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let length = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;
        entries.push(SectionEntry {
            section_type,
            offset,
            length,
        });
    }

    let footer_start = data.len() - FOOTER_SIZE;
    if footer_start < pos {
        return Err(AuraError::UnexpectedEof);
    }
    let crc_stored = u32::from_le_bytes(data[footer_start..footer_start + 4].try_into().unwrap());
    let hash_stored = &data[footer_start + 4..footer_start + 36];
    let crc_computed = crc32(&data[..footer_start]);
    if crc_stored != crc_computed {
        return Err(AuraError::ChecksumMismatch {
            stored: crc_stored,
            computed: crc_computed,
        });
    }
    let hash_computed = sha3_256(&data[..footer_start]);
    if hash_computed != hash_stored {
        return Err(AuraError::HashMismatch);
    }

    let section_bytes = |ty: u8| -> Result<&[u8], AuraError> {
        let e = entries
            .iter()
            .find(|e| e.section_type == ty)
            .ok_or(AuraError::MissingSection(ty))?;
        let start = e.offset as usize;
        let end = start + e.length as usize;
        if end > footer_start {
            return Err(AuraError::UnexpectedEof);
        }
        Ok(&data[start..end])
    };

    let bootstrap = {
        let mut r = Reader::new(section_bytes(SECTION_BOOTSTRAP)?);
        Bootstrap::read(&mut r)?
    };
    let genesis = {
        let mut r = Reader::new(section_bytes(SECTION_GENESIS)?);
        GenesisBlock::decode(&mut r)?
    };
    let scene = {
        let mut r = Reader::new(section_bytes(SECTION_RECORDS)?);
        SceneRecord::decode(&mut r)?
    };
    let dag = {
        let mut r = Reader::new(section_bytes(SECTION_SEMANTIC)?);
        SemanticDAG::decode(&mut r)?
    };
    let ledger = {
        let mut r = Reader::new(section_bytes(SECTION_LEDGER)?);
        ProvenanceLedger::decode(&mut r)?
    };

    Ok(AuraFile {
        header: AuraHeader {
            version_major,
            version_minor,
            sections: entries,
        },
        bootstrap,
        genesis,
        scene,
        dag,
        ledger,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::neural::{encode_tier0, RgbImage};
    use crate::provenance::{OpType, ProvenanceLedger};
    use ed25519_dalek::SigningKey;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn sample_scene() -> SceneRecord {
        let mut img = RgbImage::new(8, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                img.set_pixel(x, y, (x as u8, y as u8, 0));
            }
        }
        let base = encode_tier0(&img, 2);
        let rec = LuminanceChromaRecord {
            width: base.base_width(),
            height: base.base_height(),
            depth: 8,
            sampling: 1,
            data: base.data().to_vec(),
        };
        let mut scene = SceneRecord::new();
        scene.push(Box::new(rec));
        scene
    }

    fn build_file() -> Vec<u8> {
        let k = key();
        let data_hash = sha3_256(b"sensor");
        let genesis = GenesisBlock::sign(&k, data_hash, [1u8; 16], 1_700_000_000_000);
        let ledger = ProvenanceLedger::new(&k, data_hash);
        let scene = sample_scene();
        let dag = SemanticDAG::new();
        let bootstrap = Bootstrap::with_default_wasm();
        let builder = AuraBuilder::new(bootstrap, genesis, scene, dag, ledger);
        builder.build().unwrap()
    }

    #[test]
    fn write_then_read_round_trip() {
        let bytes = build_file();
        let file = open(&bytes).unwrap();
        assert_eq!(file.header.version_major, VERSION_MAJOR);
        assert!(file.bootstrap.validate());
        assert_eq!(file.scene.children.len(), 1);
        assert!(file.ledger.is_empty());
        assert!(file.dag.nodes.is_empty());
    }

    #[test]
    fn footer_crc_and_hash_validate() {
        let bytes = build_file();
        open(&bytes).unwrap(); // open() validates the footer itself
    }

    #[test]
    fn tampering_breaks_integrity() {
        let mut bytes = build_file();
        // Flip a byte in the records section body.
        bytes[120] ^= 0xFF;
        assert!(matches!(
            open(&bytes),
            Err(AuraError::ChecksumMismatch { .. }) | Err(AuraError::HashMismatch)
        ));
    }

    #[test]
    fn trust_chain_verifies_on_opened_file() {
        let bytes = build_file();
        let file = open(&bytes).unwrap();
        file.verify().unwrap();
    }

    #[test]
    fn downcast_child_record() {
        let bytes = build_file();
        let file = open(&bytes).unwrap();
        let rec = file.scene.children[0]
            .as_any()
            .downcast_ref::<LuminanceChromaRecord>()
            .unwrap();
        assert_eq!(rec.width, 4);
        assert_eq!(rec.height, 4);
    }

    #[test]
    fn signed_ledger_survives_round_trip() {
        let k = key();
        let data_hash = sha3_256(b"sensor");
        let genesis = GenesisBlock::sign(&k, data_hash, [1u8; 16], 0);
        let mut ledger = ProvenanceLedger::new(&k, data_hash);
        ledger.append(OpType::Capture, "aura/0.1", &k).unwrap();
        let scene = sample_scene();
        let builder = AuraBuilder::new(
            Bootstrap::with_default_wasm(),
            genesis,
            scene,
            SemanticDAG::new(),
            ledger,
        );
        let bytes = builder.build().unwrap();
        let file = open(&bytes).unwrap();
        assert_eq!(file.ledger.len(), 1);
        file.verify().unwrap();
    }

    #[test]
    fn truncated_section_table_errors_instead_of_panicking() {
        // Valid magic/version, but `count` (5) declares far more section
        // entries than the buffer actually has room for. This used to panic
        // via an out-of-bounds slice index; it must now return a clean error.
        let mut bytes = vec![0u8; 10 + FOOTER_SIZE];
        bytes[0..4].copy_from_slice(&AURA_MAGIC);
        bytes[4..6].copy_from_slice(&VERSION_MAJOR.to_le_bytes());
        bytes[6..8].copy_from_slice(&VERSION_MINOR.to_le_bytes());
        bytes[8..10].copy_from_slice(&5u16.to_le_bytes());
        assert!(matches!(open(&bytes), Err(AuraError::UnexpectedEof)));
    }
}
