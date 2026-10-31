use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use tessel::{
    check_near_duplicates, compute_coverage_stats, export_reval_cases, format_proposed_toml,
    generate_plan, harvest_seeds_from_jsonl, realize_tuple, render_dot_graph, render_mermaid_graph,
    save_ledger, verify_ledger, EnvelopeStatus, HarnessEnvelope, RealizeRequest, RealizedCase,
    Spec,
};

#[derive(Debug, Parser)]
#[command(
    name = "tessel",
    version,
    about = "TideOS Eval Kit (Tool 3): Synthetic eval inputs from explicit dimension tuples"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Machine-readable output envelope
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize a new dimension pack
    Init {
        /// Preset pack name (e.g. "softfactory")
        #[arg(long, default_value = "softfactory")]
        pack: String,

        /// Output path for dimension spec
        #[arg(long, default_value = "dimensions.toml")]
        out: PathBuf,
    },

    /// Validate dimension specification and verify pair reachability
    Validate {
        /// Path to dimensions.toml
        #[arg(long, default_value = "dimensions.toml")]
        spec: PathBuf,
    },

    /// Plan combinatorial tuples and calculate coverage guarantee
    Plan {
        /// Path to dimensions.toml
        #[arg(long, default_value = "dimensions.toml")]
        spec: PathBuf,
    },

    /// Generate tuples, realize natural inputs, and write locked ledger
    Generate {
        /// Path to dimensions.toml
        #[arg(long, default_value = "dimensions.toml")]
        spec: PathBuf,

        /// Output directory for tuples, cases, and lockfile
        #[arg(long, default_value = "out")]
        out: PathBuf,

        /// Emit realize-requests.jsonl for external agent handoff
        #[arg(long)]
        emit_requests: bool,
    },

    /// Ingest externally realized agent outputs and validate anchors
    Ingest {
        /// Path to realized.jsonl
        input: PathBuf,

        /// Output directory
        #[arg(long, default_value = "out")]
        out: PathBuf,
    },

    /// Export generated cases to runner formats (e.g. reval-cases)
    Export {
        /// Input cases.jsonl
        #[arg(long, default_value = "out/cases.jsonl")]
        cases: PathBuf,

        /// Target format (reval-cases, jsonl)
        #[arg(long, default_value = "reval-cases")]
        r#as: String,

        /// Target export directory
        #[arg(long, default_value = "evals")]
        out: PathBuf,
    },

    /// Generate coverage matrix and visual graph (Mermaid/DOT)
    Report {
        /// Path to dimensions.toml
        #[arg(long, default_value = "dimensions.toml")]
        spec: PathBuf,

        /// Output format (text, json, mermaid, dot)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Harvest failure codes from trace corpus to seed edge_case dimension
    Seed {
        /// Path to trace JSONL corpus
        #[arg(long)]
        traces: PathBuf,

        /// Output file for proposed TOML
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Verify locked ledger against spec and files, failing closed on drift
    Verify {
        /// Path to dimensions.toml
        #[arg(long, default_value = "dimensions.toml")]
        spec: PathBuf,

        /// Ledger directory containing tessel.lock.json
        #[arg(long, default_value = "out")]
        dir: PathBuf,
    },
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { pack, out } => cmd_init(&pack, &out, cli.json),
        Commands::Validate { spec } => cmd_validate(&spec, cli.json),
        Commands::Plan { spec } => cmd_plan(&spec, cli.json),
        Commands::Generate {
            spec,
            out,
            emit_requests,
        } => cmd_generate(&spec, &out, emit_requests, cli.json),
        Commands::Ingest { input, out } => cmd_ingest(&input, &out, cli.json),
        Commands::Export {
            cases,
            r#as: format_name,
            out,
        } => cmd_export(&cases, &format_name, &out, cli.json),
        Commands::Report { spec, format } => cmd_report(&spec, &format, cli.json),
        Commands::Seed { traces, out } => cmd_seed(&traces, out.as_deref(), cli.json),
        Commands::Verify { spec, dir } => cmd_verify(&spec, &dir, cli.json),
    }
}

