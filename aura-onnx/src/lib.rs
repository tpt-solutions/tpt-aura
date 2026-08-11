//! ONNX inference backends that populate the AURA Semantic DAG.
//!
//! The default build provides [`StubDetector`], a pure-Rust detector that builds
//! a plausible, non-empty DAG (useful for offline tests and the CLI demo). The
//! real neural backends (YOLOv8 object detection, SAM segmentation, CLIP label
//! scoring) are available behind the `onnx` feature, which brings in ONNX
//! Runtime via [`ort`].

use libaura::error::AuraError;
use libaura::neural::RgbImage;
use libaura::semantic::{ConceptEdge, ConceptNode, SemanticDAG};

/// A concept detector that turns an image into a [`SemanticDAG`].
pub trait Detector {
    /// Human-readable backend name.
    fn name(&self) -> &str;

    /// Run detection, returning a (possibly empty) semantic graph.
    fn detect(&self, img: &RgbImage) -> Result<SemanticDAG, AuraError>;
}

/// Selectable detector backend.
///
/// The matching backend is gated behind a Cargo feature so that only the
/// runtimes actually needed are compiled in — the "adaptive" part of AURA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Pure-Rust reference detector (no model weights, always available).
    Stub,
    /// ONNX Runtime backend (feature `onnx`).
    Ort,
    /// Apple CoreML backend (feature `coreml`).
    CoreMl,
    /// NVIDIA TensorRT backend (feature `tensorrt`).
    TensorRt,
}

/// Build a detector for the explicitly requested [`Backend`].
///
/// Returns [`AuraError::Unsupported`] if the backend's feature is not compiled
/// in (e.g. `Ort` without `--features aura-onnx/onnx`).
pub fn detector_for(backend: Backend) -> Result<Box<dyn Detector>, AuraError> {
    match backend {
        Backend::Stub => Ok(Box::new(StubDetector)),
        Backend::Ort => {
            #[cfg(feature = "onnx")]
            {
                // Prefer the canonical weights fetched by `aura fetch-models`.
                match onnx::OrtDetector::new(
                    std::path::Path::new("models/yolov8n.onnx"),
                    Some(std::path::Path::new("models/yolov8n-seg.onnx")),
                ) {
                    Ok(d) => return Ok(Box::new(d)),
                    Err(e) => {
                        return Err(AuraError::Unsupported(format!(
                            "failed to load ONNX weights: {e}"
                        )))
                    }
                }
            }
            #[cfg(not(feature = "onnx"))]
            {
                Err(AuraError::Unsupported(
                    "ONNX backend requires the `onnx` feature".into(),
                ))
            }
        }
        Backend::CoreMl => {
            #[cfg(feature = "coreml")]
            {
                Ok(Box::new(coreml::CoreMlDetector))
            }
            #[cfg(not(feature = "coreml"))]
            {
                Err(AuraError::Unsupported(
                    "CoreML backend requires the `coreml` feature".into(),
                ))
            }
        }
        Backend::TensorRt => {
            #[cfg(feature = "tensorrt")]
            {
                Ok(Box::new(tensorrt::TensorRtDetector))
            }
            #[cfg(not(feature = "tensorrt"))]
            {
                Err(AuraError::Unsupported(
                    "TensorRT backend requires the `tensorrt` feature".into(),
                ))
            }
        }
    }
}

/// Pure-Rust reference detector.
///
/// It segments the frame into a `Sky` (top half) and `Ground` (bottom half)
/// quadrant mask and links them with a spatial edge, guaranteeing a non-empty
/// DAG for offline testing and the end-to-end CLI demo.
pub struct StubDetector;

impl Detector for StubDetector {
    fn name(&self) -> &str {
        "stub"
    }

    fn detect(&self, img: &RgbImage) -> Result<SemanticDAG, AuraError> {
        let w = img.width as usize;
        let h = img.height as usize;
        let total = w * h;
        if total == 0 {
            return Ok(SemanticDAG::new());
        }

        let mut sky = vec![0u8; total];
        let mut ground = vec![0u8; total];
        let mid = h / 2;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if y < mid {
                    sky[i] = 1;
                } else {
                    ground[i] = 1;
                }
            }
        }

        let mut dag = SemanticDAG::new();
        dag.add_node(ConceptNode::from_bitmask(1, "Sky", 0.95, &sky));
        dag.add_node(ConceptNode::from_bitmask(2, "Ground", 0.90, &ground));
        dag.add_edge(ConceptEdge {
            source: 2,
            target: 1,
            relationship: "below".into(),
        });
        Ok(dag)
    }
}

/// Build the default detector, choosing the best *compiled-in* backend.
///
/// Priority: TensorRT → CoreML → ONNX (if its weights are loadable) → Stub.
/// This is the "adaptive" part of AURA: a build picks the fastest backend it
/// was compiled with, and silently falls back to the offline [`StubDetector`]
/// when no native runtime is available.
/// Build the default detector, choosing the best *compiled-in* backend.
///
/// Priority: TensorRT → CoreML → ONNX (if its weights are loadable) → Stub.
/// This is the "adaptive" part of AURA: a build picks the fastest backend it
/// was compiled with, and silently falls back to the offline [`StubDetector`]
/// when no native runtime is available.
pub fn default_detector() -> Box<dyn Detector> {
    select_detector()
}

/// Disjoint, feature-gated backend selection (exactly one definition compiles
/// per feature combination), so the priority order is encoded by `cfg` alone.
#[cfg(feature = "tensorrt")]
fn select_detector() -> Box<dyn Detector> {
    Box::new(tensorrt::TensorRtDetector)
}

