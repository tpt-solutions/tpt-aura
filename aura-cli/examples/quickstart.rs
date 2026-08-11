//! Runnable quickstart: create → inspect → verify → sign → compile.
//!
//! This mirrors `aura-cli/tests/e2e.rs` but runs in-process (no subprocess) and
//! needs **zero external downloads** — the source image is generated in memory.
//!
//! Run it with:
//!
//! ```sh
//! cargo run -p aura-cli --example quickstart
//! ```
//!
//! It writes its artifacts into a temporary directory and prints the pipeline
//! progress to stdout.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use image::{ImageBuffer, Rgb};
use libaura::bootstrap::Bootstrap;
use libaura::container::{open, AuraBuilder, LuminanceChromaRecord};
use libaura::neural::{encode_tier0, RgbImage};
use libaura::provenance::{sha3_256, GenesisBlock, OpType, ProvenanceLedger};
use libaura::semantic::SemanticDAG;
use rand::RngCore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join("aura_quickstart");
    std::fs::create_dir_all(&dir)?;

    let in_png = dir.join("in.png");
    let out_aura = dir.join("out.aura");
    let signed_aura = dir.join("signed.aura");
    let web_png = dir.join("web.png");
    let legal_jpg = dir.join("legal.jpg");

    // --- create ----------------------------------------------------------
    // Generate a tiny gradient image in memory (no external file needed).
    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(16, 16);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgb([x as u8, y as u8, (x.wrapping_add(y)) as u8]);
    }
    img.save(&in_png)?;
    let rgb = RgbImage {
        width: 16,
        height: 16,
        data: img.into_raw(),
    };

    let sensor_hash = sha3_256(&rgb.data);
    let base = encode_tier0(&rgb, 4);
    let lum = LuminanceChromaRecord {
        width: base.base_width(),
        height: base.base_height(),
        depth: 8,
        sampling: 1,
        data: base.data().to_vec(),
    };
    let mut scene = libaura::container::SceneRecord::new();
    scene.push(Box::new(lum));

    // Embed a Semantic DAG (offline stub detector — no model weights needed).
    let dag: SemanticDAG = aura_onnx::default_detector().detect(&rgb)?;

    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let key = SigningKey::from_bytes(&key);
    let mut device_id = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut device_id);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let genesis = GenesisBlock::sign(&key, sensor_hash, device_id, now);

    let mut ledger = ProvenanceLedger::new(&key, sensor_hash);
    ledger.append(OpType::Capture, "quickstart/create", &key)?;

    let bytes =
        AuraBuilder::new(Bootstrap::with_default_wasm(), genesis, scene, dag, ledger).build()?;
    std::fs::write(&out_aura, &bytes)?;
    println!("create -> {} ({} bytes)", out_aura.display(), bytes.len());

    // --- inspect ---------------------------------------------------------
    let file = open(&bytes)?;
    println!(
        "inspect -> version {}.{}, {} scene records, {} DAG nodes, {} ledger entries",
        file.header.version_major,
        file.header.version_minor,
        file.scene.children.len(),
        file.dag.nodes.len(),
        file.ledger.len(),
    );

    // --- verify ----------------------------------------------------------
    file.verify()?;
    println!("verify -> trust chain VERIFIED");

    // --- sign ------------------------------------------------------------
    let mut file = open(&bytes)?;
    file.ledger.append(OpType::Other, "quickstart/sign", &key)?;
    let signed = AuraBuilder::new(
        file.bootstrap,
        file.genesis,
        file.scene,
        file.dag,
        file.ledger,
    )
    .build()?;
    std::fs::write(&signed_aura, &signed)?;
    println!(
        "sign -> {} ({} ledger entries)",
        signed_aura.display(),
        open(&signed)?.ledger.len()
    );

    // --- compile: web (PNG) ---------------------------------------------
    let signed_file = open(&signed)?;
    let lum = signed_file
        .scene
        .children
        .iter()
        .find_map(|r| r.as_any().downcast_ref::<LuminanceChromaRecord>())
        .ok_or("no LUMINANCE_CHROMA record")?;
    let base_img = RgbImage {
        width: lum.width,
        height: lum.height,
        data: lum.data.clone(),
    };
    save(&base_img, &web_png)?;
    println!("compile (web) -> {}", web_png.display());

    // --- compile: legal (JPEG + C2PA manifest) ---------------------------
    save(&base_img, &legal_jpg)?;
    let manifest = signed_file.ledger.to_c2pa_manifest()?;
    std::fs::write(legal_jpg.with_extension("c2pa.json"), manifest)?;
    println!("compile (legal) -> {} + C2PA manifest", legal_jpg.display());

    println!("\nquickstart complete; artifacts in {}", dir.display());
    Ok(())
}

fn save(img: &RgbImage, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let buf = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(img.width, img.height, img.data.clone())
        .ok_or("invalid image buffer")?;
    buf.save(path)?;
    Ok(())
}
