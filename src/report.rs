use std::collections::BTreeMap;

use serde::Serialize;

use crate::combine::{Plan, Tuple};
use crate::spec::Spec;

#[derive(Debug, Clone, Serialize)]
pub struct CoverageStats {
    pub spec_id: String,
    pub spec_version: String,
    pub tuple_count: usize,
    pub feasible_pairs: usize,
    pub covered_pairs: usize,
    pub coverage_pct: f64,
    pub dimension_counts: BTreeMap<String, usize>,
    pub pair_matrix: BTreeMap<String, BTreeMap<String, usize>>,
}

#[must_use]
pub fn compute_coverage_stats(spec: &Spec, tuples: &[Tuple], plan: &Plan) -> CoverageStats {
    let mut dim_counts = BTreeMap::new();
    for d in &spec.dimensions {
        dim_counts.insert(d.name.clone(), d.values.len());
    }

    let mut pair_counts: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for t in tuples {
        for (i, d1) in spec.dimensions.iter().enumerate() {
            for d2 in spec.dimensions.iter().skip(i + 1) {
                if let (Some(v1), Some(v2)) = (t.cells.get(&d1.name), t.cells.get(&d2.name)) {
                    let k1 = format!("{}={}", d1.name, v1);
                    let k2 = format!("{}={}", d2.name, v2);
                    *pair_counts.entry(k1).or_default().entry(k2).or_insert(0) += 1;
                }
            }
        }
    }

    CoverageStats {
        spec_id: spec.spec.id.clone(),
        spec_version: spec.spec.version.clone(),
        tuple_count: tuples.len(),
        feasible_pairs: plan.feasible_pairs,
        covered_pairs: plan.covered_pairs,
        coverage_pct: plan.coverage_pct,
        dimension_counts: dim_counts,
        pair_matrix: pair_counts,
    }
}

#[must_use]
pub fn render_mermaid_graph(spec: &Spec, plan: &Plan) -> String {
    let mut out = String::new();
    out.push_str("flowchart TD\n");
    out.push_str(&format!(
        "    subgraph Spec[\"Spec: {} v{}\"]\n",
        spec.spec.id, spec.spec.version
    ));

    for dim in &spec.dimensions {
        let dim_node = format!("dim_{}", dim.name.replace('-', "_"));
        out.push_str(&format!(
            "        {}[\"Dimension: {}\"]\n",
            dim_node, dim.name
        ));
        for val in &dim.values {
            let val_node = format!("{}_{}", dim_node, val.id.replace('-', "_"));
            out.push_str(&format!("        {}[\"{}\"]\n", val_node, val.id));
            out.push_str(&format!("        {} --> {}\n", dim_node, val_node));
        }
    }
    out.push_str("    end\n");

    out.push_str(&format!(
        "    Summary[\"Coverage: {:.1}% ({} tuples, {}/{} pairs)\"]\n",
        plan.coverage_pct, plan.total_tuples, plan.covered_pairs, plan.feasible_pairs
    ));

    out
}

#[must_use]
pub fn render_dot_graph(spec: &Spec, plan: &Plan) -> String {
    let mut out = String::new();
    out.push_str("digraph TesselCoverage {\n");
    out.push_str("    rankdir=LR;\n");
    out.push_str("    node [shape=box, fontname=\"Helvetica\"];\n");

    for dim in &spec.dimensions {
        let d_id = format!("dim_{}", dim.name.replace('-', "_"));
        out.push_str(&format!(
            "    {} [label=\"{}\", style=filled, fillcolor=\"#e0e7ff\"];\n",
            d_id, dim.name
        ));
        for val in &dim.values {
            let v_id = format!("{}_{}", d_id, val.id.replace('-', "_"));
            out.push_str(&format!("    {} [label=\"{}\"];\n", v_id, val.id));
            out.push_str(&format!("    {} -> {};\n", d_id, v_id));
        }
    }

    out.push_str(&format!(
        "    summary [label=\"Coverage: {:.1}%\\n{} tuples\", shape=ellipse, style=bold];\n",
        plan.coverage_pct, plan.total_tuples
    ));
    out.push_str("}\n");
    out
}
