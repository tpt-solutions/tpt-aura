# AURA — Adaptive Universal Record Architecture

AURA is a polymorphic, semantically-aware, and cryptographically sealed
**master media format**. Unlike delivery formats (JPEG, MP4, AVIF) designed
for efficient pixel delivery, AURA is the *canonical source of truth* for
digital reality capture. It natively stores:

- **2D / 3D spatial data** via a hierarchical, typed *Record* container.
- **A Semantic DAG** of concept nodes, edges, and per-concept pixel bitmasks.
- **A C2PA-style provenance ledger** baked into the file (genesis hash +
  append-only, signed edit history).
- **A WASM bootstrap** in the header that self-describes the decoding math,
  so the file remains readable even if the standard disappears.
- **Adaptive neural encoding** (Tier 0 base layer + Tier 1 neural payload)
  for compute-scalable fidelity.

AURA is explicitly **not** for edge delivery. It is the master format from
which all delivery formats are *compiled* (web, VR/USD, print/TIFF, legal/JPEG).

> This is an open-source reference implementation of the
> [RFC 001 AURA specification](./spec.txt).

## Workspace layout

| Crate | Description |
|-------|-------------|
| `libaura` | Core library: container, provenance, bootstrap, semantic DAG, neural encoding. |
| `aura-cli` | Command-line tool (`aura create/inspect/verify/sign/compile`). |
| `aura-onnx` | Optional ONNX inference (YOLOv8 / SAM / CLIP) for populating the Semantic DAG. |

## Building

```sh
cargo build --workspace
```

The `aura-onnx` crate's real ONNX backend is gated behind the `onnx` feature
(requires network access at build time to fetch ONNX Runtime). By default a
pure-Rust stub detector is used so the workspace builds and tests offline.

```sh
cargo build --workspace --features aura-onnx/onnx
```

## Quick start

```sh
# Create an AURA file from an image
aura-cli create photo.png -o photo.aura

# Pretty-print every section
aura-cli inspect photo.aura

# Verify the cryptographic trust chain
aura-cli verify photo.aura

# Sign the ledger with a device key
aura-cli sign photo.aura --key device.key

# Compile to a delivery target
aura-cli compile photo.aura --target web -o photo.avif
```

## Testing

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or
[MIT license](./LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
