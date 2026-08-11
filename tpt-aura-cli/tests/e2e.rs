//! End-to-end CLI integration test: create -> inspect -> verify -> sign -> compile.

use std::process::Command;

use image::{ImageBuffer, Rgb, RgbImage};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aura")
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(bin()).args(args).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (out.status.success(), format!("{stdout}{stderr}"))
}

#[test]
fn e2e_pipeline() {
    let dir = std::env::temp_dir().join("aura_e2e_test");
    let _ = std::fs::create_dir_all(&dir);

    let in_png = dir.join("in.png");
    let out_aura = dir.join("out.aura");
    let signed_aura = dir.join("signed.aura");
    let web_png = dir.join("web.png");
    let legal_jpg = dir.join("legal.jpg");
    let legal_c2pa = dir.join("legal.c2pa.json");
    let vr_usda = dir.join("vr.usda");

    // Build a 16x16 gradient image.
    let mut img: RgbImage = ImageBuffer::new(16, 16);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgb([x as u8, y as u8, (x.wrapping_add(y)) as u8]);
    }
    img.save(&in_png).unwrap();

    // create (with semantic detection)
    let (ok, log) = run(&[
        "create",
        in_png.to_str().unwrap(),
        "-o",
        out_aura.to_str().unwrap(),
        "--detect",
    ]);
    assert!(ok, "create failed: {log}");
    assert!(out_aura.exists());

    // inspect
    let (ok, log) = run(&["inspect", out_aura.to_str().unwrap()]);
    assert!(ok, "inspect failed: {log}");
    assert!(log.contains("AURA"));
    assert!(log.contains("semantic DAG"));

    // verify (trust chain)
    let (ok, log) = run(&["verify", out_aura.to_str().unwrap()]);
    assert!(ok, "verify failed: {log}");
    assert!(log.contains("VERIFIED"), "verify log: {log}");

    // sign with the device key produced during create
    let key_path = out_aura.with_extension("aura.key");
    assert!(key_path.exists(), "device key should exist");
    let (ok, log) = run(&[
        "sign",
        out_aura.to_str().unwrap(),
        "--key",
        key_path.to_str().unwrap(),
        "-o",
        signed_aura.to_str().unwrap(),
    ]);
    assert!(ok, "sign failed: {log}");
    assert!(signed_aura.exists());

    // verify the signed file (now 2 ledger entries)
    let (ok, log) = run(&["verify", signed_aura.to_str().unwrap()]);
    assert!(ok, "verify(signed) failed: {log}");
    assert!(log.contains("2 entries"), "expected 2 entries: {log}");

    // compile -> web
    let (ok, log) = run(&[
        "compile",
        signed_aura.to_str().unwrap(),
        "--target",
        "web",
        "-o",
        web_png.to_str().unwrap(),
        "--upscale",
        "2",
    ]);
    assert!(ok, "compile web failed: {log}");
    assert!(web_png.exists());

    // compile -> legal (writes a C2PA manifest sidecar)
    let (ok, log) = run(&[
        "compile",
        signed_aura.to_str().unwrap(),
        "--target",
        "legal",
        "-o",
        legal_jpg.to_str().unwrap(),
    ]);
    assert!(ok, "compile legal failed: {log}");
    assert!(legal_jpg.exists());
    assert!(legal_c2pa.exists(), "C2PA manifest expected");

    // compile -> vr (USDA stub)
    let (ok, log) = run(&[
        "compile",
        signed_aura.to_str().unwrap(),
        "--target",
        "vr",
        "-o",
        vr_usda.to_str().unwrap(),
    ]);
    assert!(ok, "compile vr failed: {log}");
    assert!(vr_usda.exists());

    let _ = std::fs::remove_dir_all(&dir);
}
