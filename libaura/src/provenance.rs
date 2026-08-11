//! Cryptographic provenance: the C2PA-style immutable trust chain.
//!
//! A [`GenesisBlock`] captures the sensor readout hash plus a hardware
//! signature, and a [`ProvenanceLedger`] is an append-only, signed history of
//! every operation applied to the asset. [`ProvenanceLedger::verify`] recomputes
//! the hash chain and checks each ed25519 signature, breaking the trust seal on
//! the first inconsistency.

use crate::codec::{Reader, Writer};
use crate::error::AuraError;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha3::{Digest, Sha3_256};

/// SHA-3-256 of an arbitrary byte slice.
pub fn sha3_256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha3_256::new();
    h.update(data);
    let out = h.finalize();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
}

fn verifying_from_bytes(b: &[u8; 32]) -> Result<VerifyingKey, AuraError> {
    VerifyingKey::try_from(b.as_slice()).map_err(|_| AuraError::InvalidLength)
}

fn signature_from_bytes(b: &[u8; 64]) -> Result<Signature, AuraError> {
    Signature::try_from(b.as_slice()).map_err(|_| AuraError::InvalidLength)
}

/// The kind of operation recorded in a [`LedgerEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    /// Raw sensor capture.
    Capture,
    /// A semantic-graph edit (mask/label change).
    SemanticEdit,
    /// A color-grade / tone operation.
    ColorGrade,
    /// A crop / geometric transform.
    Crop,
    /// A compile to a delivery target.
    Compile,
    /// Any other operation.
    Other,
}

impl OpType {
    /// Map to the on-disk `u8` tag.
    pub fn to_u8(self) -> u8 {
        match self {
            OpType::Capture => 0,
            OpType::SemanticEdit => 1,
            OpType::ColorGrade => 2,
            OpType::Crop => 3,
            OpType::Compile => 4,
            OpType::Other => 5,
        }
    }

    /// Reconstruct from the on-disk `u8` tag.
    pub fn from_u8(v: u8) -> OpType {
        match v {
            0 => OpType::Capture,
            1 => OpType::SemanticEdit,
            2 => OpType::ColorGrade,
            3 => OpType::Crop,
            4 => OpType::Compile,
            _ => OpType::Other,
        }
    }
}

/// The immutable root of the trust chain: the sensor readout hash plus a
/// hardware enclave signature over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenesisBlock {
    /// SHA-3-256 of the raw sensor data at the moment of capture.
    pub data_hash: [u8; 32],
    /// Opaque device identifier (e.g. secure-element serial).
    pub device_id: [u8; 16],
    /// Unix epoch milliseconds of capture.
    pub timestamp: u64,
    /// Signature (over `data_hash || device_id || timestamp`) produced by the
    /// camera's secure enclave.
    pub hardware_sig: Vec<u8>,
}

impl GenesisBlock {
    /// Create and sign a genesis block.
    pub fn sign(
        signing_key: &SigningKey,
        data_hash: [u8; 32],
        device_id: [u8; 16],
        timestamp: u64,
    ) -> Self {
        let mut msg = Vec::with_capacity(32 + 16 + 8);
        msg.extend_from_slice(&data_hash);
        msg.extend_from_slice(&device_id);
        msg.extend_from_slice(&timestamp.to_le_bytes());
        let sig: Signature = signing_key.sign(&msg);
        GenesisBlock {
            data_hash,
            device_id,
            timestamp,
            hardware_sig: sig.to_bytes().to_vec(),
        }
    }

    /// Verify the hardware signature against the given public key.
    pub fn verify(&self, verifying_key: &VerifyingKey) -> Result<(), AuraError> {
        let mut msg = Vec::with_capacity(32 + 16 + 8);
        msg.extend_from_slice(&self.data_hash);
        msg.extend_from_slice(&self.device_id);
        msg.extend_from_slice(&self.timestamp.to_le_bytes());
        let sig = Signature::try_from(self.hardware_sig.as_slice())
            .map_err(|_| AuraError::InvalidLength)?;
        verifying_key
            .verify(&msg, &sig)
            .map_err(|_| AuraError::SignatureInvalid)
    }

    /// Serialize into a writer.
    pub fn encode(&self, w: &mut Writer) {
        w.put_raw(&self.data_hash);
        w.put_raw(&self.device_id);
        w.put_u64(self.timestamp);
        w.put_bytes(&self.hardware_sig);
    }

    /// Deserialize from a reader.
    pub fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let data_hash = r.array::<32>()?;
        let device_id = r.array::<16>()?;
        let timestamp = r.u64()?;
        let hardware_sig = r.bytes()?;
        Ok(GenesisBlock {
            data_hash,
            device_id,
            timestamp,
            hardware_sig,
        })
    }
}

/// A single signed entry in the provenance ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    /// Operation type.
    pub op: OpType,
    /// Human-readable software that produced the operation.
    pub software: String,
    /// Hash of the asset state prior to this operation.
    pub prev_hash: [u8; 32],
    /// Hash of the asset state after this operation.
    pub resulting_hash: [u8; 32],
    /// ed25519 signature over `(prev_hash || op || software || resulting_hash)`.
    pub signature: [u8; 64],
}

