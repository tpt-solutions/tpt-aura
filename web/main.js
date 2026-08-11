// AURA browser viewer — proves the embedded "self-decoding" WASM bootstrap live.
//
// It parses the AURA container in pure JavaScript (port of tpt-aura's `open`),
// renders the Tier-0 base layer, lists the Semantic DAG, and — crucially —
// instantiates the embedded WASM bootstrap in the browser to show the file is
// genuinely self-describing: a runtime can compile the embedded decoder key on
// the fly, even with no AURA library installed.

const AURA_MAGIC = [0x41, 0x55, 0x52, 0x41]; // "AURA"
const SECTION = { BOOTSTRAP: 1, GENESIS: 2, RECORDS: 3, SEMANTIC: 4, LEDGER: 5 };
const REC = {
  LUMINANCE_CHROMA: 0x10,
  SPATIAL_DEPTH: 0x11,
  SPATIAL_AUDIO: 0x12,
  TEMPORAL: 0x13,
  NEURAL_PAYLOAD: 0x14,
};
const FOOTER_SIZE = 4 + 32; // CRC-32 + SHA-3-256

// Little-endian reader over a Uint8Array (matches tpt-aura::codec::Reader).
class Reader {
  constructor(buf) {
    this.buf = buf;
    this.dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
    this.pos = 0;
  }
  u8() {
    return this.dv.getUint8(this.pos++);
  }
  u16() {
    const v = this.dv.getUint16(this.pos, true);
    this.pos += 2;
    return v;
  }
  u32() {
    const v = this.dv.getUint32(this.pos, true);
    this.pos += 4;
    return v;
  }
  f32() {
    const v = this.dv.getFloat32(this.pos, true);
    this.pos += 4;
    return v;
  }
  bytes() {
    const n = this.u32();
    const s = this.buf.subarray(this.pos, this.pos + n);
    this.pos += n;
    return s;
  }
  str() {
    return new TextDecoder().decode(this.bytes());
  }
  skip(n) {
    this.pos += n;
  }
}

