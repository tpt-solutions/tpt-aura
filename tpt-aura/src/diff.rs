//! Semantic/structural diffing between two AURA files.
//!
//! [`diff`] compares the parsed sections of two [`AuraFile`]s and reports what
//! changed between them: the Semantic DAG (concept nodes and edges), the record
//! layout, and the provenance ledger. It powers the `aura diff` CLI command and
//! is also usable as a library for tooling that tracks how a master file evolves
//! across edits.

use crate::container::AuraFile;
use crate::provenance::OpType;
use crate::semantic::SemanticDAG;
use serde::Serialize;
use std::collections::BTreeMap;

/// A change to a single concept node's confidence (label/bitmask may also
/// differ; confidence is the most useful single scalar to surface).
#[derive(Debug, Clone, Serialize)]
pub struct NodeChange {
    /// Node id (stable across edits).
    pub id: u32,
    /// Concept label.
    pub label: String,
    /// Confidence in file A.
    pub old_confidence: f32,
    /// Confidence in file B.
    pub new_confidence: f32,
}

/// Diff of the Semantic DAG between two files.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DagDiff {
    /// Concept nodes present only in B.
    pub nodes_added: Vec<(u32, String)>,
    /// Concept nodes present only in A.
    pub nodes_removed: Vec<(u32, String)>,
    /// Nodes present in both but with different confidence.
    pub nodes_changed: Vec<NodeChange>,
    /// Edges present only in B.
    pub edges_added: Vec<(u32, u32, String)>,
    /// Edges present only in A.
    pub edges_removed: Vec<(u32, u32, String)>,
}

/// Per-record-type count difference.
#[derive(Debug, Clone, Serialize)]
pub struct RecordTypeDiff {
    /// Record type tag.
    pub type_tag: u8,
    /// Count in file A.
    pub a: usize,
    /// Count in file B.
    pub b: usize,
}

/// Diff of the polymorphic record layout.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RecordsDiff {
    /// Per-type-tag counts that differ between the files.
    pub by_type: Vec<RecordTypeDiff>,
}

/// Diff of the provenance ledger (append-only edit history).
#[derive(Debug, Clone, Default, Serialize)]
pub struct LedgerDiff {
    /// Number of entries added in B relative to A.
    pub entries_added: usize,
    /// Number of entries removed in B relative to A.
    pub entries_removed: usize,
    /// Operation types of the added entries (for a quick human summary).
    pub added_ops: Vec<String>,
}

/// A full structural diff between two AURA files.
#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    /// `(major, minor)` version of file A.
    pub version_a: (u16, u16),
    /// `(major, minor)` version of file B.
    pub version_b: (u16, u16),
    /// Semantic DAG differences.
    pub dag: DagDiff,
    /// Record layout differences.
    pub records: RecordsDiff,
    /// Provenance ledger differences.
    pub ledger: LedgerDiff,
    /// Whether the embedded WASM bootstrap blob changed.
    pub bootstrap_changed: bool,
}

/// Compare two parsed AURA files and return a structured [`DiffReport`].
pub fn diff(a: &AuraFile, b: &AuraFile) -> DiffReport {
    DiffReport {
        version_a: (a.header.version_major, a.header.version_minor),
        version_b: (b.header.version_major, b.header.version_minor),
        dag: diff_dag(&a.dag, &b.dag),
        records: diff_records(a, b),
        ledger: diff_ledger(a, b),
        bootstrap_changed: a.bootstrap.bytes != b.bootstrap.bytes,
    }
}

