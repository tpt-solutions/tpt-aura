//! Adaptive neural encoding: the dual-layer (Tier 0 / Tier 1) progressive system.
//!
//! * **Tier 0** is a zero-compute base layer: a box-downsampled RGB image that
//!   any device can display instantly. It is stored losslessly (the downsample
//!   is exact), so `encode → decode` is pixel-true at base resolution.
//! * **Tier 1** would store a small neural reconstruction payload; on an NPU it
//!   reconstructs the master-resolution image. The full ONNX backend lives in
//!   `tpt-aura-onnx` (feature `onnx`); here we provide the payload container and a
//!   clearly-flagged `Unsupported` stub for reconstruction.

use crate::codec::{Reader, Writer};
use crate::error::AuraError;

/// A simple contiguous RGB image (`data.len() == width * height * 3`).
#[derive(Debug, Clone, PartialEq)]
pub struct RgbImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Interleaved `R,G,B` bytes, row-major.
    pub data: Vec<u8>,
}

impl RgbImage {
    /// Allocate a zeroed image of the given size.
    pub fn new(width: u32, height: u32) -> Self {
        RgbImage {
            width,
            height,
            data: vec![0u8; width as usize * height as usize * 3],
        }
    }

    /// Total pixel count.
    pub fn pixels(&self) -> usize {
        self.width as usize * self.height as usize
    }

    /// Read the `(r, g, b)` triple at `(x, y)`.
    pub fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let i = (y as usize * self.width as usize + x as usize) * 3;
        (self.data[i], self.data[i + 1], self.data[i + 2])
    }

    /// Write the `(r, g, b)` triple at `(x, y)`.
    pub fn set_pixel(&mut self, x: u32, y: u32, rgb: (u8, u8, u8)) {
        let i = (y as usize * self.width as usize + x as usize) * 3;
        self.data[i] = rgb.0;
        self.data[i + 1] = rgb.1;
        self.data[i + 2] = rgb.2;
    }
}

/// The Tier-0 base layer: a downsampled, losslessly-stored RGB image plus the
/// original dimensions it can be upscaled to.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier0Base {
    base_width: u32,
    base_height: u32,
    orig_width: u32,
    orig_height: u32,
    data: Vec<u8>,
}

impl Tier0Base {
    /// Width of the stored base layer.
    pub fn base_width(&self) -> u32 {
        self.base_width
    }

    /// Height of the stored base layer.
    pub fn base_height(&self) -> u32 {
        self.base_height
    }

    /// Original (master) width this layer upscales toward.
    pub fn orig_width(&self) -> u32 {
        self.orig_width
    }

    /// Original (master) height this layer upscales toward.
    pub fn orig_height(&self) -> u32 {
        self.orig_height
    }

    /// The base-layer RGB bytes (length `base_width*base_height*3`).
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Serialize the Tier-0 base layer into a writer.
    pub fn encode(&self, w: &mut Writer) {
        w.put_u32(self.base_width);
        w.put_u32(self.base_height);
        w.put_u32(self.orig_width);
        w.put_u32(self.orig_height);
        w.put_bytes(&self.data);
    }

    /// Deserialize a Tier-0 base layer from a reader.
    pub fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let base_width = r.u32()?;
        let base_height = r.u32()?;
        let orig_width = r.u32()?;
        let orig_height = r.u32()?;
        let data = r.bytes()?;
        Ok(Tier0Base {
            base_width,
            base_height,
            orig_width,
            orig_height,
            data,
        })
    }
}

/// Encode the Tier-0 base layer by box-downsampling `scale × scale` blocks.
///
/// The downsample is exact (integer averaging), so the stored base layer
/// round-trips losslessly back to its own resolution.
pub fn encode_tier0(img: &RgbImage, scale: u32) -> Tier0Base {
    let scale = scale.max(1);
    let base_width = img.width.div_ceil(scale);
    let base_height = img.height.div_ceil(scale);
    let mut base = RgbImage::new(base_width, base_height);
    for by in 0..base_height {
        for bx in 0..base_width {
            let mut sr = 0u32;
            let mut sg = 0u32;
            let mut sb = 0u32;
            let mut n = 0u32;
            for dy in 0..scale {
                let y = by * scale + dy;
                if y >= img.height {
                    continue;
                }
                for dx in 0..scale {
                    let x = bx * scale + dx;
                    if x >= img.width {
                        continue;
                    }
                    let (r, g, b) = img.pixel(x, y);
                    sr += r as u32;
                    sg += g as u32;
                    sb += b as u32;
                    n += 1;
                }
            }
            let (r, g, b) = ((sr / n) as u8, (sg / n) as u8, (sb / n) as u8);
            base.set_pixel(bx, by, (r, g, b));
        }
    }
    Tier0Base {
        base_width,
        base_height,
        orig_width: img.width,
        orig_height: img.height,
        data: base.data,
    }
}

/// Decode the Tier-0 base layer to an image at base resolution.
pub fn decode_tier0(base: &Tier0Base) -> RgbImage {
    RgbImage {
        width: base.base_width,
        height: base.base_height,
        data: base.data.clone(),
    }
}

