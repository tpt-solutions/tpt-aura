# RFC 001: AURA (Adaptive Universal Record Architecture)

> Rendered Markdown version of [`../spec.txt`](../spec.txt).
> Status: Conceptual Draft / North Star Specification.
> Version: 1.0.0.
> Domain: Digital Media, Spatial Computing, Cryptographic Provenance.

## 1. Abstract

AURA (Adaptive Universal Record Architecture) is a polymorphic, semantically-aware,
and cryptographically sealed master media format. Unlike legacy formats designed
solely for efficient pixel delivery (e.g., JPEG, MP4), AURA is designed to be the
Canonical Source of Truth for digital reality capture. It natively stores 2D/3D
spatial data, semantic conceptual graphs, and cryptographic provenance, utilizing
adaptive neural encoding to balance compute efficiency with absolute fidelity.
AURA is not intended for edge delivery; it is the master format from which all
delivery formats are compiled.

## 2. Motivation & Core Problems

Current media infrastructure suffers from four fatal flaws:

- **Dimensional Fragmentation:** Reality is captured in 2D (JPEG), 3D (USD/OBJ),
  and time (MP4) using incompatible formats.
- **Loss of Context:** Formats store "dumb" pixels. Editing requires destructive
  pixel manipulation rather than conceptual manipulation.
- **The Trust Crisis:** Metadata (EXIF) is easily stripped or forged, enabling
  deepfakes and synthetic media to bypass verification.
- **The Compute/Battery Wall:** High-efficiency formats (AVIF/HEIC) require heavy
  decoding math, draining mobile batteries, while low-compute formats sacrifice
  quality.

## 3. Core Architecture: The Polymorphic Container

AURA abandons the concept of a "flat image" or "linear video." It utilizes a
hierarchical, chunk-based container (similar to MP4/RIFF, but typed for reality)
called **Records**.

### 3.1 Record Types

A single AURA file contains a root `SCENE_RECORD` which can encapsulate any
combination of the following child records:

- `LUMINANCE_CHROMA_RECORD`: Standard 2D color data (YCbCr or RGB).
- `SPATIAL_DEPTH_RECORD`: LiDAR/Depth map data (Z-buffer) with camera intrinsics.
- `SPATIAL_AUDIO_RECORD`: Ambisonic or object-based 3D audio tracks.
- `TEMPORAL_RECORD`: Frame-timing and delta-motion vectors (if capturing video).

**Design Rule:** A viewer application only parses the Records it supports. A
smartwatch reads only `LUMINANCE_CHROMA`; a VR headset reads all four.

## 4. The Semantic & Conceptual Layer

Instead of relying on external sidecar files for metadata, AURA embeds a Semantic
Directed Acyclic Graph (DAG) directly into the file structure.

### 4.1 Concept Nodes and Masks

When the image is captured (or processed), an on-device neural engine generates a
Semantic DAG.

- **Nodes:** Represent concepts (e.g., `Node_ID: 0x01, Label: "Sky", Confidence: 0.98`).
- **Masks:** Each node contains a compressed bitmask mapping exactly to the
  pixels/voxels in the `LUMINANCE` or `SPATIAL` records that belong to that concept.
- **Edges:** Define spatial relationships (e.g., `Node: Person -> is_in_front_of -> Node: Car`).

**Use Case:** An editor can select the "Sky" node and apply a color grade. The AURA
engine uses the bitmask to isolate the pixels non-destructively, without the user
ever drawing a mask.

## 5. Adaptive Compute Scaling (Neural Progressive Encoding)

To solve the battery/compute trade-off, AURA utilizes a dual-layer encoding system.

### 5.1 Tier 0: The Base Layer (Zero-Compute)

The file contains a heavily downsampled, traditionally compressed (e.g., Wavelet or
DCT) base layer.

**Purpose:** Instant, hardware-accelerated thumbnail generation and viewing on
low-power devices (IoT, old phones) with near-zero battery drain.

### 5.2 Tier 1: The Neural Enhancement Layer

Instead of storing massive high-res pixel data, AURA stores a Neural Reconstruction
Payload (typically 50KB - 200KB). This payload consists of the specific weights for
a lightweight Super-Resolution/Reconstruction neural network.

