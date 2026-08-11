# AURA v0.1.0

Initial open-source release of the **AURA (Adaptive Universal Record Architecture)**
reference implementation — a cryptographically sealed, semantically-aware master
media format (RFC 001).

## Highlights

- **Container format** (`libaura::container`): a typed, chunk-based record
  container with a magic/version header, a section offset table, and a trailing
  CRC-32 + SHA-3-256 footer for integrity and tamper-evidence.
- **Record types**: `LuminanceChroma`, `SpatialDepth` (Z-buffer + intrinsics),
  `SpatialAudio` (ambisonic), `Temporal` (motion vectors), and a `SceneRecord`
  root that wraps child records.
- **Cryptographic provenance** (`libaura::provenance`): `GenesisBlock` + an
  append-only, ed25519-signed `ProvenanceLedger`. `verify()` validates the hash
  chain and every signature; bit-flips break the trust seal. Includes a C2PA
  manifest export helper.
- **WASM bootstrap** (`libaura::bootstrap`): a self-describing decoder-key blob
  embedded in the header so the file remains readable even if the standard
  disappears.
- **Semantic DAG** (`libaura::semantic`): concept nodes/edges with RLE-compressed
  per-concept pixel bitmasks, Kahn's cycle detection, and binary (de)serialization.
- **Neural encoding** (`libaura::neural`): a zero-compute Tier-0 base layer
  (box-downsampled, losslessly stored) plus a Tier-1 `NeuralPayloadRecord`
  container for embedded super-resolution weights.
- **CLI** (`aura-cli`): `create`, `inspect`, `verify`, `sign`, and `compile`
  (`web` / `vr` / `print` / `legal`) with an end-to-end integration test.

## Crates

| Crate | Description |
|-------|-------------|
| `libaura` | Core library (container, provenance, bootstrap, semantic, neural). |
| `aura-cli` | Command-line toolkit. |
| `aura-onnx` | ONNX inference backends (YOLOv8/SAM/CLIP) behind the `onnx` feature. |

## Build & test

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

## Known limitations (scaffolding)

- The embedded WASM is a valid but trivial decoder-key placeholder; a production
  build would embed the real AURA decoding primitives.
- Tier-1 neural reconstruction and the `aura-onnx` YOLOv8/SAM/CLIP sessions are
  **scaffolded** behind the `onnx` feature. They require ONNX Runtime (fetched at
  build time) plus model weight files; the default build uses a pure-Rust
  `StubDetector` so the workspace builds and tests fully offline.

## License

Apache-2.0 OR MIT.