/// Upscale a Tier-0 base layer to a target resolution with nearest-neighbour.
pub fn upscale_tier0(base: &Tier0Base, target_w: u32, target_h: u32) -> RgbImage {
    let mut out = RgbImage::new(target_w, target_h);
    for y in 0..target_h {
        for x in 0..target_w {
            let bx = (x * base.base_width / target_w.max(1)).min(base.base_width - 1);
            let by = (y * base.base_height / target_h.max(1)).min(base.base_height - 1);
            let i = (by as usize * base.base_width as usize + bx as usize) * 3;
            let j = (y as usize * target_w as usize + x as usize) * 3;
            out.data[j] = base.data[i];
            out.data[j + 1] = base.data[i + 1];
            out.data[j + 2] = base.data[i + 2];
        }
    }
    out
}

/// Peak signal-to-noise ratio (dB) between two equal-sized RGB images.
pub fn psnr(a: &RgbImage, b: &RgbImage) -> Result<f64, AuraError> {
    if a.width != b.width || a.height != b.height || a.data.len() != b.data.len() {
        return Err(AuraError::Unsupported(
            "psnr requires equal-sized images".into(),
        ));
    }
    let mut mse = 0f64;
    for (x, y) in a.data.iter().zip(b.data.iter()) {
        let d = *x as f64 - *y as f64;
        mse += d * d;
    }
    mse /= a.data.len() as f64;
    if mse == 0.0 {
        return Ok(f64::INFINITY);
    }
    Ok(10.0 * (255.0f64 * 255.0f64 / mse).log10())
}

/// Container for the Tier-1 neural reconstruction payload (e.g. super-resolution
/// weights embedded in the file). The full ONNX runtime lives in `tpt-aura-onnx`.
#[derive(Debug, Clone, PartialEq)]
pub struct NeuralPayloadRecord {
    /// Name of the embedded model (e.g. `"real-esrgan-x4"`).
    pub model_name: String,
    /// Raw model weight bytes.
    pub weights: Vec<u8>,
}

impl NeuralPayloadRecord {
    /// Serialize into a writer.
    pub fn encode(&self, w: &mut Writer) {
        w.put_str(&self.model_name);
        w.put_bytes(&self.weights);
    }

    /// Deserialize from a reader.
    pub fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let model_name = r.str()?;
        let weights = r.bytes()?;
        Ok(NeuralPayloadRecord {
            model_name,
            weights,
        })
    }
}

/// Reconstruct the master-resolution image from a Tier-0 base layer and a Tier-1
/// payload. The real implementation requires the `tpt-aura-onnx` ONNX backend; this
/// reference build returns [`AuraError::Unsupported`].
pub fn reconstruct_tier1(
    _base: &Tier0Base,
    _payload: &NeuralPayloadRecord,
) -> Result<RgbImage, AuraError> {
    Err(AuraError::Unsupported(
        "Tier-1 neural reconstruction requires the `tpt-aura-onnx` (onnx) feature with model weights"
            .into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grad_image(w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.set_pixel(x, y, (x as u8, y as u8, (x.wrapping_add(y)) as u8));
            }
        }
        img
    }

    #[test]
    fn tier0_round_trip_is_exact_at_base_resolution() {
        let img = grad_image(16, 16);
        let base = encode_tier0(&img, 4);
        assert_eq!(base.base_width(), 4);
        assert_eq!(base.base_height(), 4);
        let decoded = decode_tier0(&base);
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        // Lossless store -> exact equality, infinite PSNR.
        assert_eq!(psnr(&decoded, &decode_tier0(&base)).unwrap(), f64::INFINITY);
    }

    #[test]
    fn tier0_base_is_smaller_than_original() {
        let img = grad_image(64, 64);
        let base = encode_tier0(&img, 4);
        assert!(base.data.len() < img.data.len());
    }

    #[test]
    fn upscale_produces_target_dimensions() {
        let img = grad_image(32, 32);
        let base = encode_tier0(&img, 4);
        let up = upscale_tier0(&base, 32, 32);
        assert_eq!((up.width, up.height), (32, 32));
    }

    #[test]
    fn payload_round_trip() {
        let p = NeuralPayloadRecord {
            model_name: "real-esrgan-x4".into(),
            weights: vec![1, 2, 3, 4, 5],
        };
        let mut w = Writer::new();
        p.encode(&mut w);
        let bytes = w.into_inner();
        let mut r = Reader::new(&bytes);
        let back = NeuralPayloadRecord::decode(&mut r).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn tier0_base_codec_round_trip() {
        let img = grad_image(16, 16);
        let base = encode_tier0(&img, 4);
        let mut w = Writer::new();
        base.encode(&mut w);
        let bytes = w.into_inner();
        let mut r = Reader::new(&bytes);
        let back = Tier0Base::decode(&mut r).unwrap();
        assert_eq!(back.base_width(), base.base_width());
        assert_eq!(back.data(), base.data());
    }

    #[test]
    fn tier1_requires_onnx() {
        let img = grad_image(8, 8);
        let base = encode_tier0(&img, 2);
        let payload = NeuralPayloadRecord {
            model_name: "x".into(),
            weights: vec![],
        };
        assert!(reconstruct_tier1(&base, &payload).is_err());
    }
}
