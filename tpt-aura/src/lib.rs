//! # tpt-aura
//!
//! Core library for the **AURA (Adaptive Universal Record Architecture)** master
//! media format: a polymorphic, semantically-aware, cryptographically sealed
//! container. See [`spec.txt`](../spec.txt) (RFC 001) for the full design.
//!
//! ## Modules
//!
//! * [`container`] — the typed record container, header, and footer.
//! * [`provenance`] — genesis block + append-only signed provenance ledger.
//! * [`bootstrap`] — the embedded WASM decoder-key bootstrap.
//! * [`semantic`] — the Semantic DAG (concept nodes, edges, bitmask RLE).
//! * [`neural`] — adaptive Tier-0 / Tier-1 neural encoding.
//! * [`diff`] — structural diffing of two AURA files (powers `aura diff`).
//!
//! ## Example
//!
//! ```no_run
//! use tpt_aura::container::{AuraBuilder, AuraFile, open, SceneRecord};
//! use tpt_aura::bootstrap::Bootstrap;
//! use tpt_aura::provenance::{GenesisBlock, ProvenanceLedger, sha3_256};
//! use tpt_aura::semantic::SemanticDAG;
//! use ed25519_dalek::SigningKey;
//!
//! # fn main() -> Result<(), tpt_aura::error::AuraError> {
//! let key = SigningKey::from_bytes(&[7u8; 32]);
//! let data_hash = sha3_256(b"raw-sensor");
//! let genesis = GenesisBlock::sign(&key, data_hash, [1u8; 16], 0);
//! let ledger = ProvenanceLedger::new(&key, data_hash);
//! let scene = SceneRecord::new();
//! let bytes = AuraBuilder::new(
//!     Bootstrap::with_default_wasm(),
//!     genesis,
//!     scene,
//!     SemanticDAG::new(),
//!     ledger,
//! ).build()?;
//! let file: AuraFile = open(&bytes)?;
//! file.verify()?;
//! # Ok(())
//! # }
//! ```

pub mod bootstrap;
pub mod codec;
pub mod container;
pub mod diff;
pub mod error;
pub mod neural;
pub mod provenance;
pub mod semantic;

pub use error::AuraError;
pub use neural::RgbImage;
