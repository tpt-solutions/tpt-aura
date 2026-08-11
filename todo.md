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

## Phase 9 — Platform Review Follow-ups (2026-08-11)

> From a full-codebase review covering bugs, docs/adoption gaps, and innovation ideas.

### Bugs

- [x] `libaura::container::open()`: section-table loop panicked (slice-index-out-of-bounds) on a truncated `.aura` file with an inflated `count`; now bounds-checked and returns `AuraError::UnexpectedEof` (regression test `truncated_section_table_errors_instead_of_panicking` added)
- [x] README Quick Start invoked `aura-cli create/inspect/...`, but the matching binary is named `aura` (`aura-cli/Cargo.toml` `[[bin]] name = "aura"`); README now uses `aura` and documents the binary location explicitly.
- [x] README's `compile --target web` example claimed `.avif` output, but `cmd_compile`'s web branch only encodes PNG/JPEG via the `image` crate — no AVIF encoder wired up; README now documents the PNG/JPEG default (AVIF/WebP delivery noted as future work in `docs/SPEC.md`).

### Missing infrastructure

- [x] Add `CHANGELOG.md` (currently only a single-entry `RELEASE_NOTES.md`), ideally generated via `git-cliff`/`release-plz` — added a Keep-a-Changelog style `CHANGELOG.md` with `Unreleased` + `0.1.0` sections.
- [x] Add `docs/` folder or render `spec.txt` as `SPEC.md`/mdBook so RFC 001 is browsable on GitHub — added `docs/SPEC.md` (rendered Markdown) and linked it from the README.
- [x] Add `examples/` directory with a runnable quickstart (create → inspect → verify → sign → compile), mirroring `aura-cli/tests/e2e.rs` — added `aura-cli/examples/quickstart.rs` (in-process, mirrors the e2e pipeline).
- [x] Bundle 1-2 tiny sample images so `cargo run --example quickstart` works with zero external downloads — the quickstart generates its source image in-memory (zero downloads), and `examples/assets/sample.png` is bundled as a reference fixture.
- [x] Add `CODEOWNERS`
- [x] Add `SECURITY.md`
- [x] Add `dependabot.yml` (or Renovate config)
- [x] Add a release/publish GitHub Actions workflow — added `.github/workflows/release.yml` (verify → publish to crates.io → GitHub release).
- [x] Expand CI to a matrix: test declared MSRV (1.74) and add Windows/macOS runners (currently `ubuntu-latest` + stable only) — `ci.yml` now runs a 3-OS matrix plus an MSRV job and an examples/quickstart job.
- [x] Audit `serde`/`bincode` workspace dependencies — declared but not referenced in any of the three crates; remove if unused — both removed from `[workspace.dependencies]` (only transitive deps remain in `Cargo.lock`).

### Usability / automation

- [x] Add a model-weights bootstrap command/script (e.g. `aura fetch-models`) — added `aura fetch-models` (downloads the canonical Ultralytics YOLOv8 `.pt` checkpoints into `models/` and writes an export-to-ONNX `README.md`; supports `--dir`/`--manifest`/`--force`).
- [x] Add a "5-minute quickstart" section to README using the corrected binary name

### Innovation ideas (not yet scoped)

- [x] Browser-based WASM viewer/demo proving the embedded "self-decoding" bootstrap live — added `web/` (zero-dependency `index.html` + `main.js`) that parses an `.aura` file in JS, renders the Tier-0 base image, lists the Semantic DAG/ledger, and instantiates the embedded WASM bootstrap live in-browser. Verified the JS parser against `examples/assets/sample.aura` (matches `libaura::container::open`: CRC-32, sections, DAG, ledger).
- [x] GitHub Action that verifies `.aura` provenance chains on PRs (dogfood C2PA-style trust on the project's own media assets) — added `.github/actions/verify-aura/action.yml` (builds `aura`, runs `aura verify` on every `.aura` file) and a `verify-aura` CI job over `examples/assets`. Committed a signed `examples/assets/sample.aura` as the dogfood asset.
- [x] `aura diff` command to visualize semantic-DAG changes between two file versions — added `libaura::diff` (DAG/record/ledger diff) with a `Display` report and `--json` output, wired to `aura diff <a> <b>` (with unit tests).
- [x] Pluggable detector backends beyond ONNX (e.g. CoreML/TensorRT feature flags) matching the "adaptive" framing — added `Backend` enum + `detector_for()` + adaptive `default_detector()` in `aura-onnx`, plus `coreml`/`tensorrt` feature-gated scaffold backends and `aura create --backend <stub|ort|coreml|tensorrt>`.

---

Legend: `[x]` done · `[~]` scaffolded (needs model weights / network / manual step)
