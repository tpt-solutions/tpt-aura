# Changelog

All notable changes to the `tpt-aura-onnx` crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Pluggable detector backends: a `Backend` enum, `detector_for()`, and an adaptive
  `default_detector()` that picks the best compiled-in runtime.
- `coreml` / `tensorrt` feature gates scaffold those backends (returning
  `Unsupported` until the native SDK bindings land); `tpt-aura-cli create` gains a
  `--backend <stub|ort|coreml|tensorrt>` flag.

### Changed
- Crate renamed from `aura-onnx` to `tpt-aura-onnx` so every crate in the workspace
  shares the `tpt-aura` prefix. The crate id is now `tpt_aura_onnx`.

## [0.1.0] - 2026-08-11

Initial open-source release of the **AURA** ONNX inference backends.

### Added
- `Detector` trait that turns an image into a `tpt-aura::semantic::SemanticDAG`.
- `StubDetector`: a pure-Rust reference detector (Sky/Ground quadrant masks) that
  builds a plausible, non-empty DAG so the workspace builds and tests fully offline.
- Optional ONNX Runtime backend (`ort`) behind the `onnx` feature, with an
  `OrtDetector` that loads YOLOv8 (+ optional CLIP) sessions and merges results
  into a `SemanticDAG`.

### Known limitations (scaffolding)
- The `onnx` backend requires network access at build time (to fetch ONNX Runtime)
  plus model weight files (under `models/`); inference is wired but weights must be
  supplied.
- `coreml` and `tensorrt` feature gates compile and are selectable, but the native
  SDK bindings are scaffolded and return `Unsupported` until integrated.

[Unreleased]: https://github.com/tpt-org/aura/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/tpt-org/aura/releases/tag/v0.1.0