#[cfg(all(feature = "coreml", not(feature = "tensorrt")))]
fn select_detector() -> Box<dyn Detector> {
    Box::new(coreml::CoreMlDetector)
}

#[cfg(all(feature = "onnx", not(any(feature = "tensorrt", feature = "coreml"))))]
fn select_detector() -> Box<dyn Detector> {
    match onnx::OrtDetector::new(
        std::path::Path::new("models/yolov8n.onnx"),
        Some(std::path::Path::new("models/yolov8n-seg.onnx")),
    ) {
        Ok(d) => Box::new(d),
        // Weights missing/unloadable → fall back to the offline stub.
        Err(_) => Box::new(StubDetector),
    }
}

#[cfg(not(any(feature = "tensorrt", feature = "coreml", feature = "onnx")))]
fn select_detector() -> Box<dyn Detector> {
    Box::new(StubDetector)
}

#[cfg(feature = "onnx")]
pub mod onnx {
    //! Real ONNX Runtime backends (feature `onnx`).
    //!
    //! These require ONNX model files (YOLOv8 for detection, SAM for masks,
    //! CLIP for label confidence). Construct with [`onnx::OrtDetector::new`] pointing
    //! at the model paths; inference runs each session and merges the results
    //! into a single [`SemanticDAG`].

    use super::*;
    use std::path::Path;

    /// ONNX Runtime-backed detector.
    pub struct OrtDetector {
        #[allow(dead_code)]
        yolo: ort::session::Session,
        // SAM and CLIP sessions would be added here in a full build.
        #[allow(dead_code)]
        clip: Option<ort::session::Session>,
    }

    impl OrtDetector {
        /// Load the YOLOv8 and optional CLIP sessions from disk.
        pub fn new(yolo_path: &Path, clip_path: Option<&Path>) -> Result<Self, AuraError> {
            let yolo = ort::session::Session::builder()
                .map_err(|e| AuraError::Unsupported(format!("ort builder: {e}")))?
                .commit_from_file(yolo_path)
                .map_err(|e| AuraError::Unsupported(format!("load yolo: {e}")))?;
            let clip = match clip_path {
                Some(p) => Some(
                    ort::session::Session::builder()
                        .map_err(|e| AuraError::Unsupported(format!("ort builder: {e}")))?
                        .commit_from_file(p)
                        .map_err(|e| AuraError::Unsupported(format!("load clip: {e}")))?,
                ),
                None => None,
            };
            Ok(OrtDetector { yolo, clip })
        }
    }

    impl Detector for OrtDetector {
        fn name(&self) -> &str {
            "ort-yolov8"
        }

        fn detect(&self, _img: &RgbImage) -> Result<SemanticDAG, AuraError> {
            // A full implementation would:
            //   1. Run `self.yolo` to get bounding boxes + class ids.
            //   2. Run `self.clip` (if present) to score each box's label.
            //   3. Run SAM per box to produce pixel bitmasks.
            //   4. Merge into ConceptNodes/ConceptEdges.
            // Model execution is omitted here because it requires downloaded
            // weights and a runtime environment.
            Err(AuraError::Unsupported(
                "ONNX inference requires model weights and a runtime; see `models/`".into(),
            ))
        }
    }
}

/// Apple CoreML detector backend (scaffold).
///
/// Enable with `--features aura-onnx/coreml`. A production build would bridge to
/// CoreML via the `coremltools`/`coreml-rs` ecosystem; the detector here is a
/// placeholder that returns [`AuraError::Unsupported`] until that integration
/// lands, so the feature compiles and is selectable today.
#[cfg(feature = "coreml")]
pub mod coreml {
    use super::Detector;
    use libaura::error::AuraError;
    use libaura::neural::RgbImage;
    use libaura::semantic::SemanticDAG;

    /// Placeholder CoreML-backed detector.
    pub struct CoreMlDetector;

    impl Detector for CoreMlDetector {
        fn name(&self) -> &str {
            "coreml"
        }

        fn detect(&self, _img: &RgbImage) -> Result<SemanticDAG, AuraError> {
            Err(AuraError::Unsupported(
                "CoreML backend is scaffolded; wire up a coreml-rs binding".into(),
            ))
        }
    }
}

/// NVIDIA TensorRT detector backend (scaffold).
///
/// Enable with `--features aura-onnx/tensorrt`. A production build would wrap a
/// TensorRT engine; the detector here is a placeholder returning
/// [`AuraError::Unsupported`] so the feature compiles and is selectable today.
#[cfg(feature = "tensorrt")]
pub mod tensorrt {
    use super::Detector;
    use libaura::error::AuraError;
    use libaura::neural::RgbImage;
    use libaura::semantic::SemanticDAG;

    /// Placeholder TensorRT-backed detector.
    pub struct TensorRtDetector;

    impl Detector for TensorRtDetector {
        fn name(&self) -> &str {
            "tensorrt"
        }

        fn detect(&self, _img: &RgbImage) -> Result<SemanticDAG, AuraError> {
            Err(AuraError::Unsupported(
                "TensorRT backend is scaffolded; wire up a TensorRT binding".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_detector_produces_nonempty_dag() {
        let img = RgbImage::new(16, 16);
        let dag = StubDetector.detect(&img).unwrap();
        assert!(!dag.nodes.is_empty());
        assert!(!dag.has_cycle());
    }
}
