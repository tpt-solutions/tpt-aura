# Changelog

All notable changes to the `tpt-aura` crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `diff` module: structural diffing of two AURA files (Semantic-DAG, record
  layout, and provenance-ledger changes) with a `Display` report and `--json`
  machine-readable output (powers `tpt-aura-cli diff`).
- Pluggable detector backend selection in `tpt-aura-onnx` (the `Backend` enum,
  `detector_for()`, and an adaptive `default_detector()`); the CLI exposes this
  via `aura create --backend <stub|ort|coreml|tensorrt>`.

### Changed
- Crate renamed from `libaura` to `tpt-aura` so every crate in the workspace
  shares the `tpt-aura` prefix. The crate id is now `tpt_aura`.

## [0.1.0] - 2026-08-11

Initial open-source release of the **AURA (Adaptive Universal Record
Architecture)** core library — a cryptographically sealed, semantically-aware
master media format (RFC 001).

### Added
- **Container format** (`tpt-aura::container`): a typed, chunk-based record
  container with a magic/version header, a section offset table, and a trailing
  CRC-32 + SHA-3-256 footer for integrity and tamper-evidence.
- **Record types**: `LuminanceChroma`, `SpatialDepth` (Z-buffer + intrinsics),
  `SpatialAudio` (ambisonic), `Temporal` (motion vectors), and a `SceneRecord`
  root that wraps child records.
- **Cryptographic provenance** (`tpt-aura::provenance`): `GenesisBlock` + an
  append-only, ed25519-signed `ProvenanceLedger`. `verify()` validates the hash
  chain and every signature; bit-flips break the trust seal. Includes a C2PA
  manifest export helper.
- **WASM bootstrap** (`tpt-aura::bootstrap`): a self-describing decoder-key blob
  embedded in the header so the file remains readable even if the standard
  disappears.
- **Semantic DAG** (`tpt-aura::semantic`): concept nodes/edges with RLE-compressed
  per-concept pixel bitmasks, Kahn's cycle detection, and binary (de)serialization.
- **Neural encoding** (`tpt-aura::neural`): a zero-compute Tier-0 base layer
  (box-downsampled, losslessly stored) plus a Tier-1 `NeuralPayloadRecord`
  container for embedded super-resolution weights.

### Known limitations (scaffolding)
- The embedded WASM is a valid but trivial decoder-key placeholder; a production
  build would embed the real AURA decoding primitives.
- Tier-1 neural reconstruction and the `tpt-aura-onnx` YOLOv8/SAM/CLIP sessions
  are **scaffolded** behind the `onnx` feature. They require ONNX Runtime (fetched
  at build time) plus model weight files; the default build uses a pure-Rust
  `StubDetector` so the workspace builds and tests fully offline.

[Unreleased]: https://github.com/tpt-org/aura/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/tpt-org/aura/releases/tag/v0.1.0
