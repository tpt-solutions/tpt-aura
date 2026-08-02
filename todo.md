# AURA Platform — Task Checklist

> Rust workspace: `libaura` (core library) + `aura-cli` (CLI binary) + `aura-onnx` (ONNX inference)
> License: Apache 2.0 + MIT | Open source

---

## Phase 0 — Project Setup

- [x] Initialize Cargo workspace (`Cargo.toml` with members)
- [x] Create `libaura/` crate
- [x] Create `aura-cli/` crate
- [x] Create `aura-onnx/` crate
- [x] Add LICENSE-APACHE (Apache 2.0)
- [x] Add LICENSE-MIT
- [x] Write root README.md with project overview + build instructions
- [ ] Set up GitHub Actions CI (test + clippy + fmt)
- [x] Add .gitignore

---

## Phase 1 — Container Format (`libaura::container`)

- [ ] Define `AuraHeader` struct (magic bytes, version, section offsets)
- [ ] Define `Record` trait with type tag, serialize, deserialize
- [ ] Implement `LuminanceChromaRecord` (YCbCr pixel data)
- [ ] Implement `SpatialDepthRecord` (Z-buffer + camera intrinsics)
- [ ] Implement `SpatialAudioRecord` (ambisonic track bytes)
- [ ] Implement `TemporalRecord` (frame timing + motion vectors)
- [ ] Implement `SceneRecord` root container (Vec of child records)
- [ ] Implement file writer: header → bootstrap → provenance → records → DAG → ledger → footer
- [ ] Implement file reader: parse all sections into typed structs
- [ ] Implement footer + CRC/checksum validation
- [ ] Unit tests: round-trip write → read for each record type

---

## Phase 2 — Cryptographic Provenance (`libaura::provenance`)

- [ ] Add `sha3` crate dependency
- [ ] Add `ed25519-dalek` crate dependency
- [ ] Implement `GenesisBlock` (data hash, hardware sig field, timestamp, device ID)
- [ ] Implement `LedgerEntry` (op type, software, resulting hash, sig)
- [ ] Implement `ProvenanceLedger` (append-only vec, serialization)
- [ ] Implement `ledger.append(entry)` operation
- [ ] Implement `ledger.verify()` chain validation
- [ ] Implement trust seal break detection (bit-flip test)
- [ ] Implement C2PA export helper
- [ ] Unit tests: build ledger, append entries, verify, tamper + expect failure

---

## Phase 3 — WASM Bootstrap (`libaura::bootstrap`)

- [ ] Write minimal WAT (WebAssembly Text) decoder for AURA primitives
- [ ] Compile WAT → `.wasm` binary
- [ ] Embed `.wasm` as `include_bytes!` in `bootstrap/` module
- [ ] Implement `Bootstrap::write(writer)` — writes Wasm blob into file header
- [ ] Implement `Bootstrap::read(reader)` — extracts and validates Wasm blob
- [ ] Unit test: write bootstrap, read back, verify byte equality

---

## Phase 4 — Semantic DAG (`libaura::semantic`)

- [ ] Define `ConceptNode` struct (ID, label, confidence, bitmask)
- [ ] Define `ConceptEdge` struct (source, target, relationship)
- [ ] Define `SemanticDAG` struct (nodes, edges)
- [ ] Implement DAG cycle detection (Kahn's algorithm)
- [ ] Implement bitmask RLE compression/decompression
- [ ] Implement DAG serialization → binary section
- [ ] Implement DAG deserialization from binary
- [ ] Implement `aura-onnx`: YOLOv8 session + inference → `Vec<ConceptNode>`
- [ ] Implement `aura-onnx`: SAM session + inference → per-node bitmasks
- [ ] Implement `aura-onnx`: CLIP session + inference → label confidence scoring
- [ ] Unit tests: DAG round-trip, cycle detection, bitmask compress/decompress
- [ ] Integration test: run ONNX inference on sample image, confirm non-empty DAG

---

## Phase 5 — Neural Encoding (`libaura::neural`)

- [ ] Add `ort` (OnnxRuntime) dependency to `aura-onnx`
- [ ] Implement Tier 0 encoder: downsample + wavelet compress → base layer bytes
- [ ] Implement Tier 0 decoder: base layer bytes → low-res image
- [ ] Download/bundle Real-ESRGAN ONNX model weights
- [ ] Implement Tier 1 payload write: embed ONNX weights as `NeuralPayloadRecord`
- [ ] Implement Tier 1 inference: base layer + ONNX weights → full-res image
- [ ] Expose `encode(image) → (base_bytes, payload_bytes)` in `libaura`
- [ ] Expose `decode(base_bytes, payload_bytes) → image` in `libaura`
- [ ] Unit test: encode → decode, verify image dimensions + PSNR > threshold

---

## Phase 6 — CLI (`aura-cli`)

- [ ] Add `clap` dependency to `aura-cli`
- [ ] Implement `aura create <input> -o <output.aura>`
- [ ] Implement `aura inspect <file.aura>` (pretty-print all sections)
- [ ] Implement `aura verify <file.aura>` (trust chain report)
- [ ] Implement `aura sign <file.aura> --key <key-file>`
- [ ] Implement `aura compile --target web` (→ AVIF/WebP)
- [ ] Implement `aura compile --target vr` (→ USDA USD stub)
- [ ] Implement `aura compile --target print` (→ TIFF via Tier 1 ONNX)
- [ ] Implement `aura compile --target legal` (→ JPEG + C2PA credentials)
- [ ] End-to-end CLI integration test (create → inspect → verify → compile)

---

## Phase 7 — Testing & Docs

- [ ] Write rustdoc for all public API items in `libaura`
- [ ] Write CLI help text and usage examples
- [ ] Add CONTRIBUTING.md
- [ ] Add GitHub issue templates (bug, feature)
- [ ] Add benchmark suite (`criterion`) for encode/decode throughput
- [ ] Verify `cargo test --workspace` all pass
- [ ] Verify `cargo clippy --workspace -- -D warnings` clean
- [ ] Verify `cargo fmt --check` clean

---

## Phase 8 — Open Source Release

- [ ] Initialize GitHub repo + push initial commit
- [ ] Set up branch protection on `main`
- [ ] Add PR template
- [ ] Verify GitHub Actions CI passes on first push
- [ ] Configure `crates.io` publish metadata in `Cargo.toml`
- [ ] Tag v0.1.0
- [ ] Write v0.1.0 release notes
