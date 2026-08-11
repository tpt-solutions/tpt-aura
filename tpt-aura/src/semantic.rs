//! The Semantic Directed Acyclic Graph (DAG).
//!
//! A [`SemanticDAG`] embeds a concept graph directly in the file: each
//! [`ConceptNode`] carries a label, a confidence, and a run-length-encoded
//! bitmask mapping the concept onto the pixel grid. [`ConceptEdge`]s describe
//! spatial relationships between concepts. The module provides Kahn's
//! algorithm for cycle detection and RLE compression for the bitmasks.

use crate::codec::{Reader, Writer};
use crate::error::AuraError;

/// A single concept in the semantic graph (e.g. "Sky", "Person").
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptNode {
    /// Stable node identifier (used by edges).
    pub id: u32,
    /// Human-readable label.
    pub label: String,
    /// Model confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Run-length-encoded bitmask (see [`compress_bitmask`]) aligned to the
    /// luminance/spatial record it annotates.
    pub bitmask_rle: Vec<u8>,
}

impl ConceptNode {
    /// Build a node from a raw (uncompressed) 0/1 bitmask.
    pub fn from_bitmask(id: u32, label: &str, confidence: f32, mask: &[u8]) -> Self {
        ConceptNode {
            id,
            label: label.to_owned(),
            confidence,
            bitmask_rle: compress_bitmask(mask),
        }
    }

    /// Decompress the stored bitmask back into a 0/1 vector.
    pub fn bitmask(&self) -> Result<Vec<u8>, AuraError> {
        decompress_bitmask(&self.bitmask_rle)
    }

    fn encode(&self, w: &mut Writer) {
        w.put_u32(self.id);
        w.put_str(&self.label);
        w.put_f32(self.confidence);
        w.put_bytes(&self.bitmask_rle);
    }

    fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let id = r.u32()?;
        let label = r.str()?;
        let confidence = r.f32()?;
        let bitmask_rle = r.bytes()?;
        Ok(ConceptNode {
            id,
            label,
            confidence,
            bitmask_rle,
        })
    }
}

/// A directed relationship between two concept nodes (e.g. `Person -> Car`).
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptEdge {
    /// Source node id.
    pub source: u32,
    /// Target node id.
    pub target: u32,
    /// Relationship label, e.g. `"is_in_front_of"`.
    pub relationship: String,
}

impl ConceptEdge {
    fn encode(&self, w: &mut Writer) {
        w.put_u32(self.source);
        w.put_u32(self.target);
        w.put_str(&self.relationship);
    }

    fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let source = r.u32()?;
        let target = r.u32()?;
        let relationship = r.str()?;
        Ok(ConceptEdge {
            source,
            target,
            relationship,
        })
    }
}

/// A semantic graph of concepts and their spatial relationships.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticDAG {
    /// Concept nodes.
    pub nodes: Vec<ConceptNode>,
    /// Directed edges between node ids.
    pub edges: Vec<ConceptEdge>,
}

impl SemanticDAG {
    /// An empty graph.
    pub fn new() -> Self {
        SemanticDAG::default()
    }

    /// Add a concept node; returns its id.
    pub fn add_node(&mut self, node: ConceptNode) -> u32 {
        let id = node.id;
        self.nodes.push(node);
        id
    }

    /// Add a directed edge.
    pub fn add_edge(&mut self, edge: ConceptEdge) {
        self.edges.push(edge);
    }

    /// Whether the graph contains a directed cycle (Kahn's algorithm).
    pub fn has_cycle(&self) -> bool {
        self.topo_sort().is_err()
    }

    /// Topologically sort node ids (Kahn's algorithm). Returns
    /// [`AuraError::CycleDetected`] if a cycle exists.
    pub fn topo_sort(&self) -> Result<Vec<u32>, AuraError> {
        let mut indegree: std::collections::HashMap<u32, usize> =
            self.nodes.iter().map(|n| (n.id, 0)).collect();
        let mut adj: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        for e in &self.edges {
            *indegree.entry(e.target).or_insert(0) += 1;
            adj.entry(e.source).or_default().push(e.target);
        }
        let mut queue: std::collections::VecDeque<u32> = indegree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(targets) = adj.get(&id) {
                for &t in targets {
                    let d = indegree.get_mut(&t).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(t);
                    }
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err(AuraError::CycleDetected);
        }
        Ok(order)
    }

