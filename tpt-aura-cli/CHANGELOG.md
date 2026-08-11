# Changelog

All notable changes to the `tpt-aura-cli` crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `aura diff <a.aura> <b.aura>`: visualize Semantic-DAG, record-layout, and
  provenance-ledger changes between two file versions, with a `--json`
  machine-readable mode (built on `tpt-aura::diff`).
- `aura create --backend <stub|ort|coreml|tensorrt>`: select the detector backend
  used to populate the Semantic DAG (adaptive auto-select when omitted).
- `aura fetch-models`: download / scaffold the ONNX model weights used by the
  `tpt-aura-onnx` backend.

### Changed
- Crate renamed from `aura-cli` to `tpt-aura-cli` so every crate in the workspace
  shares the `tpt-aura` prefix. The produced binary is still named `aura`.

## [0.1.0] - 2026-08-11

Initial open-source release of the **AURA** command-line toolkit.

### Added
- `aura create <input> -o <output.aura>`: build an AURA file (Tier-0 base +
  optional Semantic DAG via `--detect`). A device key is generated automatically
  and saved as `<output>.key` unless one is supplied with `--key`.
- `aura inspect <file.aura>`: pretty-print every section.
- `aura verify <file.aura>`: verify the cryptographic trust chain.
- `aura sign <file.aura> --key <key> [-o <out.aura>]`: append a signed ledger entry.
- `aura compile <file.aura> --target <web|vr|print|legal> -o <out>`: compile the
  master file to a delivery target.
- End-to-end integration test (`tests/e2e.rs`) covering the full
  create → inspect → verify → sign → compile pipeline.

[Unreleased]: https://github.com/tpt-org/aura/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/tpt-org/aura/releases/tag/v0.1.0