fn diff_dag(a: &SemanticDAG, b: &SemanticDAG) -> DagDiff {
    let mut out = DagDiff::default();

    let mut a_by_id: BTreeMap<u32, &crate::semantic::ConceptNode> = BTreeMap::new();
    for n in &a.nodes {
        a_by_id.insert(n.id, n);
    }
    let mut b_by_id: BTreeMap<u32, &crate::semantic::ConceptNode> = BTreeMap::new();
    for n in &b.nodes {
        b_by_id.insert(n.id, n);
    }

    for (id, node) in &b_by_id {
        match a_by_id.get(id) {
            None => out.nodes_added.push((*id, node.label.clone())),
            Some(old) => {
                if (old.confidence - node.confidence).abs() > f32::EPSILON
                    || old.label != node.label
                    || old.bitmask_rle != node.bitmask_rle
                {
                    out.nodes_changed.push(NodeChange {
                        id: *id,
                        label: node.label.clone(),
                        old_confidence: old.confidence,
                        new_confidence: node.confidence,
                    });
                }
            }
        }
    }
    for (id, node) in &a_by_id {
        if !b_by_id.contains_key(id) {
            out.nodes_removed.push((*id, node.label.clone()));
        }
    }

    let a_edges: std::collections::HashSet<(u32, u32, String)> = a
        .edges
        .iter()
        .map(|e| (e.source, e.target, e.relationship.clone()))
        .collect();
    let b_edges: std::collections::HashSet<(u32, u32, String)> = b
        .edges
        .iter()
        .map(|e| (e.source, e.target, e.relationship.clone()))
        .collect();
    for e in &b_edges {
        if !a_edges.contains(e) {
            out.edges_added.push(e.clone());
        }
    }
    for e in &a_edges {
        if !b_edges.contains(e) {
            out.edges_removed.push(e.clone());
        }
    }

    out
}

fn count_records(file: &AuraFile) -> BTreeMap<u8, usize> {
    let mut counts = BTreeMap::new();
    for r in &file.scene.children {
        *counts.entry(r.type_tag()).or_insert(0) += 1;
    }
    counts
}

fn diff_records(a: &AuraFile, b: &AuraFile) -> RecordsDiff {
    let ca = count_records(a);
    let cb = count_records(b);
    let mut tags: Vec<u8> = ca.keys().chain(cb.keys()).copied().collect();
    tags.sort_unstable();
    tags.dedup();
    let mut by_type = Vec::new();
    for t in tags {
        let na = ca.get(&t).copied().unwrap_or(0);
        let nb = cb.get(&t).copied().unwrap_or(0);
        if na != nb {
            by_type.push(RecordTypeDiff {
                type_tag: t,
                a: na,
                b: nb,
            });
        }
    }
    RecordsDiff { by_type }
}

fn ledger_key(op: OpType, software: &str, hash: &[u8; 32]) -> String {
    format!("{:?}:{}:{}", op, software, hex(hash))
}

