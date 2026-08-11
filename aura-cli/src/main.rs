//! `aura` — command-line tool for the AURA master media format.
//!
//! ```text
//! aura create  <input> -o <output.aura> [--key key] [--scale N] [--detect]
//! aura inspect <file.aura>
//! aura verify  <file.aura>
//! aura sign    <file.aura> --key <key> [-o <out.aura>]
//! aura compile <file.aura> --target <web|vr|print|legal> -o <out>
//! ```

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use libaura::bootstrap::Bootstrap;
use libaura::container::{
    open, AuraBuilder, AuraFile, LuminanceChromaRecord, REC_LUMINANCE_CHROMA,
};
use libaura::neural::{encode_tier0, RgbImage};
use libaura::provenance::{sha3_256, GenesisBlock, OpType, ProvenanceLedger};
use libaura::semantic::SemanticDAG;
use rand::RngCore;

type Err = Box<dyn std::error::Error>;

#[derive(Parser)]
#[command(name = "aura", version, about = "AURA master media format toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create an AURA file from an image (Tier-0 base layer + optional DAG).
    Create(CreateArgs),
    /// Pretty-print every section of an AURA file.
    Inspect(InspectArgs),
    /// Verify the cryptographic trust chain.
    Verify(VerifyArgs),
    /// Append a signed ledger entry (e.g. to certify an export).
    Sign(SignArgs),
    /// Compile an AURA master file to a delivery target.
    Compile(CompileArgs),
    /// Download / scaffold the ONNX model weights used by the `aura-onnx` backend.
    FetchModels(FetchModelsArgs),
    /// Show structural changes between two AURA files (DAG, records, ledger).
    Diff(DiffArgs),
}

#[derive(Parser)]
struct CreateArgs {
    /// Input image (PNG/JPEG/...).
    input: PathBuf,
    /// Output `.aura` file.
    #[arg(short, long)]
    output: PathBuf,
    /// Device signing key (32 raw bytes). Generated and saved as `<output>.key` if omitted.
    #[arg(long)]
    key: Option<PathBuf>,
    /// Tier-0 downsample factor.
    #[arg(long, default_value_t = 4)]
    scale: u32,
    /// Run the semantic detector to embed a Semantic DAG.
    #[arg(long)]
    detect: bool,
    /// Detector backend to use with `--detect` (default: adaptive auto-select).
    #[arg(long)]
    backend: Option<BackendArg>,
}

/// CLI-facing detector backend selector (maps to `aura_onnx::Backend`).
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum BackendArg {
    Stub,
    Ort,
    CoreMl,
    TensorRt,
}

fn to_aura_backend(b: BackendArg) -> aura_onnx::Backend {
    match b {
        BackendArg::Stub => aura_onnx::Backend::Stub,
        BackendArg::Ort => aura_onnx::Backend::Ort,
        BackendArg::CoreMl => aura_onnx::Backend::CoreMl,
        BackendArg::TensorRt => aura_onnx::Backend::TensorRt,
    }
}

#[derive(Parser)]
struct InspectArgs {
    file: PathBuf,
}

#[derive(Parser)]
struct VerifyArgs {
    file: PathBuf,
}