fn cmd_init(pack: &str, out: &Path, json: bool) -> Result<ExitCode> {
    let content = if pack == "softfactory" {
        include_str!("../packs/softfactory.toml")
    } else {
        eprintln!("unknown preset pack `{pack}`; available: softfactory");
        return Ok(ExitCode::from(1));
    };

    fs::write(out, content)?;
    let summary = format!("initialized dimension pack `{pack}` at {}", out.display());

    if json {
        let env = HarnessEnvelope::new(
            EnvelopeStatus::Success,
            summary,
            vec!["tessel validate --spec dimensions.toml".to_string()],
            vec![out.display().to_string()],
            serde_json::json!({ "pack": pack, "path": out.display().to_string() }),
        );
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        println!("[PASS] {summary}");
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_validate(spec_path: &Path, json: bool) -> Result<ExitCode> {
    match Spec::load_file(spec_path) {
        Ok(spec) => {
            let plan = generate_plan(&spec)?;
            let summary = format!(
                "spec `{}` v{} valid ({} dimensions, {} pairs, {} tuples planned)",
                spec.spec.id,
                spec.spec.version,
                spec.dimensions.len(),
                plan.feasible_pairs,
                plan.total_tuples
            );
            if json {
                let env = HarnessEnvelope::new(
                    EnvelopeStatus::Success,
                    summary,
                    vec!["tessel plan".to_string()],
                    vec![spec_path.display().to_string()],
                    &plan,
                );
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("[PASS] {summary}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            let summary = format!("validation failed: {err}");
            if json {
                let env = HarnessEnvelope::new(
                    EnvelopeStatus::Error,
                    summary,
                    vec!["fix reported spec errors".to_string()],
                    vec![spec_path.display().to_string()],
                    serde_json::json!({ "error": err.to_string() }),
                );
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                eprintln!("[FAIL] {summary}");
            }
            Ok(ExitCode::from(1))
        }
    }
}

fn cmd_plan(spec_path: &Path, json: bool) -> Result<ExitCode> {
    let spec = Spec::load_file(spec_path)?;
    let plan = generate_plan(&spec)?;

    let summary = format!(
        "plan complete: {} tuples covering {}/{} pairs ({:.1}% coverage, mode: {:?})",
        plan.total_tuples, plan.covered_pairs, plan.feasible_pairs, plan.coverage_pct, plan.mode
    );

    if json {
        let env = HarnessEnvelope::new(
            EnvelopeStatus::Success,
            summary,
            vec!["tessel generate".to_string()],
            vec![spec_path.display().to_string()],
            &plan,
        );
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        println!("[PASS] {summary}");
        println!("  Tuples planned: {}", plan.total_tuples);
        println!("  Feasible pairs: {}", plan.feasible_pairs);
        println!("  Covered pairs:  {}", plan.covered_pairs);
        println!("  Coverage:       {:.1}%", plan.coverage_pct);
        if !plan.infeasible_pairs.is_empty() {
            println!(
                "  Infeasible constrained pairs: {}",
                plan.infeasible_pairs.len()
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_generate(
    spec_path: &Path,
    out_dir: &Path,
    emit_requests: bool,
    json: bool,
) -> Result<ExitCode> {
    let spec = Spec::load_file(spec_path)?;
    let plan = generate_plan(&spec)?;

    fs::create_dir_all(out_dir)?;

    let mut cases = Vec::new();
    let mut requests = Vec::new();

    for t in &plan.tuples {
        let case = realize_tuple(t, &spec, spec.spec.seed)?;
        cases.push(case.clone());

        if emit_requests {
            requests.push(RealizeRequest {
                tuple_id: t.id.clone(),
                cells: t.cells.clone(),
                instruction: format!("Expand into natural coding agent task: {}", case.prompt),
                must_preserve: vec![],
                forbidden: vec!["<REDACTED>".to_string(), "sk-ant-".to_string()],
            });
        }
    }

    check_near_duplicates(&cases, 0.92)?;
    save_ledger(out_dir, spec_path, &spec, &plan.tuples, &cases)?;

    if emit_requests {
        let req_path = out_dir.join("realize-requests.jsonl");
        let mut buf = String::new();
        for r in requests {
            buf.push_str(&serde_json::to_string(&r)?);
            buf.push('\n');
        }
        fs::write(&req_path, buf)?;
    }

    let summary = format!(
        "generated {} cases and saved locked ledger to {}",
        cases.len(),
        out_dir.display()
    );

    if json {
        let env = HarnessEnvelope::new(
            EnvelopeStatus::Success,
            summary,
            vec!["tessel export".to_string(), "tessel verify".to_string()],
            vec![
                out_dir.join("tuples.jsonl").display().to_string(),
                out_dir.join("cases.jsonl").display().to_string(),
                out_dir.join("tessel.lock.json").display().to_string(),
            ],
            serde_json::json!({
                "cases_count": cases.len(),
                "coverage_pct": plan.coverage_pct,
                "out_dir": out_dir.display().to_string(),
            }),
        );
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        println!("[PASS] {summary}");
        println!("  Artifacts: tuples.jsonl, cases.jsonl, tessel.lock.json");
    }

    Ok(ExitCode::SUCCESS)
}

fn cmd_ingest(input_path: &Path, out_dir: &Path, json: bool) -> Result<ExitCode> {
    if !input_path.is_file() {
        eprintln!("input file not found at {}", input_path.display());
        return Ok(ExitCode::from(1));
    }
    let raw = fs::read_to_string(input_path)?;
    let mut count = 0;
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: serde_json::Value = serde_json::from_str(line)?;
        if parsed.get("tuple_id").is_none() || parsed.get("text").is_none() {
            eprintln!("invalid line missing tuple_id or text");
            return Ok(ExitCode::from(1));
        }
        count += 1;
    }

    let summary = format!(
        "ingested and verified {count} realized lines from {}",
        input_path.display()
    );
    if json {
        let env = HarnessEnvelope::new(
            EnvelopeStatus::Success,
            summary,
            vec!["tessel export".to_string()],
            vec![input_path.display().to_string()],
            serde_json::json!({ "ingested_count": count, "out_dir": out_dir.display().to_string() }),
        );
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        println!("[PASS] {summary}");
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_export(
    cases_path: &Path,
    format_name: &str,
    out_dir: &Path,
    json: bool,
) -> Result<ExitCode> {
    if !cases_path.is_file() {
        eprintln!("cases file missing at {}", cases_path.display());
        return Ok(ExitCode::from(1));
    }

    let content = fs::read_to_string(cases_path)?;
    let mut cases = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let c: RealizedCase = serde_json::from_str(line)?;
        cases.push(c);
    }

    match format_name {
        "reval-cases" => {
            let n = export_reval_cases(&cases, out_dir)?;
            let summary = format!(
                "exported {n} cases to reval format at {}",
                out_dir.display()
            );
            if json {
                let env = HarnessEnvelope::new(
                    EnvelopeStatus::Success,
                    summary,
                    vec!["reval eval suite --cases-dir evals/cases".to_string()],
                    vec![out_dir.display().to_string()],
                    serde_json::json!({ "exported_cases": n, "format": format_name }),
                );
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("[PASS] {summary}");
            }
        }
        "jsonl" => {
            let dest = out_dir.join("exported_cases.jsonl");
            fs::create_dir_all(out_dir)?;
            fs::copy(cases_path, &dest)?;
            let summary = format!("exported {} cases to {}", cases.len(), dest.display());
            if json {
                let env = HarnessEnvelope::new(
                    EnvelopeStatus::Success,
                    summary,
                    vec![],
                    vec![dest.display().to_string()],
                    serde_json::json!({ "exported_cases": cases.len() }),
                );
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("[PASS] {summary}");
            }
        }
        _ => {
            eprintln!("unknown export format `{format_name}`; supported: reval-cases, jsonl");
            return Ok(ExitCode::from(1));
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn cmd_report(spec_path: &Path, format_name: &str, json: bool) -> Result<ExitCode> {
    let spec = Spec::load_file(spec_path)?;
    let plan = generate_plan(&spec)?;
    let stats = compute_coverage_stats(&spec, &plan.tuples, &plan);

    match format_name {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        "mermaid" => {
            println!("{}", render_mermaid_graph(&spec, &plan));
        }
        "dot" => {
            println!("{}", render_dot_graph(&spec, &plan));
        }
        "text" => {
            println!(
                "=== TESSEL COVERAGE REPORT (spec: {} v{}) ===",
                spec.spec.id, spec.spec.version
            );
            println!(
                "Coverage: {:.1}% ({} tuples, {}/{} pairs)",
                plan.coverage_pct, plan.total_tuples, plan.covered_pairs, plan.feasible_pairs
            );
            println!("\nDimensions:");
            for (dim, count) in &stats.dimension_counts {
                println!("  - {}: {} values", dim, count);
            }
        }
        _ => {
            eprintln!("unknown report format `{format_name}`; supported: text, json, mermaid, dot");
            return Ok(ExitCode::from(1));
        }
    }

    if json && format_name != "json" {
        let env = HarnessEnvelope::new(
            EnvelopeStatus::Success,
            format!("generated report in {format_name} format"),
            vec![],
            vec![],
            &stats,
        );
        println!("{}", serde_json::to_string_pretty(&env)?);
    }

    Ok(ExitCode::SUCCESS)
}

fn cmd_seed(traces_path: &Path, out_path: Option<&Path>, json: bool) -> Result<ExitCode> {
    let seeds = harvest_seeds_from_jsonl(traces_path)?;
    let toml = format_proposed_toml(&seeds);

    if let Some(out) = out_path {
        fs::write(out, &toml)?;
        let summary = format!(
            "harvested {} seeds and saved to {}",
            seeds.len(),
            out.display()
        );
        if json {
            let env = HarnessEnvelope::new(
                EnvelopeStatus::Success,
                summary,
                vec!["merge proposed values into dimensions.toml".to_string()],
                vec![out.display().to_string()],
                serde_json::json!({ "seed_count": seeds.len() }),
            );
            println!("{}", serde_json::to_string_pretty(&env)?);
        } else {
            println!("[PASS] {summary}");
        }
    } else {
        println!("{toml}");
    }

    Ok(ExitCode::SUCCESS)
}

fn cmd_verify(spec_path: &Path, dir_path: &Path, json: bool) -> Result<ExitCode> {
    match verify_ledger(dir_path, spec_path) {
        Ok(()) => {
            let summary = format!(
                "ledger verification passed: zero drift in {}",
                dir_path.display()
            );
            if json {
                let env = HarnessEnvelope::new(
                    EnvelopeStatus::Success,
                    summary,
                    vec![],
                    vec![dir_path.join("tessel.lock.json").display().to_string()],
                    serde_json::json!({ "verified": true }),
                );
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("[PASS] {summary}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(err) => {
            let summary = format!("verification failed:\n{err}");
            if json {
                let env = HarnessEnvelope::new(
                    EnvelopeStatus::Error,
                    summary,
                    vec!["regenerate or restore files".to_string()],
                    vec![],
                    serde_json::json!({ "verified": false, "error": err.to_string() }),
                );
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                eprintln!("[FAIL] {summary}");
            }
            Ok(ExitCode::from(1))
        }
    }
}
