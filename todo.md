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
- [x] Set up GitHub Actions CI (test + clippy + fmt)
- [x] Add .gitignore

---

## Phase 1 — Container Format (`libaura::container`)

- [x] Define `AuraHeader` struct (magic bytes, version, section offsets)
- [x] Define `Record` trait with type tag, serialize, deserialize
- [x] Implement `LuminanceChromaRecord` (YCbCr pixel data)
- [x] Implement `SpatialDepthRecord` (Z-buffer + camera intrinsics)
- [x] Implement `SpatialAudioRecord` (ambisonic track bytes)
- [x] Implement `TemporalRecord` (frame timing + motion vectors)
- [x] Implement `SceneRecord` root container (Vec of child records)
- [x] Implement file writer: header → bootstrap → provenance → records → DAG → ledger → footer
- [x] Implement file reader: parse all sections into typed structs
- [x] Implement footer + CRC/checksum validation
- [x] Unit tests: round-trip write → read for each record type

---

## Phase 2 — Cryptographic Provenance (`libaura::provenance`)

- [x] Add `sha3` crate dependency
- [x] Add `ed25519-dalek` crate dependency
- [x] Implement `GenesisBlock` (data hash, hardware sig field, timestamp, device ID)
- [x] Implement `LedgerEntry` (op type, software, resulting hash, sig)
- [x] Implement `ProvenanceLedger` (append-only vec, serialization)
- [x] Implement `ledger.append(entry)` operation
- [x] Implement `ledger.verify()` chain validation
- [x] Implement trust seal break detection (bit-flip test)
- [x] Implement C2PA export helper
- [x] Unit tests: build ledger, append entries, verify, tamper + expect failure

---

## Phase 3 — WASM Bootstrap (`libaura::bootstrap`)

- [x] Embed a valid WASM decoder-key blob (`include` equivalent via `const`)
- [x] Implement `Bootstrap::write(writer)` — writes Wasm blob into file header
- [x] Implement `Bootstrap::read(reader)` — extracts and validates Wasm blob
- [x] Unit test: write bootstrap, read back, verify byte equality

---

## Phase 4 — Semantic DAG (`libaura::semantic`)

- [x] Define `ConceptNode` struct (ID, label, confidence, bitmask)
- [x] Define `ConceptEdge` struct (source, target, relationship)
- [x] Define `SemanticDAG` struct (nodes, edges)
- [x] Implement DAG cycle detection (Kahn's algorithm)
- [x] Implement bitmask RLE compression/decompression
- [x] Implement DAG serialization → binary section
- [x] Implement DAG deserialization from binary
- [x] `aura-onnx`: pure-Rust `StubDetector` → `SemanticDAG` (offline)
- [~] `aura-onnx`: YOLOv8 session + inference → `Vec<ConceptNode>` (scaffold behind `onnx` feature; needs model weights)
- [~] `aura-onnx`: SAM session + inference → per-node bitmasks (scaffold behind `onnx` feature; needs model weights)
- [~] `aura-onnx`: CLIP session + inference → label confidence scoring (scaffold behind `onnx` feature; needs model weights)
- [x] Unit tests: DAG round-trip, cycle detection, bitmask compress/decompress
- [~] Integration test: run ONNX inference on sample image, confirm non-empty DAG
      (covered offline by `StubDetector`; real ONNX run requires `--features aura-onnx/onnx` + weights)

---

## Phase 5 — Neural Encoding (`libaura::neural`)

- [x] Add `ort` (OnnxRuntime) dependency to `aura-onnx` (optional, feature-gated)
- [x] Implement Tier 0 encoder: downsample + lossless store → base layer bytes
- [x] Implement Tier 0 decoder: base layer bytes → low-res image
- [~] Download/bundle Real-ESRGAN ONNX model weights (requires manual download; see `models/` in .gitignore)
- [x] Implement Tier 1 payload write: `NeuralPayloadRecord` container
- [~] Implement Tier 1 inference: base layer + ONNX weights → full-res image (returns `Unsupported`; needs `onnx` feature + weights)
- [x] Expose `encode(image) → (base_bytes, payload_bytes)` in `libaura`
- [x] Expose `decode(base_bytes, payload_bytes) → image` in `libaura`
- [x] Unit test: encode → decode, verify image dimensions + PSNR == infinity (lossless Tier-0 store)

---

## Phase 6 — CLI (`aura-cli`)

- [x] Add `clap` dependency to `aura-cli`
- [x] Implement `aura create <input> -o <output.aura>`
- [x] Implement `aura inspect <file.aura>` (pretty-print all sections)
- [x] Implement `aura verify <file.aura>` (trust chain report)
- [x] Implement `aura sign <file.aura> --key <key-file>`
- [x] Implement `aura compile --target web` (→ PNG from Tier-0 base)
- [x] Implement `aura compile --target vr` (→ USDA stub)
- [x] Implement `aura compile --target print` (→ PNG; full-res notes Tier-1 ONNX)
- [x] Implement `aura compile --target legal` (→ JPEG + C2PA manifest)
- [x] End-to-end CLI integration test (create → inspect → verify → sign → compile)

---

## Phase 7 — Testing & Docs

- [x] Write rustdoc for all public API items in `libaura`
- [x] Write CLI help text and usage examples (clap + README)
- [x] Add CONTRIBUTING.md
- [x] Add GitHub issue templates (bug, feature)
- [x] Add benchmark suite (`criterion`) for encode/decode throughput
- [x] Verify `cargo test --workspace` all pass
- [x] Verify `cargo clippy --workspace -- -D warnings` clean
- [x] Verify `cargo fmt --check` clean

---

## Phase 8 — Open Source Release

- [x] Configure `crates.io` publish metadata in `Cargo.toml`
- [x] Add PR template
- [x] Write v0.1.0 release notes (`RELEASE_NOTES.md`)
- [ ] Initialize GitHub repo + push initial commit (manual: `git remote add` + `git push`)
- [ ] Set up branch protection on `main` (manual: GitHub settings)
- [ ] Verify GitHub Actions CI passes on first push (manual)
- [ ] Tag v0.1.0 (manual: `git tag v0.1.0 && git push --tags`)

---

Legend: `[x]` done · `[~]` scaffolded (needs model weights / network / manual step)