    /// Serialize into a writer.
    pub fn encode(&self, w: &mut Writer) {
        w.put_u32(self.nodes.len() as u32);
        for n in &self.nodes {
            n.encode(w);
        }
        w.put_u32(self.edges.len() as u32);
        for e in &self.edges {
            e.encode(w);
        }
    }

    /// Deserialize from a reader.
    pub fn decode(r: &mut Reader) -> Result<Self, AuraError> {
        let n = r.u32()? as usize;
        let mut nodes = Vec::with_capacity(n);
        for _ in 0..n {
            nodes.push(ConceptNode::decode(r)?);
        }
        let m = r.u32()? as usize;
        let mut edges = Vec::with_capacity(m);
        for _ in 0..m {
            edges.push(ConceptEdge::decode(r)?);
        }
        Ok(SemanticDAG { nodes, edges })
    }
}

/// Run-length-encode a 0/1 bitmask: a sequence of `(value: u8, count: u32)` runs.
pub fn compress_bitmask(mask: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut iter = mask.iter().copied();
    if let Some(first) = iter.next() {
        let mut value = first;
        let mut count: u32 = 1;
        for bit in iter {
            if bit == value {
                count += 1;
            } else {
                out.push(value.min(1));
                out.extend_from_slice(&count.to_le_bytes());
                value = bit;
                count = 1;
            }
        }
        out.push(value.min(1));
        out.extend_from_slice(&count.to_le_bytes());
    }
    out
}

/// Reverse [`compress_bitmask`], reconstructing the original 0/1 mask.
pub fn decompress_bitmask(rle: &[u8]) -> Result<Vec<u8>, AuraError> {
    let mut r = Reader::new(rle);
    let mut out = Vec::new();
    while r.remaining() > 0 {
        let value = r.u8()?;
        let count = r.u32()?;
        for _ in 0..count {
            out.push(value.min(1));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmask_round_trip() {
        let mask: Vec<u8> = vec![1, 1, 1, 0, 0, 1, 0, 0, 0, 0];
        let rle = compress_bitmask(&mask);
        let back = decompress_bitmask(&rle).unwrap();
        assert_eq!(mask, back);
        // RLE should be shorter than the raw mask for long, repetitive data.
        let long: Vec<u8> = std::iter::repeat(1u8).take(1000).collect();
        assert!(compress_bitmask(&long).len() <= long.len());
    }

    #[test]
    fn no_cycle_is_topo_sorted() {
        let mut dag = SemanticDAG::new();
        dag.add_node(ConceptNode::from_bitmask(1, "Sky", 0.9, &[]));
        dag.add_node(ConceptNode::from_bitmask(2, "Person", 0.8, &[]));
        dag.add_edge(ConceptEdge {
            source: 2,
            target: 1,
            relationship: "is_in_front_of".into(),
        });
        assert!(!dag.has_cycle());
        assert!(dag.topo_sort().is_ok());
    }

    #[test]
    fn cycle_is_detected() {
        let mut dag = SemanticDAG::new();
        dag.add_node(ConceptNode::from_bitmask(1, "A", 1.0, &[]));
        dag.add_node(ConceptNode::from_bitmask(2, "B", 1.0, &[]));
        dag.add_edge(ConceptEdge {
            source: 1,
            target: 2,
            relationship: "x".into(),
        });
        dag.add_edge(ConceptEdge {
            source: 2,
            target: 1,
            relationship: "x".into(),
        });
        assert!(dag.has_cycle());
        assert!(matches!(dag.topo_sort(), Err(AuraError::CycleDetected)));
    }

    #[test]
    fn dag_round_trip() {
        let mut dag = SemanticDAG::new();
        dag.add_node(ConceptNode::from_bitmask(
            1,
            "Sky",
            0.99,
            &[1, 1, 0, 0, 1, 1, 0, 0],
        ));
        dag.add_node(ConceptNode::from_bitmask(
            2,
            "Person",
            0.9,
            &[0, 0, 1, 1, 0, 0, 1, 1],
        ));
        dag.add_edge(ConceptEdge {
            source: 2,
            target: 1,
            relationship: "is_in_front_of".into(),
        });
        let mut w = Writer::new();
        dag.encode(&mut w);
        let bytes = w.into_inner();
        let mut r = Reader::new(&bytes);
        let back = SemanticDAG::decode(&mut r).unwrap();
        assert_eq!(back, dag);
        // Bitmasks survive the round trip too.
        assert_eq!(back.nodes[0].bitmask().unwrap()[0], 1);
    }
}