impl LedgerEntry {
    fn encode(&self, w: &mut Writer) {
        w.put_u8(self.op.to_u8());
        w.put_str(&self.software);
        w.put_raw(&self.prev_hash);
        w.put_raw(&self.resulting_hash);
        w.put_raw(&self.signature);
    }

    fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let op = OpType::from_u8(r.u8()?);
        let software = r.str()?;
        let prev_hash = r.array::<32>()?;
        let resulting_hash = r.array::<32>()?;
        let signature = r.array::<64>()?;
        Ok(LedgerEntry {
            op,
            software,
            prev_hash,
            resulting_hash,
            signature,
        })
    }
}

/// Append-only, signed edit history anchored to a [`GenesisBlock`].
#[derive(Debug, Clone)]
pub struct ProvenanceLedger {
    signer: [u8; 32],
    root_hash: [u8; 32],
    current_hash: [u8; 32],
    /// The ordered list of operations.
    pub entries: Vec<LedgerEntry>,
}

impl ProvenanceLedger {
    /// Create an empty ledger anchored to `genesis_hash`, verifiable with the
    /// public half of `signing_key`.
    pub fn new(signing_key: &SigningKey, genesis_hash: [u8; 32]) -> Self {
        ProvenanceLedger {
            signer: signing_key.verifying_key().to_bytes(),
            root_hash: genesis_hash,
            current_hash: genesis_hash,
            entries: Vec::new(),
        }
    }

    /// Number of entries recorded.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the ledger has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The hash of the current (latest) asset state.
    pub fn current_hash(&self) -> [u8; 32] {
        self.current_hash
    }

    /// The genesis (root) hash the chain is anchored to.
    pub fn root_hash(&self) -> [u8; 32] {
        self.root_hash
    }

    /// Reconstruct the verifying key for the chain.
    pub fn signer_key(&self) -> Result<VerifyingKey, AuraError> {
        verifying_from_bytes(&self.signer)
    }

    /// Append a new operation, signing it against the current chain head.
    pub fn append(
        &mut self,
        op: OpType,
        software: &str,
        signing_key: &SigningKey,
    ) -> Result<(), AuraError> {
        let prev = self.current_hash;
        let mut hash_input = Vec::with_capacity(32 + 1 + software.len());
        hash_input.extend_from_slice(&prev);
        hash_input.push(op.to_u8());
        hash_input.extend_from_slice(software.as_bytes());
        let resulting = sha3_256(&hash_input);

        let mut sig_msg = Vec::with_capacity(hash_input.len() + 32);
        sig_msg.extend_from_slice(&hash_input);
        sig_msg.extend_from_slice(&resulting);
        let sig: Signature = signing_key.sign(&sig_msg);

        self.entries.push(LedgerEntry {
            op,
            software: software.to_owned(),
            prev_hash: prev,
            resulting_hash: resulting,
            signature: sig.to_bytes(),
        });
        self.current_hash = resulting;
        Ok(())
    }

    /// Verify the entire chain: hash continuity, resulting-hash integrity, and
    /// ed25519 signatures. Returns [`AuraError::TrustSealBroken`] on the first
    /// failure (bit-flip detection).
    pub fn verify(&self) -> Result<(), AuraError> {
        let vk = self.signer_key()?;
        let mut prev = self.root_hash;
        for e in &self.entries {
            if e.prev_hash != prev {
                return Err(AuraError::TrustSealBroken(format!(
                    "prev-hash link broken at entry {}",
                    e.prev_hash_hex()
                )));
            }
            let mut hash_input = Vec::with_capacity(32 + 1 + e.software.len());
            hash_input.extend_from_slice(&e.prev_hash);
            hash_input.push(e.op.to_u8());
            hash_input.extend_from_slice(e.software.as_bytes());
            let resulting = sha3_256(&hash_input);
            if e.resulting_hash != resulting {
                return Err(AuraError::TrustSealBroken(
                    "resulting-hash recomputation mismatch".to_string(),
                ));
            }
            let mut sig_msg = Vec::with_capacity(hash_input.len() + 32);
            sig_msg.extend_from_slice(&hash_input);
            sig_msg.extend_from_slice(&e.resulting_hash);
            let sig = signature_from_bytes(&e.signature)?;
            vk.verify(&sig_msg, &sig)
                .map_err(|_| AuraError::SignatureInvalid)?;
            prev = e.resulting_hash;
        }
        Ok(())
    }

