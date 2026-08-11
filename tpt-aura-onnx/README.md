# tpt-aura-onnx

ONNX inference backends that populate the **AURA Semantic DAG** — the
semantically-aware layer of the AURA master media format.

> Part of the [`tpt-aura` workspace](https://github.com/tpt-org/aura). This crate
> is the `tpt-aura` prefix crate family's neural backend (formerly published as
> `tpt-aura-onnx`). It depends on [`tpt-aura`](../tpt-aura) and is consumed by
> [`tpt-aura-cli`](../tpt-aura-cli).

## What it does

A detector turns an image into a `tpt_aura::semantic::SemanticDAG`:

```rust,ignore
use tpt_aura_onnx::default_detector;

let detector = default_detector();
let dag = detector.detect(&rgb_image)?; // -> SemanticDAG
```

### Detector trait

```rust
pub trait Detector {
    fn name(&self) -> &str;
    fn detect(&self, img: &RgbImage) -> Result<SemanticDAG, AuraError>;
}
```

## Backends

| Backend | Feature | Status |
|---------|---------|--------|
| `Stub`  | (always) | Pure-Rust reference detector. Builds a plausible Sky/Ground DAG, no weights needed. |
| `Ort`   | `onnx`   | ONNX Runtime (YOLOv8 detection + optional CLIP). Requires fetched runtime + weights. |
| `CoreMl`| `coreml` | Scaffold — wire up a `coreml-rs` binding to enable. |
| `TensorRt` | `tensorrt` | Scaffold — wire up a TensorRT binding to enable. |

The adaptive `default_detector()` picks the best *compiled-in* backend
(TensorRT → CoreML → ONNX → Stub) and silently falls back to the offline
`StubDetector` when no native runtime is available.

```rust,ignore
use tpt_aura_onnx::{detector_for, Backend};

let detector = detector_for(Backend::Ort)?; // Err(Unsupported) if feature off / weights missing
```

## Installation

```toml
[dependencies]
tpt-aura-onnx = { version = "0.1", default-features = false }
# enable a real backend:
tpt-aura-onnx = { version = "0.1", features = ["onnx"] }
```

MSRV: Rust **1.74**.

## Building

The default build uses the pure-Rust `StubDetector` so the workspace builds and
tests **fully offline**. The real ONNX backend is gated behind the `onnx` feature,
which fetches the ONNX Runtime prebuilt at build time and needs model weights:

```sh
cargo build --workspace --features tpt-aura-onnx/onnx
```

Model weights (YOLOv8 + SAM + CLIP) are fetched via `aura fetch-models` from the
CLI, or supplied under `models/` (see `.gitignore`).

## Feature flags

- `default` — empty (offline `StubDetector` only).
- `onnx` — enable the ONNX Runtime backend via `ort`.
- `coreml` — scaffold the Apple CoreML backend.
- `tensorrt` — scaffold the NVIDIA TensorRT backend.

## Known limitations (scaffolding)

- The `onnx` backend loads YOLOv8 (+ optional CLIP) sessions; SAM-based per-concept
  masks and full inference wiring require model weights and a runtime environment.
- `coreml` / `tensorrt` compile and are selectable but return `Unsupported` until
  the native SDK bindings are integrated.

## License

Licensed under either of [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](https://opensource.org/licenses/MIT) at your option.
