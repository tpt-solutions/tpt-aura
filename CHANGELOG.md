# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `aura diff <a.aura> <b.aura>` (`libaura::diff`): visualize Semantic-DAG,
  record-layout, and provenance-ledger changes between two file versions, with a
  `--json` machine-readable mode.
- Pluggable detector backends in `aura-onnx`: a `Backend` enum, `detector_for()`,
  and an adaptive `default_detector()` that picks the best compiled-in runtime.
  New `coreml` / `tensorrt` feature gates scaffold those backends (returning
  `Unsupported` until the native SDK bindings land); `aura create` gains a
  `--backend <stub|ort|coreml|tensorrt>` flag.
- `.github/actions/verify-aura`: a composite GitHub Action that builds `aura` and
  runs `aura verify` across every `.aura` file — dogfooding C2PA-style provenance
  on the project's own media assets. Wired into CI as the `verify-aura` job over
  `examples/assets` (which now ships a signed `sample.aura`).
- `web/`: a zero-dependency browser viewer that parses an `.aura` file in plain
  JavaScript, renders the Tier-0 base image, lists the Semantic DAG/ledger, and
  instantiates the embedded WASM bootstrap live in-browser — proving the
  self-decoding design end to end.
- `examples/quickstart.rs`: runnable create → inspect → verify → sign → compile
  demo that needs zero external downloads (generates an in-memory image).
- `SECURITY.md` with vulnerability reporting guidance.
- `dependabot.yml` for automated dependency updates.
- `CODEOWNERS` pointing at the core maintainers.
- `.github/workflows/release.yml`: publish `libaura`/`aura-cli`/`aura-onnx` to
  crates.io and cut a GitHub release on tag push.
- `.github/workflows/ci.yml` now runs a 3-OS matrix (`ubuntu`/`windows`/`macos`)
  and additionally tests the declared MSRV (`rust-version = "1.74"`).

### Changed
- README Quick Start now uses the correct binary name (`aura`, not `aura-cli`)
  and documents the `--target web` PNG/JPEG output instead of the unsupported
  `.avif` extension.
- Removed unused `serde` / `bincode` workspace dependencies (no crate referenced
  them directly).

## [0.1.0] - 2026-08-11

Initial open-source release of the **AURA (Adaptive Universal Record
Architecture)** reference implementation — a cryptographically sealed,
semantically-aware master media format (RFC 001).

### Added
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

### Known limitations (scaffolding)
- The embedded WASM is a valid but trivial decoder-key placeholder; a production
  build would embed the real AURA decoding primitives.
- Tier-1 neural reconstruction and the `aura-onnx` YOLOv8/SAM/CLIP sessions are
  **scaffolded** behind the `onnx` feature. They require ONNX Runtime (fetched at
  build time) plus model weight files; the default build uses a pure-Rust
  `StubDetector` so the workspace builds and tests fully offline.

[Unreleased]: https://github.com/tpt-org/aura/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/tpt-org/aura/releases/tag/v0.1.0
