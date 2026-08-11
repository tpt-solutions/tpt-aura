//! Benchmark: Tier-0 neural encode/decode throughput.
//!
//! Run with `cargo bench -p tpt_aura`.

use criterion::{criterion_group, criterion_main, Criterion};
use tpt_aura::neural::{decode_tier0, encode_tier0, RgbImage};

fn gradient(w: u32, h: u32) -> RgbImage {
    let mut img = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            img.set_pixel(x, y, (x as u8, y as u8, (x.wrapping_add(y)) as u8));
        }
    }
    img
}

fn bench_tier0(c: &mut Criterion) {
    let img = gradient(128, 128);
    c.bench_function("encode_tier0_128x128", |b| b.iter(|| encode_tier0(&img, 4)));
    let base = encode_tier0(&img, 4);
    c.bench_function("decode_tier0", |b| b.iter(|| decode_tier0(&base)));
}

criterion_group!(benches, bench_tier0);
criterion_main!(benches);