fn diff_ledger(a: &AuraFile, b: &AuraFile) -> LedgerDiff {
    let set_a: std::collections::HashSet<String> = a
        .ledger
        .entries
        .iter()
        .map(|e| ledger_key(e.op, &e.software, &e.resulting_hash))
        .collect();
    let set_b: std::collections::HashSet<String> = b
        .ledger
        .entries
        .iter()
        .map(|e| ledger_key(e.op, &e.software, &e.resulting_hash))
        .collect();

    let mut added_ops = Vec::new();
    for e in &b.ledger.entries {
        let k = ledger_key(e.op, &e.software, &e.resulting_hash);
        if !set_a.contains(&k) {
            added_ops.push(format!("{:?}", e.op));
        }
    }
    let entries_removed = set_a.difference(&set_b).count();

    LedgerDiff {
        entries_added: b.ledger.entries.len().saturating_sub(set_a.len()),
        entries_removed,
        added_ops,
    }
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

impl std::fmt::Display for DiffReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "AURA diff  (v{}.{} -> v{}.{})",
            self.version_a.0, self.version_a.1, self.version_b.0, self.version_b.1
        )?;
        if self.bootstrap_changed {
            writeln!(f, "  bootstrap: CHANGED")?;
        }

        let d = &self.dag;
        writeln!(f, "  semantic DAG:")?;
        if d.nodes_added.is_empty() && d.nodes_removed.is_empty() && d.nodes_changed.is_empty() {
            writeln!(f, "    (no node changes)")?;
        }
        for (id, label) in &d.nodes_added {
            writeln!(f, "    + node {id} \"{label}\"")?;
        }
        for (id, label) in &d.nodes_removed {
            writeln!(f, "    - node {id} \"{label}\"")?;
        }
        for c in &d.nodes_changed {
            writeln!(
                f,
                "    ~ node {} \"{}\" conf {:.2} -> {:.2}",
                c.id, c.label, c.old_confidence, c.new_confidence
            )?;
        }
        for (s, t, r) in &d.edges_added {
            writeln!(f, "    + edge {s} ->({r})-> {t}")?;
        }
        for (s, t, r) in &d.edges_removed {
            writeln!(f, "    - edge {s} ->({r})-> {t}")?;
        }

        writeln!(f, "  records:")?;
        if self.records.by_type.is_empty() {
            writeln!(f, "    (no layout changes)")?;
        }
        for rt in &self.records.by_type {
            writeln!(f, "    ~ type 0x{:02x}: {} -> {}", rt.type_tag, rt.a, rt.b)?;
        }

        writeln!(f, "  ledger:")?;
        if self.ledger.entries_added == 0 && self.ledger.entries_removed == 0 {
            writeln!(f, "    (no ledger changes)")?;
        } else {
            writeln!(
                f,
                "    + {} entries (- {} removed): {:?}",
                self.ledger.entries_added, self.ledger.entries_removed, self.ledger.added_ops
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::Bootstrap;
    use crate::container::{open, AuraBuilder, AuraFile, SceneRecord};
    use crate::provenance::{sha3_256, GenesisBlock, OpType, ProvenanceLedger};
    use crate::semantic::{ConceptNode, SemanticDAG};
    use ed25519_dalek::SigningKey;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn file_with(dag: SemanticDAG, ledger_entries: usize) -> AuraFile {
        let k = key();
        let h = sha3_256(b"x");
        let genesis = GenesisBlock::sign(&k, h, [1u8; 16], 0);
        let mut ledger = ProvenanceLedger::new(&k, h);
        for _ in 0..ledger_entries {
            ledger.append(OpType::Other, "test", &k).unwrap();
        }
        let scene = SceneRecord::new();
        let bytes = AuraBuilder::new(Bootstrap::with_default_wasm(), genesis, scene, dag, ledger)
            .build()
            .unwrap();
        open(&bytes).unwrap()
    }

    #[test]
    fn detects_added_node() {
        let mut a = SemanticDAG::new();
        a.add_node(ConceptNode::from_bitmask(1, "Sky", 0.9, &[]));
        let mut b = SemanticDAG::new();
        b.add_node(ConceptNode::from_bitmask(1, "Sky", 0.9, &[]));
        b.add_node(ConceptNode::from_bitmask(2, "Ground", 0.8, &[]));

        let r = diff(&file_with(a, 0), &file_with(b, 0));
        assert_eq!(r.dag.nodes_added, vec![(2, "Ground".to_string())]);
        assert!(r.dag.nodes_removed.is_empty());
        assert!(r.dag.nodes_changed.is_empty());
    }

    #[test]
    fn detects_removed_node() {
        let mut a = SemanticDAG::new();
        a.add_node(ConceptNode::from_bitmask(1, "Sky", 0.9, &[]));
        a.add_node(ConceptNode::from_bitmask(2, "Ground", 0.8, &[]));
        let b = SemanticDAG::new();

        let r = diff(&file_with(a, 0), &file_with(b, 0));
        assert_eq!(
            r.dag.nodes_removed,
            vec![(1, "Sky".to_string()), (2, "Ground".to_string())]
        );
    }

    #[test]
    fn detects_confidence_change() {
        let mut a = SemanticDAG::new();
        a.add_node(ConceptNode::from_bitmask(1, "Sky", 0.5, &[]));
        let mut b = SemanticDAG::new();
        b.add_node(ConceptNode::from_bitmask(1, "Sky", 0.9, &[]));

        let r = diff(&file_with(a, 0), &file_with(b, 0));
        assert_eq!(r.dag.nodes_changed.len(), 1);
        assert!((r.dag.nodes_changed[0].new_confidence - 0.9).abs() < 1e-6);
    }

    #[test]
    fn detects_ledger_additions() {
        let r = diff(
            &file_with(SemanticDAG::new(), 0),
            &file_with(SemanticDAG::new(), 2),
        );
        assert_eq!(r.ledger.entries_added, 2);
        assert_eq!(
            r.ledger.added_ops,
            vec!["Other".to_string(), "Other".to_string()]
        );
    }

    #[test]
    fn identical_files_have_empty_diff() {
        let r = diff(
            &file_with(SemanticDAG::new(), 1),
            &file_with(SemanticDAG::new(), 1),
        );
        assert!(r.dag.nodes_added.is_empty());
        assert!(r.dag.nodes_removed.is_empty());
        assert!(r.dag.nodes_changed.is_empty());
        assert!(r.records.by_type.is_empty());
        assert_eq!(r.ledger.entries_added, 0);
        assert!(!r.bootstrap_changed);
    }
}
