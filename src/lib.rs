#![forbid(unsafe_code)]

pub mod combine;
pub mod envelope;
pub mod error;
pub mod export;
pub mod ledger;
pub mod realize;
pub mod report;
pub mod rng;
pub mod seed;
pub mod spec;

pub use combine::{enumerate_all_pairs, generate_plan, Pair, Plan, Tuple};
pub use envelope::{EnvelopeStatus, HarnessEnvelope};
pub use error::TesselError;
pub use export::export_reval_cases;
pub use ledger::{
    save_ledger, verify_ledger, FileDigest, LockFile, CASES_FILE, LOCK_FILE, TUPLES_FILE,
};
pub use realize::{
    check_near_duplicates, jaccard_similarity, realize_tuple, token_shingles, RealizeRequest,
    RealizedCase,
};
pub use report::{compute_coverage_stats, render_dot_graph, render_mermaid_graph, CoverageStats};
pub use rng::Rng;
pub use seed::{format_proposed_toml, harvest_seeds_from_jsonl, HarvestedSeed};
pub use spec::{
    CheckSpecTemplate, CheckTemplate, Constraint, CoverageMode, Dimension, Meta, Severity, Spec,
    Value,
};
