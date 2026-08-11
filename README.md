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
> [RFC 001 AURA specification](./docs/SPEC.md) (also available as plain text in
> [`spec.txt`](./spec.txt)).

## Workspace layout

| Crate | Description |
|-------|-------------|
| `tpt-aura` | Core library: container, provenance, bootstrap, semantic DAG, neural encoding. |
| `tpt-aura-cli` | Command-line tool (`aura create/inspect/verify/sign/compile`). |
| `tpt-aura-onnx` | Optional ONNX inference (YOLOv8 / SAM / CLIP) for populating the Semantic DAG. |

## Building

```sh
cargo build --workspace
```

The `tpt-aura-onnx` crate's real ONNX backend is gated behind the `onnx` feature
(requires network access at build time to fetch ONNX Runtime). By default a
pure-Rust stub detector is used so the workspace builds and tests offline.

```sh
cargo build --workspace --features tpt-aura-onnx/onnx
```

## Quick start

> The CLI binary is named **`aura`** (the crate is `tpt-aura-cli`). Build it with
> `cargo build --workspace`; the binary lands at `target/debug/aura`.

### 5-minute quickstart

```sh
# Build the CLI
cargo build --workspace

# Create an AURA file from an image (a device key is generated automatically)
./target/debug/aura create photo.png -o photo.aura

# Pretty-print every section
./target/debug/aura inspect photo.aura

# Verify the cryptographic trust chain
./target/debug/aura verify photo.aura

# Sign the ledger with the device key produced during `create`
./target/debug/aura sign photo.aura --key photo.aura.key -o signed.aura

# Compile to a delivery target (Tier-0 base layer; PNG is the web default)
./target/debug/aura compile signed.aura --target web -o photo.png
```

### Available commands

| Command | Description |
|---------|-------------|
| `aura create <input> -o <output.aura>` | Build an AURA file (Tier-0 base + optional Semantic DAG via `--detect`). |
| `aura inspect <file.aura>` | Pretty-print every section. |
| `aura verify <file.aura>` | Verify the cryptographic trust chain. |
| `aura sign <file.aura> --key <key>` | Append a signed ledger entry. |
| `aura compile <file.aura> --target <web|vr|print|legal> -o <out>` | Compile to a delivery target. |

> **Note on `--target web`:** the reference compiler emits the Tier-0 base
> layer as a standard **PNG/JPEG** (format inferred from the output extension).
> AVIF/WebP delivery is planned; see the RFC 001 "Compiler" model in
> [`docs/SPEC.md`](./docs/SPEC.md).

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