    /// Export a minimal C2PA-style manifest (claim + assertions) as JSON text.
    ///
    /// This mirrors the structure a C2PA *Content Credential* would carry: a
    /// claim generator, a `c2pa.actions` assertion built from the ledger, and a
    /// hard-binding to the genesis data hash.
    pub fn to_c2pa_manifest(&self) -> Result<String, AuraError> {
        let mut actions = String::new();
        for (i, e) in self.entries.iter().enumerate() {
            if i > 0 {
                actions.push(',');
            }
            actions.push_str(&format!(
                "{{\"action\":\"{op}\",\"softwareAgent\":\"{sw}\",\"prevHash\":\"{prev}\",\"resultingHash\":\"{res}\"}}",
                op = format!("{:?}", e.op).to_lowercase(),
                sw = e.software.replace('"', "'"),
                prev = hex(&e.prev_hash),
                res = hex(&e.resulting_hash),
            ));
        }
        let manifest = format!(
            "{{\
\"claimGenerator\":\"aura-cli/0.1.0\",\
\"assertions\":{{\
\"c2pa.actions\":[{}],\
\"aura.genesis\":\"{}\"\
}}\
}}",
            actions,
            hex(&self.root_hash)
        );
        Ok(manifest)
    }

    /// Serialize into a writer.
    pub fn encode(&self, w: &mut Writer) {
        w.put_raw(&self.signer);
        w.put_raw(&self.root_hash);
        w.put_raw(&self.current_hash);
        w.put_u32(self.entries.len() as u32);
        for e in &self.entries {
            e.encode(w);
        }
    }

    /// Deserialize from a reader.
    pub fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let signer = r.array::<32>()?;
        let root_hash = r.array::<32>()?;
        let current_hash = r.array::<32>()?;
        let n = r.u32()? as usize;
        let mut entries = Vec::with_capacity(n);
        for _ in 0..n {
            entries.push(LedgerEntry::decode(r)?);
        }
        Ok(ProvenanceLedger {
            signer,
            root_hash,
            current_hash,
            entries,
        })
    }
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

impl LedgerEntry {
    fn prev_hash_hex(&self) -> String {
        hex(&self.prev_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keypair() -> SigningKey {
        // Deterministic test key (DO NOT use in production).
        SigningKey::from_bytes(&[7u8; 32])
    }

    #[test]
    fn genesis_sign_verify() {
        let k = keypair();
        let gh = sha3_256(b"raw-sensor-bytes");
        let g = GenesisBlock::sign(&k, gh, [9u8; 16], 1_700_000_000_000);
        g.verify(&k.verifying_key()).unwrap();
    }

    #[test]
    fn ledger_append_verify() {
        let k = keypair();
        let gh = sha3_256(b"raw-sensor-bytes");
        let mut ledger = ProvenanceLedger::new(&k, gh);
        ledger
            .append(OpType::Capture, "aura-capture/0.1", &k)
            .unwrap();
        ledger
            .append(OpType::ColorGrade, "aura-studio/0.1", &k)
            .unwrap();
        ledger
            .append(OpType::SemanticEdit, "aura-seg/0.1", &k)
            .unwrap();
        assert_eq!(ledger.len(), 3);
        ledger.verify().unwrap();
    }

    #[test]
    fn ledger_tamper_detected() {
        let k = keypair();
        let gh = sha3_256(b"raw-sensor-bytes");
        let mut ledger = ProvenanceLedger::new(&k, gh);
        ledger
            .append(OpType::Capture, "aura-capture/0.1", &k)
            .unwrap();
        ledger
            .append(OpType::ColorGrade, "aura-studio/0.1", &k)
            .unwrap();
        // Bit-flip the resulting hash of the last entry -> trust seal must break.
        ledger.entries[1].resulting_hash[0] ^= 0xFF;
        assert!(matches!(
            ledger.verify(),
            Err(AuraError::TrustSealBroken(_))
        ));
    }

    #[test]
    fn ledger_wrong_key_fails() {
        let k = keypair();
        let other = SigningKey::from_bytes(&[3u8; 32]);
        let gh = sha3_256(b"raw-sensor-bytes");
        let mut ledger = ProvenanceLedger::new(&k, gh);
        // Append with a different key than the one the ledger trusts.
        ledger
            .append(OpType::Capture, "aura-capture/0.1", &other)
            .unwrap();
        assert!(matches!(ledger.verify(), Err(AuraError::SignatureInvalid)));
    }

    #[test]
    fn ledger_round_trip() {
        let k = keypair();
        let gh = sha3_256(b"raw-sensor-bytes");
        let mut ledger = ProvenanceLedger::new(&k, gh);
        ledger
            .append(OpType::Capture, "aura-capture/0.1", &k)
            .unwrap();
        let mut w = Writer::new();
        ledger.encode(&mut w);
        let bytes = w.into_inner();
        let mut r = Reader::new(&bytes);
        let back = ProvenanceLedger::decode(&mut r).unwrap();
        assert_eq!(back.entries, ledger.entries);
        back.verify().unwrap();
    }

    #[test]
    fn c2pa_manifest_is_json() {
        let k = keypair();
        let gh = sha3_256(b"raw");
        let mut ledger = ProvenanceLedger::new(&k, gh);
        ledger.append(OpType::Capture, "aura/0.1", &k).unwrap();
        let m = ledger.to_c2pa_manifest().unwrap();
        assert!(m.starts_with('{') && m.contains("c2pa.actions"));
    }
}
