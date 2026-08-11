# AURA browser viewer

A dependency-free, self-contained demo that proves the AURA **self-decoding
WASM bootstrap** live in a browser. It is pure HTML + JavaScript (no build
step, no frameworks) and parses the AURA container format directly, mirroring
`libaura::container::open`.

## What it shows

- Parses the AURA header + section table in JavaScript.
- Validates the file's **CRC-32** integrity (matches `codec::crc32`).
- Renders the **Tier-0 base layer** to a canvas — decoded from the file itself.
- Lists the **Semantic DAG** (concept nodes + edges) embedded in the file.
- Lists the **provenance ledger** entries.
- **Instantiates the embedded WASM bootstrap** with `WebAssembly.instantiate`,
  proving the file is genuinely self-describing: a runtime can compile the
  embedded decoder key on the fly, even with no AURA library installed.

## Running it

The page fetches the bundled `../examples/assets/sample.aura`, so it must be
served over HTTP (browsers block `fetch` of `file://`). From the repository
root:

```sh
python3 -m http.server 8000
# then open http://localhost:8000/web/
```

You can also use **Choose .aura file** to load any AURA master file from disk.

## Layout

| File | Purpose |
|------|---------|
| `index.html` | UI shell + styling. |
| `main.js` | AURA parser, canvas renderer, and live WASM bootstrap instantiation. |

> The embedded WASM is currently the placeholder decoder key from
> `libaura::bootstrap` (a valid `add` module). A production build would embed the
> real AURA decoding primitives, which this viewer would then run to decode the
> container instead of the hand-written JS fallback.
