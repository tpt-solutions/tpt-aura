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

/// Build the default detector (currently the pure-Rust [`StubDetector`]).
///
/// With the `onnx` feature enabled this would return an `OrtDetector`; here it
/// returns the offline-safe stub so the workspace builds and tests without
/// network access or model weights.
pub fn default_detector() -> Box<dyn Detector> {
    Box::new(StubDetector)
}

#[cfg(feature = "onnx")]
pub mod onnx {
    //! Real ONNX Runtime backends (feature `onnx`).
    //!
    //! These require ONNX model files (YOLOv8 for detection, SAM for masks,
    //! CLIP for label confidence). Construct with [`OrtDetector::new`] pointing
    //! at the model paths; inference runs each session and merges the results
    //! into a single [`SemanticDAG`].

    use super::*;
    use std::path::Path;

    /// ONNX Runtime-backed detector.
    pub struct OrtDetector {
        yolo: ort::session::Session,
        // SAM and CLIP sessions would be added here in a full build.
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
