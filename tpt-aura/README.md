# tpt-aura

Core library for the **AURA (Adaptive Universal Record Architecture)** — a
polymorphic, semantically-aware, and cryptographically sealed **master media
format**.

Unlike delivery formats (JPEG, MP4, AVIF) designed for efficient pixel delivery,
AURA is the *canonical source of truth* for digital reality capture. It natively
stores:

- **2D / 3D spatial data** via a hierarchical, typed *Record* container.
- **A Semantic DAG** of concept nodes, edges, and per-concept pixel bitmasks.
- **A C2PA-style provenance ledger** baked into the file (genesis hash +
  append-only, signed edit history).
- **A WASM bootstrap** in the header that self-describes the decoding math, so
  the file remains readable even if the standard disappears.
- **Adaptive neural encoding** (Tier 0 base layer + Tier 1 neural payload) for
  compute-scalable fidelity.

> Part of the [`tpt-aura` workspace](https://github.com/tpt-org/aura). See the
> [RFC 001 specification](https://github.com/tpt-org/aura/blob/main/docs/SPEC.md)
> for the full design. This crate is the `tpt-aura` prefix crate family's core
> library (formerly published as `libaura`).

## Features

| Module | Responsibility |
|--------|----------------|
| `container` | Typed, chunk-based record container, header, and footer (CRC-32 + SHA-3-256). |
| `provenance` | `GenesisBlock` + append-only, ed25519-signed `ProvenanceLedger` with C2PA export. |
| `bootstrap` | The embedded WASM decoder-key bootstrap. |
| `semantic` | The Semantic DAG (concept nodes, edges, RLE-compressed bitmasks). |
| `neural` | Adaptive Tier-0 / Tier-1 neural encoding. |
| `diff` | Structural diffing of two AURA files (powers `aura diff`). |
| `error` | The crate-wide `AuraError` type. |

## Installation

```toml
[dependencies]
tpt-aura = "0.1"
```

MSRV: Rust **1.74**.

## Example

```rust,no_run
use tpt_aura::container::{AuraBuilder, AuraFile, open, SceneRecord};
use tpt_aura::bootstrap::Bootstrap;
use tpt_aura::provenance::{GenesisBlock, ProvenanceLedger, sha3_256};
use tpt_aura::semantic::SemanticDAG;
use ed25519_dalek::SigningKey;

fn main() -> Result<(), tpt_aura::error::AuraError> {
    let key = SigningKey::from_bytes(&[7u8; 32]);
    let data_hash = sha3_256(b"raw-sensor");
    let genesis = GenesisBlock::sign(&key, data_hash, [1u8; 16], 0);
    let ledger = ProvenanceLedger::new(&key, data_hash);
    let scene = SceneRecord::new();
    let bytes = AuraBuilder::new(
        Bootstrap::with_default_wasm(),
        genesis,
        scene,
        SemanticDAG::new(),
        ledger,
    ).build()?;
    let file: AuraFile = open(&bytes)?;
    file.verify()?;
    Ok(())
}
```

## Semantic DAG

`SemanticDAG` holds `ConceptNode`s (a label, a confidence score, and an
RLE-compressed pixel bitmask) and `ConceptEdge`s linking them. Kahn's algorithm
detects cycles so the graph stays a DAG. Binary (de)serialization is provided for
efficient storage inside the AURA container.

## Provenance

Each AURA file begins with a `GenesisBlock` (the root SHA-3-256 hash of the
captured data, signed by the device key). Every subsequent operation appends a
signed entry to the `ProvenanceLedger`. `AuraFile::verify()` validates the full
hash chain and every signature; a single flipped bit breaks the trust seal.
A C2PA manifest export helper is included for interoperability.

## Neural encoding

- **Tier 0** — a zero-compute, box-downsampled base layer stored losslessly. It
  is always decodable and is what delivery targets compile from.
- **Tier 1** — a `NeuralPayloadRecord` container for embedded super-resolution
  weights, enabling compute-scalable fidelity on capable runtimes.

## Building and testing

```sh
cargo build -p tpt-aura
cargo test  -p tpt-aura
cargo bench -p tpt-aura   # neural throughput benchmark
```

## License

Licensed under either of [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](https://opensource.org/licenses/MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