**Purpose:** When opened on a device with an NPU (Neural Processing Unit), the NPU
takes the Tier 0 Base Layer and runs it through the embedded Neural Payload to
reconstruct the image to absolute, mathematically perfect master resolution.

**Result:** File size remains tiny, but visual quality scales infinitely with the
hardware viewing it.

## 6. Cryptographic Provenance (The Trust Chain)

AURA natively implements a C2PA-style immutable ledger, but baked into the core file
architecture rather than bolted on as metadata.

### 6.1 The Provenance Chain

- **Genesis Hash:** At the moment of sensor readout, the raw bayer data is hashed
  (using SHA-3 or Post-Quantum Cryptography). This hash is signed by the camera
  hardware's secure enclave.
- **Append-Only Ledger:** Every subsequent edit (semantic adjustment, color grade,
  crop) generates a new block containing the operation performed, the software used,
  and the new hash of the resulting state.
- **Verification:** Any viewer can mathematically verify the exact lineage of the
  file from the physical sensor to the current state. If a single bit is altered
  outside the ledger, the file's trust seal breaks.

## 7. "Forever" Compatibility (Self-Describing Math)

To prevent format obsolescence, AURA includes a WASM Bootstrap in its header.

### 7.1 The Decoder Key

The first 150KB of the file contains a compiled WebAssembly (Wasm) binary. This
binary contains the exact mathematical primitives and decoding algorithms required
to read the specific proprietary or experimental compression math used in the rest
of the file.

**Result:** If the AURA standard dies, or if a future computer uses a completely
different architecture, the OS simply reads the Wasm Bootstrap, compiles it on the
fly, and gains the ability to decode the file natively.

## 8. Conceptual Byte Layout

```text
[ 0x0000 ] MAGIC_BYTES ("AURA") & VERSION
[ 0x0010 ] WASM_BOOTSTRAP (Self-describing decoder key)
[ 0x4000 ] PROVENANCE_GENESIS (Sensor hash & hardware signature)
[ 0x4100 ] POLYMORPHIC_RECORDS_START
           ├── [Tier 0 Base] LUMINANCE_CHROMA (Low-res, standard math)
           ├── [Tier 0 Base] SPATIAL_DEPTH (Low-res)
           ├── [Tier 1 Neural] RECONSTRUCTION_PAYLOAD (NPU Weights)
           └── [Tier 1 Neural] SPATIAL_AUDIO (High-res)
[ 0x8000 ] SEMANTIC_DAG (Nodes, relationships, and pixel bitmasks)
[ 0xA000 ] PROVENANCE_LEDGER (Append-only edit history & hashes)
[ 0xBFFF ] FOOTER & CHECKSUMS
```

## 9. Ecosystem Integration: The "Compiler" Model

AURA is explicitly not designed to be sent over the wire to a web browser or texted
to a friend. It is the Master Format. The ecosystem relies on an `aura-compile`
pipeline. When a user exports or shares an AURA file, the OS or server dynamically
compiles it into the optimal delivery format:

- **Target: Web Browser / Social Media**
  - Action: Strip Semantic DAG and Provenance. Compile Tier 0 Base Layer into AVIF
    or WebP.
- **Target: VR / Spatial Computing Headset**
  - Action: Compile all Polymorphic Records and Semantic DAG into USD (Universal
    Scene Description).
- **Target: Professional Print / Archival**
  - Action: Run Tier 1 Neural Payload through NPU to generate full-res pixels.
    Compile into TIFF or EXR.
- **Target: Journalism / Legal Evidence**
  - Action: Compile into standard JPEG, but inject the Provenance Ledger as a C2PA
    Content Credential.

## 10. Limitations and Trade-offs

- **Storage Overhead:** The inclusion of the WASM Bootstrap, Semantic DAG, and
  Provenance Ledger adds roughly 200KB - 500KB of overhead per file. This is
  unacceptable for edge delivery, reinforcing its role strictly as a Master format.
- **Capture Compute:** Generating the Semantic DAG and Neural Payload at the exact
  moment of capture requires significant ISP/NPU overhead on the camera device.
- **Ecosystem Friction:** Requires universal agreement on the Semantic DAG taxonomy
  (what constitutes a "sky" or a "person" across different AI models) to ensure
  interoperability.
