# tpt-aura-cli

Command-line toolkit for the **AURA (Adaptive Universal Record Architecture)**
master media format. The binary is named **`aura`** and supports creating,
inspecting, verifying, signing, and compiling AURA files.

> Part of the [`tpt-aura` workspace](https://github.com/tpt-org/aura). This crate
> is the `tpt-aura` prefix crate family's command-line tool (formerly published as
> `tpt-aura-cli`). The core format logic lives in [`tpt-aura`](../tpt-aura) and the
> optional neural backends live in [`tpt-aura-onnx`](../tpt-aura-onnx).

## Installation

```toml
[dependencies]
tpt-aura-cli = "0.1"
```

Or build from source:

```sh
cargo build --workspace
# the binary lands at target/debug/aura
```

MSRV: Rust **1.74**.

## Commands

| Command | Description |
|---------|-------------|
| `aura create <input> -o <output.aura>` | Build an AURA file (Tier-0 base + optional Semantic DAG via `--detect`). |
| `aura inspect <file.aura>` | Pretty-print every section. |
| `aura verify <file.aura>` | Verify the cryptographic trust chain. |
| `aura sign <file.aura> --key <key> [-o <out.aura>]` | Append a signed ledger entry (e.g. to certify an export). |
| `aura compile <file.aura> --target <web\|vr\|print\|legal> -o <out>` | Compile the master file to a delivery target. |
| `aura fetch-models` | Download / scaffold the ONNX model weights used by `tpt-aura-onnx`. |
| `aura diff <a.aura> <b.aura>` | Show Structural-DAG, record-layout, and ledger changes (`--json` for machines). |

### Common flags

- `--key <path>` — device signing key (32 raw bytes). Generated and saved as
  `<output>.key` during `create` if omitted.
- `--scale <N>` — Tier-0 downsample factor (default `4`).
- `--detect` — run the semantic detector to embed a Semantic DAG.
- `--backend <stub|ort|coreml|tensorrt>` — detector backend for `--detect`
  (adaptive auto-select when omitted).

> **Note on `--target web`:** the reference compiler emits the Tier-0 base layer
> as a standard **PNG/JPEG** (format inferred from the output extension).
> AVIF/WebP delivery is planned; see RFC 001.

## Quick start

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

# Structural diff between two versions
./target/debug/aura diff photo.aura signed.aura
```

## Library usage

`tpt-aura-cli` is primarily a binary, but its command implementations are built
directly on the public APIs of `tpt-aura` and `tpt-aura-onnx`:

```rust,ignore
use tpt_aura::container::{open, AuraBuilder, SceneRecord};
use tpt_aura_onnx::default_detector;

let dag = default_detector().detect(&rgb_image)?;
let scene = SceneRecord::new();
let bytes = AuraBuilder::new(bootstrap, genesis, scene, dag, ledger).build()?;
let file = open(&bytes)?;
file.verify()?;
```

## Examples

`examples/quickstart.rs` is a runnable create → inspect → verify → sign → compile
demo that needs zero external downloads (it generates an in-memory image):

```sh
cargo run -p tpt-aura-cli --example quickstart
```

## License

Licensed under either of [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](https://opensource.org/licenses/MIT) at your option.
