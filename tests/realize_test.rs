#![allow(clippy::unwrap_used, clippy::expect_used)]

use tessel::{jaccard_similarity, realize_tuple, token_shingles, Spec, Tuple};

#[test]
fn realizes_natural_prompt_with_attached_checks() {
    let toml = include_str!("../packs/softfactory.toml");
    let spec = Spec::from_toml(toml, "pack.toml").unwrap();

    let mut cells = std::collections::BTreeMap::new();
    cells.insert("persona".to_string(), "dispatched-worker".to_string());
    cells.insert("intent".to_string(), "fix".to_string());
    cells.insert("complexity".to_string(), "multi-file".to_string());
    cells.insert("edge_case".to_string(), "typecheck-build-order".to_string());
    cells.insert("target_stack".to_string(), "ts-pnpm".to_string());

    let tuple = Tuple::new(cells);
    let case = realize_tuple(&tuple, &spec, 357).expect("realize succeeds");

    assert!(case.prompt.contains("worker"));
    assert!(case.prompt.contains("typecheck"));
    assert_eq!(case.stratum, "core");
    assert!(!case.checks.is_empty());
}

#[test]
fn shingle_jaccard_distance_detects_clones() {
    let s1 = token_shingles("You are a worker fixing the contracts package build", 3);
    let s2 = token_shingles("You are a worker fixing the contracts package build", 3);
    let sim = jaccard_similarity(&s1, &s2);
    assert!((sim - 1.0).abs() < 1e-6);

    let s3 = token_shingles("Run full lint verification without modifying any files", 3);
    let sim_diff = jaccard_similarity(&s1, &s3);
    assert!(sim_diff < 0.2);
}