#[derive(Parser)]
struct SignArgs {
    file: PathBuf,
    #[arg(long)]
    key: PathBuf,
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Parser)]
struct CompileArgs {
    file: PathBuf,
    /// Delivery target.
    #[arg(long)]
    target: CompileTarget,
    /// Output path.
    #[arg(short, long)]
    output: PathBuf,
    /// Nearest-neighbour upscale factor for web/print targets.
    #[arg(long, default_value_t = 1)]
    upscale: u32,
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum CompileTarget {
    Web,
    Vr,
    Print,
    Legal,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Err> {
    let cli = Cli::parse();
    match cli.command {
        Command::Create(a) => cmd_create(a),
        Command::Inspect(a) => cmd_inspect(a),
        Command::Verify(a) => cmd_verify(a),
        Command::Sign(a) => cmd_sign(a),
        Command::Compile(a) => cmd_compile(a),
        Command::FetchModels(a) => cmd_fetch_models(a),
        Command::Diff(a) => cmd_diff(a),
    }
}

/// A model weight to download as part of `aura fetch-models`.
#[derive(Clone)]
struct ModelAsset {
    /// File name written under the models directory.
    name: String,
    /// Source URL (verified public release asset).
    url: String,
    /// Human-readable note about what the model is for.
    note: String,
}

/// Default weights: Ultralytics YOLOv8 checkpoints (canonical `.pt` releases).
///
/// The `aura-onnx` `onnx` feature consumes **ONNX** weights; these `.pt`
/// checkpoints are the official source from which to export `.onnx` (see
/// `models/README.md` written by this command).
fn default_models() -> Vec<ModelAsset> {
    vec![
        ModelAsset {
            name: "yolov8n.pt".into(),
            url: "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolov8n.pt".into(),
            note: "YOLOv8n detection backbone (used to build the object-detection DAG).".into(),
        },
        ModelAsset {
            name: "yolov8n-seg.pt".into(),
            url: "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolov8n-seg.pt"
                .into(),
            note: "YOLOv8n-seg segmentation backbone (per-concept pixel masks).".into(),
        },
    ]
}

#[derive(Parser)]
struct FetchModelsArgs {
    /// Destination directory for the model weights (created if missing).
    #[arg(long, default_value = "models")]
    dir: PathBuf,
    /// Override the built-in asset list with a JSON manifest
    /// (`[{ "name": "...", "url": "..." }]`).
    #[arg(long)]
    manifest: Option<PathBuf>,
    /// Re-download even if the file already exists.
    #[arg(long)]
    force: bool,
}

/// Best-effort download via whatever CLI fetcher is available on the host.
fn download_file(url: &str, dest: &Path) -> Result<(), Err> {
    if cfg!(target_os = "windows") {
        let ps = format!(
            "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
            url,
            dest.display()
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps])
            .status()?;
        if status.success() {
            return Ok(());
        }
    }
    for tool in ["curl", "wget"] {
        if std::process::Command::new(tool)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            let status = if tool == "curl" {
                std::process::Command::new(tool)
                    .args(["-fL", "-o", &dest.display().to_string(), url])
                    .status()?
            } else {
                std::process::Command::new(tool)
                    .args(["-O", url])
                    .status()?
            };
            if status.success() {
                return Ok(());
            }
        }
    }
    Err(format!("no supported downloader found (curl/wget/powershell) for {url}").into())
}

