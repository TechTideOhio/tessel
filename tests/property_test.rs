#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation
)]

use tessel::{generate_plan, Dimension, Meta, Rng, Spec, Value};

#[test]
fn property_pairwise_coverage_always_reaches_full_feasible_pairs() {
    let mut rng = Rng::new(0x7E55_E100_2026_1030);

    for run_idx in 0..50 {
        let num_dims = 3 + rng.below(3); // 3 to 5 dimensions
        let mut dimensions = Vec::new();

        for d_idx in 0..num_dims {
            let num_vals = 2 + rng.below(4); // 2 to 5 values per dimension
            let mut values = Vec::new();
            for v_idx in 0..num_vals {
                values.push(Value {
                    id: format!("v{v_idx}"),
                    weight: 1 + (rng.below(5) as u32),
                    hint: String::new(),
                    anchor: None,
                    checks: vec![],
                    source: None,
                    stratum: None,
                });
            }
            dimensions.push(Dimension {
                name: format!("dim{d_idx}"),
                values,
            });
        }

        let spec = Spec {
            spec: Meta {
                id: format!("prop-spec-{run_idx}"),
                version: "1.0.0".to_string(),
                seed: rng.next_u64(),
                coverage: tessel::CoverageMode::Pairwise,
                sample: 100,
                description: None,
            },
            dimensions,
            constraints: vec![],
        };

        let plan = generate_plan(&spec).expect("pairwise plan generation must succeed");
        assert_eq!(
            plan.covered_pairs, plan.feasible_pairs,
            "run {run_idx}: failed to cover all feasible pairs ({}/{})",
            plan.covered_pairs, plan.feasible_pairs
        );
        assert!((plan.coverage_pct - 100.0).abs() < 1e-4);
    }
}
