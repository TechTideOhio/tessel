# tessel: TideOS Eval Kit (Tool 3)

<div align="center">

**Deterministic synthetic evaluation input generator from explicit dimension tuples (persona x intent x complexity x edge case) with mathematical pairwise covering array guarantees.**

[![CI](https://github.com/TechTideOhio/tessel/actions/workflows/ci.yml/badge.svg)](https://github.com/TechTideOhio/tessel/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/TechTideOhio/tessel?color=blue&label=release)](https://github.com/TechTideOhio/tessel/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-emerald.svg)](LICENSE)
[![Rust: 2021 / 1.82+](https://img.shields.io/badge/Rust-2021%20%2F%201.82%2B-orange.svg)](Cargo.toml)
[![Zero-FFI](https://img.shields.io/badge/Dependencies-Zero--FFI%20Pure%20Rust-purple.svg)](Cargo.toml)

[Quickstart](#60-second-quickstart) • [Core Capabilities](#core-capabilities) • [Architecture](#architecture) • [Expert Patterns](#expert-patterns-grounded-in-real-agent-engineering) • [Contributing](CONTRIBUTING.md)

</div>

---

## Why tessel?

Most AI evaluation datasets suffer from near-duplicate prompt disease: engineers ask an LLM for "50 test questions," resulting in 50 variations of the exact same phrasing. Real production edge cases (such as monorepo build-order races, lint ratchets, and secret leaks) are left completely untested.

`tessel` solves this by strictly separating **dimension enumeration** from **natural language realization**:

1. **Define explicit dimensions**: persona x intent x complexity x edge case x target stack.
2. **Compute mathematical covering arrays**: Greedy pairwise combinatorial planning guarantees 100% of all feasible 2-way interactions are exercised (compressing 12,000 Cartesian combos into 90-120 high-leverage tuples).
3. **Attach deterministic code assertions**: Every generated test case carries code-graded verification checks (exit codes, file existence, regex, secret absence) that fail closed in CI.
4. **Reproducible and content-addressed**: Deterministic seeded generation (SplitMix64) and SHA-256 lockfile (`tessel.lock.json`) guarantee byte-identical outputs across machines.

`tessel` is a single, zero-dependency, pure-Rust CLI tool with no `-sys` C libraries, no external Python runtimes, and no LLM API calls in the generation path. It plans and realizes evaluation datasets in milliseconds.

---

## Feature Comparison

| Capability | `tessel` (TideOS) | Manual Prompt Writing | Promptfoo | LLM-as-a-Generator |
|---|:---:|:---:|:---:|:---:|
| **Mathematical Pairwise Guarantee** | **Yes** (100% 2-way coverage) | No (Ad-hoc) | No (Cartesian) | No (Stochastic) |
| **Deterministic Code Check Attachments** | **Yes** (Exit codes, regex, files) | Manual | Configurable | No |
| **Grounded Trace Harvester** | **Yes** (Harvests `reval` open-codes) | No | No | No |
| **Token-Shingle Deduplication** | **Yes** (Jaccard similarity gate) | No | No | No |
| **Cryptographic Lockfile** | **Yes** (`tessel.lock.json`) | No | No | No |
| **Reval CaseSpec Exporter** | **Yes** (`case.toml` + `prompt.md`) | No | No | No |
| **Binary Portability** | **Pure Rust** (Single binary) | Manual | Node.js | Python / API |

---

## Core Capabilities

### 1. Greedy Pairwise Combinatorial Planning
Full Cartesian product generation quickly explodes into thousands of redundant test cases ($5 \times 8 \times 5 \times 12 \times 5 = 12,000$ combinations). `tessel plan` computes a minimal covering array where every feasible pairwise interaction between any two dimensions is tested at least once, reducing test volume by **99%** while maintaining total interaction coverage:

```bash
tessel plan --spec dimensions.toml
```

Output:
```text
=== TESSEL COMBINATORIAL PLAN ===
Dimensions: 5 (persona: 5, intent: 8, complexity: 5, edge_case: 12, target_stack: 5)
Cartesian combinations: 12,000
Total feasible pairs: 465
Covering array tuples: 97 (100.0% pairwise coverage)
Compression ratio: 123.7x reduction
```

### 2. Trace Failure Harvester (The Flywheel)
Instead of guessing edge cases, `tessel seed` harvests real failure annotations directly from `reval` trace logs:

```bash
tessel seed --traces ../reval/fixtures/open_coding.jsonl --spec dimensions.toml
```

Harvested failure modes (`TYPECHECK-BUILD-ORDER-CASCADE`, `STREAM-DISCONNECT-ABORT`, `LINT-RATCHET-HYGIENE-GATE`) are injected directly into the edge case dimension, ensuring your evaluation suite expands as new failure modes emerge in production.

### 3. Attached Deterministic Code Assertions
Every realized test case carries explicit, machine-verifiable assertions:

```toml
# Generated case assertion contract
[[checks]]
type = "exit_code"
expected = 0

[[checks]]
type = "file_exists"
path = "packages/contracts/dist/index.d.ts"

[[checks]]
type = "not_regex"
pattern = "(gho_[A-Za-z0-9_]{36}|sk-ant-[A-Za-z0-9_-]{20,})"
```

### 4. Content-Addressed Ledger & Cryptographic Lock
Every generation run appends to an immutable ledger (`tuples.jsonl`, `cases.jsonl`) and writes a SHA-256 lockfile (`tessel.lock.json`). Running `tessel verify` ensures no silent prompt drift occurs in CI.

---

## 60-Second Quickstart

### Installation

#### Option 1: Cargo (from source)
```bash
cargo install --git https://github.com/TechTideOhio/tessel
```

#### Option 2: Pre-compiled GitHub Release Binary
Download the single standalone binary directly from [Releases](https://github.com/TechTideOhio/tessel/releases/latest) for your platform (Linux x86_64, macOS Apple Silicon / Intel, Windows x86_64).

### 1. Initialize Default Dimension Pack
Initializes a software factory dimension pack calibrated against real multi-agent traces:

```bash
tessel init --pack softfactory
```

### 2. Plan Combinatorial Coverage
Calculates pairwise coverage and verifies all constraints:

```bash
tessel plan --spec dimensions.toml
```

### 3. Generate Locked Synthetic Dataset
Generates deterministic test cases and writes cryptographic lock digests:

```bash
tessel generate --spec dimensions.toml --out out/
```

### 4. Export to Reval Golden Cases
Exports ready-to-run evaluation suites for `reval`:

```bash
tessel export --cases out/cases.jsonl --as reval-cases --out evals/
```

Outputs `evals/cases/<id>/case.toml` and `prompt.md` suites directly evaluable by `reval eval suite --cases-dir evals/cases`.

### 5. Verify Zero Drift
Recomputes digests and ensures generated inputs have not suffered unauthorized modification:

```bash
tessel verify --spec dimensions.toml --dir out/
```

---

## Machine-Readable JSON Protocol

Every subcommand supports `--json` to output TechTide's canonical harness envelope:

```json
{
  "status": "success",
  "summary": "planned 97 covering array tuples with 100.0% pairwise coverage",
  "next_actions": ["run `tessel generate --spec dimensions.toml --out out/`"],
  "artifacts": ["dimensions.toml"],
  "data": { ... }
}
```

---

## Architecture

```text
tessel/
├── Cargo.toml               # Rust 2021 edition, pure Rust dependencies
├── .cargo/config.toml       # Lints, target optimizations
├── .github/workflows/ci.yml # Multi-platform CI (Ubuntu, macOS, Windows)
├── .github/workflows/release.yml # Multi-target automated binary releases
├── tessel.ps1               # PowerShell wrapper
├── packs/
│   └── softfactory.toml     # Calibrated software factory dimension pack
├── src/
│   ├── main.rs              # CLI commands (init, plan, generate, export, report, seed, verify)
│   ├── lib.rs               # Library root export
│   ├── spec.rs              # Dimension specification parser and constraint validator
│   ├── combine.rs           # Combinatorial planner (pairwise greedy covering array, full, sample)
│   ├── realize.rs           # Deterministic slot realizer and Jaccard duplicate prevention
│   ├── ledger.rs            # Append-only content-addressed ledger and lockfile verification
│   ├── export.rs            # Reval CaseSpec export adapter
│   ├── report.rs            # Coverage matrix and Mermaid/DOT graph renderers
│   ├── seed.rs              # Trace corpus failure harvester (trace -> dataset flywheel)
│   ├── rng.rs               # Deterministic SplitMix64 PRNG
│   ├── error.rs             # TesselError domain error enum
│   └── envelope.rs          # HarnessEnvelope JSON protocol schema
└── tests/                   # Specification, combine, realize, ledger, and property test suites
```

---

## Expert Patterns (Grounded in Real Agent Engineering)

`tessel` incorporates concrete design patterns from three senior engineering leaders:
- **Peter Steinberger (@steipete)**: CLI-first over MCP, zero embedded LLM clients, standalone static binary, fast (< 50ms) execution, explicit `--json` harness envelope.
- **Harrison Chase (LangChain / LangSmith)**: Trace-to-dataset flywheel, trajectories as evaluation units, deterministic code checks per case, stratified test suites.
- **TJ Sweet (@orneryd / NornicDB & Mimir)**: Content-addressed ledger, unbroken lineage graphs (Case -> Tuple -> Dimension -> Trace), native Mermaid visualization, fail-closed validation.

---

## Verification & Test Gates

Run the test suite and release build locally:

```bash
# Formatting
cargo fmt --all -- --check

# Clippy lints (warnings treated as errors)
cargo clippy --all-targets -- -D warnings

# Integration and property test suite (50+ randomized property iterations)
cargo test --all-targets

# Release build
cargo build --release
```

---

## License

MIT (c) 2026 TechTide AI & Alexi5000. See [LICENSE](LICENSE) for details.