fn cmd_fetch_models(a: FetchModelsArgs) -> Result<(), Err> {
    std::fs::create_dir_all(&a.dir)?;

    let assets: Vec<ModelAsset> = match &a.manifest {
        Some(p) => {
            let raw = std::fs::read_to_string(p)?;
            let parsed: Vec<serde_json::Value> = serde_json::from_str(&raw)
                .map_err(|e| format!("invalid manifest {}: {e}", p.display()))?;
            parsed
                .into_iter()
                .map(|v| ModelAsset {
                    name: v["name"].as_str().unwrap_or("model.bin").to_string(),
                    url: v["url"].as_str().unwrap_or("").to_string(),
                    note: v["note"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        }
        None => default_models(),
    };

    for asset in &assets {
        let dest = a.dir.join(&asset.name);
        if dest.exists() && !a.force {
            println!("skip   {} (already present)", asset.name);
            continue;
        }
        println!("fetch  {} — {}", asset.name, asset.note);
        download_file(&asset.url, &dest)?;
        if !dest.exists() || std::fs::metadata(&dest)?.len() == 0 {
            return Err(format!("download produced no data for {}", asset.name).into());
        }
        println!(
            "ok     {} ({} bytes)",
            asset.name,
            std::fs::metadata(&dest)?.len()
        );
    }

    // Write a usage note so users know how to turn these into ONNX weights.
    let readme = a.dir.join("README.md");
    std::fs::write(
        &readme,
        "# AURA model weights\n\n\
This directory holds the neural model weights used by the `aura-onnx` `onnx`\n\
feature. The files fetched by `aura fetch-models` are Ultralytics YOLOv8 `.pt`\n\
checkpoints (the canonical public releases).\n\n\
## Exporting to ONNX\n\n\
The `onnx` backend expects ONNX weights. Export the checkpoints with the\n\
Ultralytics CLI (requires Python + `pip install ultralytics`):\n\n\
```sh\n\
yolo export model=yolov8n.pt format=onnx\n\
yolo export model=yolov8n-seg.pt format=onnx\n\
```\n\n\
Then build with:\n\n\
```sh\n\
cargo build --workspace --features aura-onnx/onnx\n\
aura create photo.png -o photo.aura --detect --onnx\n\
```\n",
    )?;
    println!("wrote  {} (usage notes)", readme.display());
    println!(
        "\nfetch-models complete; {} weights in {}",
        assets.len(),
        a.dir.display()
    );
    Ok(())
}

#[derive(Parser)]
struct DiffArgs {
    /// First (older) AURA file.
    a: PathBuf,
    /// Second (newer) AURA file.
    b: PathBuf,
    /// Emit the diff as JSON instead of a human-readable report.
    #[arg(long)]
    json: bool,
}

fn cmd_diff(a: DiffArgs) -> Result<(), Err> {
    let bytes_a = std::fs::read(&a.a)?;
    let bytes_b = std::fs::read(&a.b)?;
    let file_a = open(&bytes_a)?;
    let file_b = open(&bytes_b)?;

    let report = libaura::diff::diff(&file_a, &file_b);
    if a.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{report}");
    }
    Ok(())
}

/// Load an image file into an `RgbImage`.
fn load_image(path: &PathBuf) -> Result<RgbImage, Err> {
    let dyn_img = image::open(path)?;
    let rgb = dyn_img.to_rgb8();
    let (w, h) = (rgb.width(), rgb.height());
    Ok(RgbImage {
        width: w,
        height: h,
        data: rgb.into_raw(),
    })
}

/// Save an `RgbImage` to a file (format inferred from extension).
fn save_image(img: &RgbImage, path: &PathBuf) -> Result<(), Err> {
    let buf = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(
        img.width,
        img.height,
        img.data.clone(),
    )
    .ok_or("invalid image buffer")?;
    buf.save(path)?;
    Ok(())
}

fn load_or_generate_key(
    path: &Option<PathBuf>,
    fallback: &Path,
) -> Result<(SigningKey, PathBuf), Err> {
    if let Some(p) = path {
        let bytes = std::fs::read(p)?;
        if bytes.len() != 32 {
            return Err("key file must be exactly 32 bytes".into());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok((SigningKey::from_bytes(&arr), p.clone()))
    } else {
        let mut arr = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut arr);
        let key_path = fallback.with_extension("aura.key");
        std::fs::write(&key_path, arr)?;
        Ok((SigningKey::from_bytes(&arr), key_path))
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn cmd_create(a: CreateArgs) -> Result<(), Err> {
    let img = load_image(&a.input)?;
    let sensor_hash = sha3_256(&img.data);

    let base = encode_tier0(&img, a.scale.max(1));
    let lum = LuminanceChromaRecord {
        width: base.base_width(),
        height: base.base_height(),
        depth: 8,
        sampling: 1,
        data: base.data().to_vec(),
    };
    let mut scene = libaura::container::SceneRecord::new();
    scene.push(Box::new(lum));

    let dag: SemanticDAG = if a.detect {
        let detector: Box<dyn aura_onnx::Detector> = match a.backend {
            Some(b) => aura_onnx::detector_for(to_aura_backend(b))?,
            None => aura_onnx::default_detector(),
        };
        detector.detect(&img)?
    } else {
        SemanticDAG::new()
    };

    let (key, key_path) = load_or_generate_key(&a.key, &a.output)?;
    let mut device_id = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut device_id);
    let genesis = GenesisBlock::sign(&key, sensor_hash, device_id, now_ms());

    let mut ledger = ProvenanceLedger::new(&key, sensor_hash);
    ledger.append(OpType::Capture, "aura-cli/create", &key)?;

    let builder = AuraBuilder::new(Bootstrap::with_default_wasm(), genesis, scene, dag, ledger);
    let bytes = builder.build()?;
    std::fs::write(&a.output, &bytes)?;

    println!(
        "created {} ({} bytes); device key written to {}",
        a.output.display(),
        bytes.len(),
        key_path.display()
    );
    if a.detect {
        println!(
            "embedded semantic DAG with {} nodes",
            builder.dag.nodes.len()
        );
    }
    Ok(())
}

fn cmd_inspect(a: InspectArgs) -> Result<(), Err> {
    let bytes = std::fs::read(&a.file)?;
    let file = open(&bytes)?;
    println!("== AURA file: {} ==", a.file.display());
    println!(
        "version: {}.{}",
        file.header.version_major, file.header.version_minor
    );
    println!("sections:");
    for e in &file.header.sections {
        println!(
            "  - type 0x{:02x} @ {} len {}",
            e.section_type, e.offset, e.length
        );
    }
    println!(
        "bootstrap: {} bytes (valid WASM: {})",
        file.bootstrap.bytes.len(),
        file.bootstrap.validate()
    );
    println!("genesis: data_hash={}", hex(&file.genesis.data_hash));
    println!("scene records: {}", file.scene.children.len());
    for (i, rec) in file.scene.children.iter().enumerate() {
        let tag = rec.type_tag();
        let desc = match tag {
            REC_LUMINANCE_CHROMA => "LUMINANCE_CHROMA",
            _ => "unknown",
        };
        println!("  [{}] 0x{:02x} {}", i, tag, desc);
    }
    println!(
        "semantic DAG: {} nodes, {} edges",
        file.dag.nodes.len(),
        file.dag.edges.len()
    );
    for n in &file.dag.nodes {
        println!("  node {} \"{}\" conf={:.2}", n.id, n.label, n.confidence);
    }
    println!("provenance ledger: {} entries", file.ledger.len());
    for (i, e) in file.ledger.entries.iter().enumerate() {
        println!("  [{}] {:?} by {}", i, e.op, e.software);
    }
    Ok(())
}

fn cmd_verify(a: VerifyArgs) -> Result<(), Err> {
    let bytes = std::fs::read(&a.file)?;
    let file = open(&bytes)?;

    // Verify the genesis hardware signature against the ledger's signer key.
    let signer = file.ledger.signer_key()?;
    match file.genesis.verify(&signer) {
        Ok(()) => println!("genesis signature: OK"),
        Err(e) => println!("genesis signature: FAILED ({e})"),
    }

    match file.verify() {
        Ok(()) => println!("trust chain: VERIFIED ({} entries)", file.ledger.len()),
        Err(e) => {
            println!("trust chain: BROKEN ({e})");
            return Err("trust seal broken".into());
        }
    }
    Ok(())
}

fn cmd_sign(a: SignArgs) -> Result<(), Err> {
    let key_bytes = std::fs::read(&a.key)?;
    if key_bytes.len() != 32 {
        return Err("key file must be exactly 32 bytes".into());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&key_bytes);
    let key = SigningKey::from_bytes(&arr);

    let bytes = std::fs::read(&a.file)?;
    let mut file: AuraFile = open(&bytes)?;
    file.ledger.append(OpType::Other, "aura-cli/sign", &key)?;

    let builder = AuraBuilder::new(
        file.bootstrap,
        file.genesis,
        file.scene,
        file.dag,
        file.ledger,
    );
    let out = builder.build()?;
    let dest = a.output.clone().unwrap_or(a.file);
    std::fs::write(&dest, &out)?;
    println!(
        "signed -> {} ({} entries)",
        dest.display(),
        open(&out)?.ledger.len()
    );
    Ok(())
}

fn cmd_compile(a: CompileArgs) -> Result<(), Err> {
    let bytes = std::fs::read(&a.file)?;
    let file = open(&bytes)?;

    // Locate the luminance/chroma record to obtain the Tier-0 base image.
    let lum = file
        .scene
        .children
        .iter()
        .find_map(|r| r.as_any().downcast_ref::<LuminanceChromaRecord>())
        .ok_or("no LUMINANCE_CHROMA record found")?;
    let mut img = RgbImage {
        width: lum.width,
        height: lum.height,
        data: lum.data.clone(),
    };
    if a.upscale > 1 {
        img = upscale_nearest(&img, img.width * a.upscale, img.height * a.upscale);
    }

    match a.target {
        CompileTarget::Web => {
            save_image(&img, &a.output)?;
            println!(
                "compiled -> {} (web delivery; Tier-0 base)",
                a.output.display()
            );
        }
        CompileTarget::Print => {
            save_image(&img, &a.output)?;
            println!(
                "compiled -> {} (print delivery; full-res requires Tier-1 ONNX)",
                a.output.display()
            );
        }
        CompileTarget::Vr => {
            let usda = usda_stub(&file);
            std::fs::write(&a.output, usda)?;
            println!(
                "compiled -> {} (USD scene description stub)",
                a.output.display()
            );
        }
        CompileTarget::Legal => {
            save_image(&img, &a.output)?;
            let manifest = file.ledger.to_c2pa_manifest()?;
            let manifest_path = a.output.with_extension("c2pa.json");
            std::fs::write(&manifest_path, &manifest)?;
            println!(
                "compiled -> {} + C2PA manifest {}",
                a.output.display(),
                manifest_path.display()
            );
        }
    }
    Ok(())
}

/// Nearest-neighbour upscale of an RGB image.
fn upscale_nearest(src: &RgbImage, tw: u32, th: u32) -> RgbImage {
    let mut out = RgbImage::new(tw, th);
    for y in 0..th {
        for x in 0..tw {
            let sx = (x * src.width / tw.max(1)).min(src.width - 1);
            let sy = (y * src.height / th.max(1)).min(src.height - 1);
            out.set_pixel(x, y, src.pixel(sx, sy));
        }
    }
    out
}

/// Minimal USD (USDA) scene description stub.
fn usda_stub(file: &AuraFile) -> String {
    let mut s = String::from("#usda 1.0\n(\n    doc = \"Compiled from AURA master file\"\n)\n{\n");
    s.push_str("    def Scope \"AURA\"\n    {\n");
    for n in &file.dag.nodes {
        s.push_str(&format!(
            "        def Xform \"concept_{}\"\n        {{\n            string label = \"{}\"\n            float confidence = {:.3}\n        }}\n",
            n.id, n.label, n.confidence
        ));
    }
    s.push_str("    }\n}\n");
    s
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upscale_is_correct_size() {
        let img = RgbImage::new(2, 2);
        let up = upscale_nearest(&img, 8, 8);
        assert_eq!((up.width, up.height), (8, 8));
    }
}