// IEEE 802.3 CRC-32 (polynomial 0xEDB88320, reflected) — mirrors codec::crc32.
function crc32(bytes) {
  let crc = 0xffffffff;
  for (let i = 0; i < bytes.length; i++) {
    crc ^= bytes[i];
    for (let k = 0; k < 8; k++) {
      crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function hex(bytes) {
  let s = "";
  for (let i = 0; i < bytes.length; i++) s += bytes[i].toString(16).padStart(2, "0");
  return s;
}

function parseAura(bytes) {
  if (bytes.length < 10 + FOOTER_SIZE) throw new Error("file too small");
  for (let i = 0; i < 4; i++) {
    if (bytes[i] !== AURA_MAGIC[i]) throw new Error("bad magic — not an AURA file");
  }
  const version_major = (bytes[4] | (bytes[5] << 8)) >>> 0;
  const version_minor = (bytes[6] | (bytes[7] << 8)) >>> 0;
  const count = bytes[8] | (bytes[9] << 8);
  const fv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const sections = [];
  let pos = 10;
  for (let i = 0; i < count; i++) {
    const type = bytes[pos++];
    const offset = Number(fv.getBigUint64(pos, true));
    pos += 8;
    const length = Number(fv.getBigUint64(pos, true));
    pos += 8;
    sections.push({ type, offset, length });
  }

  const footerStart = bytes.length - FOOTER_SIZE;
  const crcStored = fv.getUint32(footerStart, true);
  const crcComputed = crc32(bytes.subarray(0, footerStart));
  const crcOk = crcStored === crcComputed;

  const byType = (t) => {
    const e = sections.find((s) => s.type === t);
    if (!e) return null;
    return bytes.subarray(e.offset, e.offset + e.length);
  };

  let bootstrap = null;
  const boot = byType(SECTION.BOOTSTRAP);
  if (boot) {
    const r = new Reader(boot);
    const len = r.u32();
    bootstrap = { wasm: boot.subarray(r.pos, r.pos + len) };
  }

  let image = null;
  const rec = byType(SECTION.RECORDS);
  if (rec) {
    const r = new Reader(rec);
    const n = r.u32();
    for (let i = 0; i < n; i++) {
      const tag = r.u8();
      if (tag === REC.LUMINANCE_CHROMA) {
        const w = r.u32();
        const h = r.u32();
        r.u8(); // depth
        r.u8(); // sampling
        const data = r.bytes();
        image = { w, h, data };
      } else {
        break; // unknown record layout — stop parsing children
      }
    }
  }

  let dag = { nodes: [], edges: [] };
  const sem = byType(SECTION.SEMANTIC);
  if (sem) {
    const r = new Reader(sem);
    const nn = r.u32();
    for (let i = 0; i < nn; i++) {
      const id = r.u32();
      const label = r.str();
      const conf = r.f32();
      r.bytes(); // bitmask_rle (skipped here)
      dag.nodes.push({ id, label, conf });
    }
    const ne = r.u32();
    for (let i = 0; i < ne; i++) {
      const src = r.u32();
      const tgt = r.u32();
      const rel = r.str();
      dag.edges.push({ src, tgt, rel });
    }
  }

  let ledger = { count: 0, ops: [] };
  const led = byType(SECTION.LEDGER);
  if (led) {
    const r = new Reader(led);
    r.skip(32 + 32 + 32); // signer, root_hash, current_hash
    const n = r.u32();
    ledger.count = n;
    for (let i = 0; i < n; i++) {
      const op = r.u8();
      const sw = r.str();
      r.skip(32 + 32 + 64); // prev_hash, resulting_hash, signature
      ledger.ops.push({ op, sw });
    }
  }

  return {
    version_major,
    version_minor,
    crcOk,
    sections,
    bootstrap,
    image,
    dag,
    ledger,
  };
}

function renderImage(canvas, image) {
  canvas.width = image.w;
  canvas.height = image.h;
  const ctx = canvas.getContext("2d");
  const out = ctx.createImageData(image.w, image.h);
  for (let i = 0; i < image.w * image.h; i++) {
    out.data[i * 4 + 0] = image.data[i * 3 + 0];
    out.data[i * 4 + 1] = image.data[i * 3 + 1];
    out.data[i * 4 + 2] = image.data[i * 3 + 2];
    out.data[i * 4 + 3] = 255;
  }
  ctx.putImageData(out, 0, 0);
}

async function run(bytes, sourceLabel) {
  const out = document.getElementById("output");
  out.textContent = "";
  const log = (s) => {
    const pre = document.createElement("div");
    pre.className = "line";
    pre.textContent = s;
    out.appendChild(pre);
  };
  try {
    const f = parseAura(bytes);
    log(`Loaded ${sourceLabel}`);
    log(`AURA v${f.version_major}.${f.version_minor}`);
    log(`CRC-32 integrity: ${f.crcOk ? "OK" : "MISMATCH"}`);
    log(`Sections: ${f.sections.map((s) => s.type).join(", ")}`);

    if (f.bootstrap) {
      const isWasm =
        f.bootstrap.wasm[0] === 0x00 &&
        f.bootstrap.wasm[1] === 0x61 &&
        f.bootstrap.wasm[2] === 0x73 &&
        f.bootstrap.wasm[3] === 0x6d;
      log(`Bootstrap: ${f.bootstrap.wasm.length} bytes, WASM magic ${isWasm ? "valid" : "INVALID"}`);
      if (isWasm) {
        try {
          const { instance } = await WebAssembly.instantiate(f.bootstrap.wasm);
          const exports = Object.keys(instance.exports);
          log(`Bootstrap instantiated live in-browser ✓ exports: ${exports.join(", ")}`);
        } catch (e) {
          log(`Bootstrap failed to instantiate: ${e.message}`);
        }
      }
    }

    const canvas = document.getElementById("preview");
    if (f.image) {
      renderImage(canvas, f.image);
      log(`Tier-0 base layer rendered: ${f.image.w}x${f.image.h} (decoded from the file itself)`);
    } else {
      log("No LUMINANCE_CHROMA record to render.");
    }

    log(`Semantic DAG: ${f.dag.nodes.length} nodes, ${f.dag.edges.length} edges`);
    for (const n of f.dag.nodes) log(`  • node ${n.id} "${n.label}" (conf ${n.conf.toFixed(2)})`);
    for (const e of f.dag.edges) log(`  • edge ${e.src} →(${e.rel})→ ${e.tgt}`);

    log(`Provenance ledger: ${f.ledger.count} entries`);
    for (const e of f.ledger.ops) log(`  • op#${e.op} by ${e.sw}`);
  } catch (err) {
    log(`Error: ${err.message}`);
  }
}

function handleFile(file) {
  const reader = new FileReader();
  reader.onload = () => run(new Uint8Array(reader.result), file.name);
  reader.readAsArrayBuffer(file);
}

window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("file").addEventListener("change", (e) => {
    if (e.target.files[0]) handleFile(e.target.files[0]);
  });
  document.getElementById("sample").addEventListener("click", async () => {
    try {
      const res = await fetch("../examples/assets/sample.aura");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const buf = await res.arrayBuffer();
      run(new Uint8Array(buf), "examples/assets/sample.aura");
    } catch (e) {
      document.getElementById("output").textContent =
        `Could not load sample (serve this page over http, e.g. \`python -m http.server\`): ${e.message}`;
    }
  });
});
