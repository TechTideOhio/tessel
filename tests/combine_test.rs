#![allow(clippy::unwrap_used, clippy::expect_used)]

use tessel::{generate_plan, Spec};

#[test]
fn pairwise_achieves_complete_coverage() {
    let toml = include_str!("../packs/softfactory.toml");
    let spec = Spec::from_toml(toml, "packs/softfactory.toml").expect("valid pack");
    let plan = generate_plan(&spec).expect("planning must succeed");

    assert!(
        plan.coverage_pct >= 99.0,
        "coverage was {:.1}%",
        plan.coverage_pct
    );
    assert_eq!(plan.covered_pairs, plan.feasible_pairs);
    // 5 * 8 * 5 * 12 * 5 = 12,000 full combos
    // pairwise should be roughly ~90-150 tuples
    assert!(plan.total_tuples < 300, "tuples: {}", plan.total_tuples);
    assert!(plan.total_tuples > 50, "tuples: {}", plan.total_tuples);
}

#[test]
fn deterministic_output_across_runs() {
    let toml = include_str!("../packs/softfactory.toml");
    let spec1 = Spec::from_toml(toml, "pack.toml").unwrap();
    let spec2 = Spec::from_toml(toml, "pack.toml").unwrap();

    let plan1 = generate_plan(&spec1).unwrap();
    let plan2 = generate_plan(&spec2).unwrap();

    assert_eq!(plan1.total_tuples, plan2.total_tuples);
    for (t1, t2) in plan1.tuples.iter().zip(plan2.tuples.iter()) {
        assert_eq!(t1.id, t2.id);
        assert_eq!(t1.cells, t2.cells);
    }
}

#[test]
fn respects_forbid_constraints() {
    let toml = include_str!("../packs/softfactory.toml");
    let spec = Spec::from_toml(toml, "pack.toml").unwrap();
    let plan = generate_plan(&spec).unwrap();

    for t in &plan.tuples {
        assert!(t.satisfies_constraints(&spec));
        if t.cells.get("complexity").map(String::as_str) == Some("smoke") {
            assert_ne!(
                t.cells.get("edge_case").map(String::as_str),
                Some("typecheck-build-order")
            );
        }
    }
}
