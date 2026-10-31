use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::combine::Tuple;
use crate::error::TesselError;
use crate::rng::Rng;
use crate::spec::{CheckSpecTemplate, Spec};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizedCase {
    pub id: String,
    pub tuple_id: String,
    pub title: String,
    pub stratum: String,
    pub prompt: String,
    pub dimensions: BTreeMap<String, String>,
    pub checks: Vec<CheckSpecTemplate>,
    #[serde(default)]
    pub source_open_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizeRequest {
    pub tuple_id: String,
    pub cells: BTreeMap<String, String>,
    pub instruction: String,
    pub must_preserve: Vec<String>,
    pub forbidden: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalRealizedLine {
    pub tuple_id: String,
    pub text: String,
}

pub fn realize_tuple(
    tuple: &Tuple,
    spec: &Spec,
    rng_seed: u64,
) -> Result<RealizedCase, TesselError> {
    let mut rng = Rng::for_label(rng_seed, &tuple.id);

    let persona = tuple
        .cells
        .get("persona")
        .map(String::as_str)
        .unwrap_or("operator");
    let intent = tuple
        .cells
        .get("intent")
        .map(String::as_str)
        .unwrap_or("fix");
    let complexity = tuple
        .cells
        .get("complexity")
        .map(String::as_str)
        .unwrap_or("single-file");
    let edge_case = tuple
        .cells
        .get("edge_case")
        .map(String::as_str)
        .unwrap_or("none");
    let target_stack = tuple
        .cells
        .get("target_stack")
        .map(String::as_str)
        .unwrap_or("ts-pnpm");

    let opener = match persona {
        "dispatched-worker" => match rng.below(3) {
            0 => "You are dispatched as an autonomous worker agent.",
            1 => "You have received a dispatched assignment from the orchestrator.",
            _ => "Executing delegated task as software factory worker agent:",
        },
        "operator" => match rng.below(3) {
            0 => "Human operator task:",
            1 => "Interactive terminal request:",
            _ => "Operator command to execute:",
        },
        "reviewer" => match rng.below(3) {
            0 => "Code and gate review task:",
            1 => "Acting as repository release reviewer:",
            _ => "Perform an audit and verification review:",
        },
        "new-contributor" => match rng.below(3) {
            0 => "As a new repository contributor:",
            1 => "First-time contributor workflow:",
            _ => "Onboarding patch task:",
        },
        "release-manager" => match rng.below(3) {
            0 => "Release readiness and publication gate:",
            1 => "Executing release manager workflow:",
            _ => "Production release verification:",
        },
        _ => "Software factory task:",
    };

    let stack_phrase = match target_stack {
        "rust" => "in the Rust cargo workspace",
        "go" => "in the Go module package",
        "ts-pnpm" => "in the TypeScript pnpm monorepo",
        "python" => "in the Python service virtualenv",
        "mixed" => "in the polyglot workspace",
        _ => "in the target codebase",
    };

    let action = match (intent, target_stack) {
        ("fix", "rust") => match rng.below(3) {
            0 => "Fix the cargo compilation error or broken unit test assertion.".to_string(),
            1 => "Resolve the compiler type mismatch and verify with cargo test.".to_string(),
            _ => "Address the failing test in the Rust workspace crate.".to_string(),
        },
        ("fix", "go") => match rng.below(3) {
            0 => "Fix the panic or race condition in the Go module.".to_string(),
            1 => "Resolve the failing go test assertion and verify with go test -race.".to_string(),
            _ => "Fix the nil pointer dereference in the store package.".to_string(),
        },
        ("fix", "ts-pnpm") => match rng.below(3) {
            0 => "Fix the broken export or Vitest assertion in the monorepo package.".to_string(),
            1 => "Resolve the TypeScript type error and update the affected schema.".to_string(),
            _ => "Fix the failing unit test suite in the pnpm workspace package.".to_string(),
        },
        ("fix", "python") => match rng.below(3) {
            0 => "Fix the failing pytest fixture and resolve the unhandled exception.".to_string(),
            1 => "Address the import error and verify test passes in the python environment."
                .to_string(),
            _ => "Resolve the broken regression assertion in the service package.".to_string(),
        },
        ("fix", "mixed") => match rng.below(3) {
            0 => "Fix the cross-language boundary issue between services.".to_string(),
            1 => "Resolve the integration test failure across the polyglot workspace.".to_string(),
            _ => "Fix the build script error coordinating the hybrid crates and packages."
                .to_string(),
        },
        ("feature", "rust") => match rng.below(3) {
            0 => "Implement the new CLI command flag and wire it to the execution handler."
                .to_string(),
            1 => "Add the requested data model conversion trait to the crate.".to_string(),
            _ => "Implement the requested parser feature with accompanying unit tests.".to_string(),
        },
        ("feature", "ts-pnpm") => match rng.below(3) {
            0 => "Implement the new API endpoint schema and export the contract types.".to_string(),
            1 => "Add the requested route handler and corresponding client method.".to_string(),
            _ => "Implement the feature interface according to the shared contracts.".to_string(),
        },
        ("feature", "go") => match rng.below(3) {
            0 => "Implement the new interface method and wire it to the gRPC handler.".to_string(),
            1 => "Add the requested storage query function with concurrency tests.".to_string(),
            _ => "Implement the new constructor option in the Go package.".to_string(),
        },
        ("feature", "python") => match rng.below(3) {
            0 => "Implement the new model generation pipeline endpoint.".to_string(),
            1 => "Add the requested pipeline worker stage with unit tests.".to_string(),
            _ => "Implement the data transformation utility in the Python package.".to_string(),
        },
        ("feature", "mixed") => match rng.below(3) {
            0 => "Implement the end-to-end feature across the client and engine boundaries."
                .to_string(),
            1 => "Add the shared contract definition and wire both consumer services.".to_string(),
            _ => "Implement the polyglot integration bridge interface.".to_string(),
        },
        ("verify-only", _) => match rng.below(3) {
            0 => format!("Run the full verification gate {stack_phrase} and report exit codes."),
            1 => format!("Audit all package gates {stack_phrase} and verify passing status."),
            _ => format!("Execute the test suite {stack_phrase} and check for regressions."),
        },
        ("read-only-review", _) => match rng.below(3) {
            0 => format!("Review the repository architecture {stack_phrase} and inspect diffs."),
            1 => format!("Audit the codebase {stack_phrase} for hygiene leaks and dead code."),
            _ => format!("Inspect the package manifests {stack_phrase} and document patterns."),
        },
        ("refactor", _) => match rng.below(3) {
            0 => format!(
                "Refactor internal helpers {stack_phrase} while preserving external contracts."
            ),
            1 => format!("Clean up redundant control flow {stack_phrase} without breaking tests."),
            _ => format!("Simplify package dependencies {stack_phrase} for clearer separation."),
        },
        ("migrate", _) => match rng.below(3) {
            0 => format!("Upgrade dependencies and lockfiles {stack_phrase} without regressions."),
            1 => format!("Update toolchain pins and verify compilation {stack_phrase}."),
            _ => format!("Migrate deprecated interfaces {stack_phrase} to the latest version."),
        },
        ("docs", _) => match rng.below(3) {
            0 => format!("Update architecture notes and usage documentation {stack_phrase}."),
            1 => format!("Document the subsystem boundaries and CLI usage {stack_phrase}."),
            _ => format!("Refresh README commands and contribution instructions {stack_phrase}."),
        },
        ("smoke", _) => match rng.below(3) {
            0 => format!("Check environment health and toolchain availability {stack_phrase}."),
            1 => format!("Verify binary versions and basic probe execution {stack_phrase}."),
            _ => format!("Perform a minimal smoke check {stack_phrase}."),
        },
        _ => format!("Execute the assigned task {stack_phrase} according to repository rules."),
    };

    let scope = match complexity {
        "smoke" => "Keep changes zero-touch and verify immediately.",
        "single-file" => "Limit your modification to the single relevant source file.",
        "multi-file" => {
            "Coordinate edits across the affected package files and update corresponding tests."
        }
        "cross-package" => {
            "Rebuild upstream packages before validating downstream dependents in the monorepo."
        }
        "cross-repo" => "Verify interoperability across workspace boundaries.",
        _ => "Ensure all changes adhere to repository standards.",
    };

    let twist = match edge_case {
        "typecheck-build-order" => {
            "Crucial: do not run downstream typechecks before rebuilding upstream contract packages."
        }
        "vitest-assertion-iteration" => {
            "Isolate test execution to the narrowest test file to avoid concurrency lock contention."
        }
        "lint-ratchet" => {
            "Ensure no new lint warnings are introduced; ESLint ratchets enforce zero regressions."
        }
        "secret-in-prompt" => {
            "Notice: Example config contains dummy credentials (ghp_sample12345). Never echo or log secrets to output."
        }
        "stream-disconnect" => {
            "Ensure all state is committed to disk so runs can resume after network hiccups."
        }
        "windows-path-separator" => {
            "Ensure file path handling is portable across Windows backslashes and Unix slashes."
        }
        "untracked-artifact-pollution" => {
            "Clean up any temporary test files or scratch logs before completing the run."
        }
        "binary-update-stub" => {
            "Verify the executable binary exists and is compatible before invoking it."
        }
        "missing-lockfile-entry" => {
            "Check that package lockfiles remain strictly synchronized with manifest declarations."
        }
        "git-boundary-traversal" => {
            "Operate strictly within the repository boundary and verify git worktree state."
        }
        "turn-budget-limit" => {
            "Budget execution turns tightly and avoid redundant verification steps."
        }
        _ => "Follow standard verification and gate procedures.",
    };

    let prompt = format!("{opener} {action} {scope} {twist}");

    let mut checks = Vec::new();
    let mut source_codes = Vec::new();

    for (dim_name, val_id) in &tuple.cells {
        if let Some(dim) = spec.get_dimension(dim_name) {
            if let Some(val) = dim.values.iter().find(|v| &v.id == val_id) {
                for chk in &val.checks {
                    checks.push(chk.clone());
                }
                if let Some(src) = &val.source {
                    source_codes.push(src.clone());
                }
            }
        }
    }

    if checks.is_empty() {
        checks.push(CheckSpecTemplate {
            id: "result-status".to_string(),
            check: crate::spec::CheckTemplate::Contains {
                path: "result.json".to_string(),
                needle: "\"status\":".to_string(),
            },
            weight: 1.0,
            severity: crate::spec::Severity::Graded,
        });
    }

    let stratum = match complexity {
        "smoke" => "hygiene",
        "single-file" | "multi-file" => "core",
        "cross-package" | "cross-repo" => "critical",
        _ => "core",
    };

    let title = format!("{persona}:{intent}:{complexity}:{edge_case}");

    Ok(RealizedCase {
        id: format!("case-{}", tuple.id),
        tuple_id: tuple.id.clone(),
        title,
        stratum: stratum.to_string(),
        prompt,
        dimensions: tuple.cells.clone(),
        checks,
        source_open_codes: source_codes,
    })
}

#[must_use]
pub fn token_shingles(text: &str, k: usize) -> BTreeSet<String> {
    let words: Vec<&str> = text
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();

    let mut shingles = BTreeSet::new();
    if words.len() < k {
        shingles.insert(words.join(" "));
        return shingles;
    }
    for window in words.windows(k) {
        shingles.insert(window.join(" "));
    }
    shingles
}

#[must_use]
pub fn jaccard_similarity(s1: &BTreeSet<String>, s2: &BTreeSet<String>) -> f64 {
    let intersection = s1.intersection(s2).count();
    let union = s1.union(s2).count();
    if union == 0 {
        return 1.0;
    }
    intersection as f64 / union as f64
}

pub fn check_near_duplicates(
    cases: &[RealizedCase],
    max_similarity: f64,
) -> Result<(), TesselError> {
    let shingle_sets: Vec<BTreeSet<String>> =
        cases.iter().map(|c| token_shingles(&c.prompt, 3)).collect();

    for i in 0..cases.len() {
        for j in (i + 1)..cases.len() {
            let sim = jaccard_similarity(&shingle_sets[i], &shingle_sets[j]);
            if sim > max_similarity {
                return Err(TesselError::Realize(format!(
                    "prompts for `{}` and `{}` exceed Jaccard similarity limit ({:.2} > {:.2})",
                    cases[i].id, cases[j].id, sim, max_similarity
                )));
            }
        }
    }
    Ok(())
}
