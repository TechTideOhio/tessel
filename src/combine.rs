use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::TesselError;
use crate::rng::Rng;
use crate::spec::{CoverageMode, Spec};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tuple {
    pub id: String,
    pub cells: BTreeMap<String, String>,
}

impl Tuple {
    #[must_use]
    pub fn compute_id(cells: &BTreeMap<String, String>) -> String {
        let mut h = Sha256::new();
        for (k, v) in cells {
            h.update(k.as_bytes());
            h.update(b"=");
            h.update(v.as_bytes());
            h.update(b";");
        }
        let hex = format!("{:x}", h.finalize());
        hex[..12].to_string()
    }

    #[must_use]
    pub fn new(cells: BTreeMap<String, String>) -> Self {
        let id = Self::compute_id(&cells);
        Self { id, cells }
    }

    #[must_use]
    pub fn satisfies_constraints(&self, spec: &Spec) -> bool {
        for c in &spec.constraints {
            let mut matches_all = true;
            for (dim, val) in &c.forbid {
                if self.cells.get(dim) != Some(val) {
                    matches_all = false;
                    break;
                }
            }
            if matches_all {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pair {
    pub dim_a: String,
    pub val_a: String,
    pub dim_b: String,
    pub val_b: String,
}

impl Pair {
    #[must_use]
    pub fn new(d1: &str, v1: &str, d2: &str, v2: &str) -> Self {
        if d1 <= d2 {
            Self {
                dim_a: d1.to_string(),
                val_a: v1.to_string(),
                dim_b: d2.to_string(),
                val_b: v2.to_string(),
            }
        } else {
            Self {
                dim_a: d2.to_string(),
                val_a: v2.to_string(),
                dim_b: d1.to_string(),
                val_b: v1.to_string(),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub spec_id: String,
    pub spec_version: String,
    pub mode: CoverageMode,
    pub total_tuples: usize,
    pub feasible_pairs: usize,
    pub covered_pairs: usize,
    pub coverage_pct: f64,
    pub tuples: Vec<Tuple>,
    pub infeasible_pairs: Vec<String>,
}

#[must_use]
pub fn enumerate_all_pairs(spec: &Spec) -> BTreeSet<Pair> {
    let mut pairs = BTreeSet::new();
    let dims = &spec.dimensions;
    for i in 0..dims.len() {
        for j in (i + 1)..dims.len() {
            let da = &dims[i];
            let db = &dims[j];
            for va in &da.values {
                for vb in &db.values {
                    pairs.insert(Pair::new(&da.name, &va.id, &db.name, &vb.id));
                }
            }
        }
    }
    pairs
}

#[must_use]
pub fn is_pair_feasible(pair: &Pair, spec: &Spec) -> bool {
    for c in &spec.constraints {
        let has_a = c.forbid.get(&pair.dim_a) == Some(&pair.val_a);
        let has_b = c.forbid.get(&pair.dim_b) == Some(&pair.val_b);
        if c.forbid.len() == 2 && has_a && has_b {
            return false;
        }
    }
    true
}

pub fn generate_plan(spec: &Spec) -> Result<Plan, TesselError> {
    match spec.spec.coverage {
        CoverageMode::Full => generate_full(spec),
        CoverageMode::Pairwise => generate_pairwise(spec),
        CoverageMode::Sample => generate_sample(spec),
    }
}

fn generate_full(spec: &Spec) -> Result<Plan, TesselError> {
    let mut total_combos: u128 = 1;
    for d in &spec.dimensions {
        total_combos = total_combos.saturating_mul(d.values.len() as u128);
    }
    const MAX_CAP: usize = 50_000;
    if total_combos > MAX_CAP as u128 {
        return Err(TesselError::TooLarge {
            count: total_combos,
            cap: MAX_CAP,
        });
    }

    let mut current = vec![BTreeMap::new()];
    for d in &spec.dimensions {
        let mut next = Vec::new();
        for existing in &current {
            for val in &d.values {
                let mut map = existing.clone();
                map.insert(d.name.clone(), val.id.clone());
                next.push(map);
            }
        }
        current = next;
    }

    let tuples: Vec<Tuple> = current
        .into_iter()
        .map(Tuple::new)
        .filter(|t| t.satisfies_constraints(spec))
        .collect();

    compute_plan_summary(spec, tuples, CoverageMode::Full)
}

fn generate_pairwise(spec: &Spec) -> Result<Plan, TesselError> {
    let all_pairs = enumerate_all_pairs(spec);
    let mut feasible_pairs = BTreeSet::new();
    let mut infeasible_pairs = Vec::new();

    for p in &all_pairs {
        if is_pair_feasible(p, spec) {
            feasible_pairs.insert(p.clone());
        } else {
            infeasible_pairs.push(format!(
                "{}={} with {}={}",
                p.dim_a, p.val_a, p.dim_b, p.val_b
            ));
        }
    }

    let mut uncovered = feasible_pairs.clone();
    let mut tuples: Vec<Tuple> = Vec::new();
    let mut rng = Rng::new(spec.spec.seed);

    let max_rounds = 50_000;
    let mut round = 0;

    while !uncovered.is_empty() && round < max_rounds {
        round += 1;

        let target_pair = match uncovered.iter().next() {
            Some(p) => p.clone(),
            None => break,
        };

        let mut best_cells = None;
        let mut best_cover_count = 0usize;

        const CANDIDATE_TRIES: usize = 40;
        for _ in 0..CANDIDATE_TRIES {
            let mut cells = BTreeMap::new();
            cells.insert(target_pair.dim_a.clone(), target_pair.val_a.clone());
            cells.insert(target_pair.dim_b.clone(), target_pair.val_b.clone());

            for dim in &spec.dimensions {
                if cells.contains_key(&dim.name) {
                    continue;
                }
                let weights: Vec<u32> = dim.values.iter().map(|v| v.weight).collect();
                let picked_idx = rng.weighted(&weights);
                cells.insert(dim.name.clone(), dim.values[picked_idx].id.clone());
            }

            let candidate = Tuple::new(cells.clone());
            if !candidate.satisfies_constraints(spec) {
                continue;
            }

            let mut count = 0;
            for (i, d1) in spec.dimensions.iter().enumerate() {
                for d2 in spec.dimensions.iter().skip(i + 1) {
                    if let (Some(v1), Some(v2)) = (cells.get(&d1.name), cells.get(&d2.name)) {
                        let p = Pair::new(&d1.name, v1, &d2.name, v2);
                        if uncovered.contains(&p) {
                            count += 1;
                        }
                    }
                }
            }

            if count > best_cover_count || best_cells.is_none() {
                best_cover_count = count;
                best_cells = Some(cells);
            }
        }

        if let Some(cells) = best_cells {
            let tuple = Tuple::new(cells);
            for (i, d1) in spec.dimensions.iter().enumerate() {
                for d2 in spec.dimensions.iter().skip(i + 1) {
                    if let (Some(v1), Some(v2)) =
                        (tuple.cells.get(&d1.name), tuple.cells.get(&d2.name))
                    {
                        let p = Pair::new(&d1.name, v1, &d2.name, v2);
                        uncovered.remove(&p);
                    }
                }
            }
            tuples.push(tuple);
        } else {
            uncovered.remove(&target_pair);
            infeasible_pairs.push(format!(
                "unreachable in search: {}={} with {}={}",
                target_pair.dim_a, target_pair.val_a, target_pair.dim_b, target_pair.val_b
            ));
        }
    }

    tuples.sort_by(|a, b| a.id.cmp(&b.id));

    compute_plan_summary(spec, tuples, CoverageMode::Pairwise)
}

fn generate_sample(spec: &Spec) -> Result<Plan, TesselError> {
    let mut tuples = Vec::new();
    let mut rng = Rng::new(spec.spec.seed);
    let sample_target = spec.spec.sample;

    let mut attempts = 0;
    while tuples.len() < sample_target && attempts < sample_target * 50 {
        attempts += 1;
        let mut cells = BTreeMap::new();
        for dim in &spec.dimensions {
            let weights: Vec<u32> = dim.values.iter().map(|v| v.weight).collect();
            let picked_idx = rng.weighted(&weights);
            cells.insert(dim.name.clone(), dim.values[picked_idx].id.clone());
        }
        let t = Tuple::new(cells);
        if t.satisfies_constraints(spec)
            && !tuples.iter().any(|existing: &Tuple| existing.id == t.id)
        {
            tuples.push(t);
        }
    }

    tuples.sort_by(|a, b| a.id.cmp(&b.id));
    compute_plan_summary(spec, tuples, CoverageMode::Sample)
}

fn compute_plan_summary(
    spec: &Spec,
    tuples: Vec<Tuple>,
    mode: CoverageMode,
) -> Result<Plan, TesselError> {
    let all_pairs = enumerate_all_pairs(spec);
    let mut feasible = 0;
    let mut infeasible = Vec::new();
    for p in &all_pairs {
        if is_pair_feasible(p, spec) {
            feasible += 1;
        } else {
            infeasible.push(format!(
                "{}={} with {}={}",
                p.dim_a, p.val_a, p.dim_b, p.val_b
            ));
        }
    }

    let mut covered = BTreeSet::new();
    for t in &tuples {
        for (i, d1) in spec.dimensions.iter().enumerate() {
            for d2 in spec.dimensions.iter().skip(i + 1) {
                if let (Some(v1), Some(v2)) = (t.cells.get(&d1.name), t.cells.get(&d2.name)) {
                    let p = Pair::new(&d1.name, v1, &d2.name, v2);
                    if is_pair_feasible(&p, spec) {
                        covered.insert(p);
                    }
                }
            }
        }
    }

    let covered_count = covered.len();
    let pct = if feasible > 0 {
        (covered_count as f64 / feasible as f64) * 100.0
    } else {
        100.0
    };

    Ok(Plan {
        spec_id: spec.spec.id.clone(),
        spec_version: spec.spec.version.clone(),
        mode,
        total_tuples: tuples.len(),
        feasible_pairs: feasible,
        covered_pairs: covered_count,
        coverage_pct: (pct * 10.0).round() / 10.0,
        tuples,
        infeasible_pairs: infeasible,
    })
}
