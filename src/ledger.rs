use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::combine::Tuple;
use crate::error::TesselError;
use crate::realize::RealizedCase;
use crate::spec::Spec;

pub const LOCK_FILE: &str = "tessel.lock.json";
pub const TUPLES_FILE: &str = "tuples.jsonl";
pub const CASES_FILE: &str = "cases.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub spec_id: String,
    pub spec_version: String,
    pub spec_sha256: String,
    pub seed: u64,
    pub files: BTreeMap<String, FileDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDigest {
    pub sha256: String,
    pub byte_count: usize,
    pub record_count: usize,
}

pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

pub fn save_ledger(
    out_dir: impl AsRef<Path>,
    spec_path: impl AsRef<Path>,
    spec: &Spec,
    tuples: &[Tuple],
    cases: &[RealizedCase],
) -> Result<(), TesselError> {
    let out = out_dir.as_ref();
    fs::create_dir_all(out).map_err(|e| TesselError::io(out, e))?;

    let spec_bytes =
        fs::read(spec_path.as_ref()).map_err(|e| TesselError::io(spec_path.as_ref(), e))?;
    let spec_sha256 = compute_sha256(&spec_bytes);

    let mut tuples_buf = String::new();
    for t in tuples {
        let line = serde_json::to_string(t).map_err(|e| TesselError::Json {
            path: TUPLES_FILE.to_string(),
            source: e,
        })?;
        tuples_buf.push_str(&line);
        tuples_buf.push('\n');
    }

    let tuples_path = out.join(TUPLES_FILE);
    fs::write(&tuples_path, &tuples_buf).map_err(|e| TesselError::io(&tuples_path, e))?;

    let mut cases_buf = String::new();
    for c in cases {
        let line = serde_json::to_string(c).map_err(|e| TesselError::Json {
            path: CASES_FILE.to_string(),
            source: e,
        })?;
        cases_buf.push_str(&line);
        cases_buf.push('\n');
    }

    let cases_path = out.join(CASES_FILE);
    fs::write(&cases_path, &cases_buf).map_err(|e| TesselError::io(&cases_path, e))?;

    let mut files = BTreeMap::new();
    files.insert(
        TUPLES_FILE.to_string(),
        FileDigest {
            sha256: compute_sha256(tuples_buf.as_bytes()),
            byte_count: tuples_buf.len(),
            record_count: tuples.len(),
        },
    );
    files.insert(
        CASES_FILE.to_string(),
        FileDigest {
            sha256: compute_sha256(cases_buf.as_bytes()),
            byte_count: cases_buf.len(),
            record_count: cases.len(),
        },
    );

    let lock = LockFile {
        schema_version: 1,
        tool: "tessel".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        spec_id: spec.spec.id.clone(),
        spec_version: spec.spec.version.clone(),
        spec_sha256,
        seed: spec.spec.seed,
        files,
    };

    let lock_json = serde_json::to_string_pretty(&lock).map_err(|e| TesselError::Json {
        path: LOCK_FILE.to_string(),
        source: e,
    })?;

    let lock_path = out.join(LOCK_FILE);
    fs::write(&lock_path, lock_json).map_err(|e| TesselError::io(&lock_path, e))?;

    Ok(())
}

pub fn verify_ledger(
    out_dir: impl AsRef<Path>,
    spec_path: impl AsRef<Path>,
) -> Result<(), TesselError> {
    let out = out_dir.as_ref();
    let lock_path = out.join(LOCK_FILE);
    if !lock_path.is_file() {
        return Err(TesselError::Invalid(format!(
            "lockfile missing at {}",
            lock_path.display()
        )));
    }

    let lock_raw = fs::read_to_string(&lock_path).map_err(|e| TesselError::io(&lock_path, e))?;
    let lock: LockFile = serde_json::from_str(&lock_raw).map_err(|e| TesselError::Json {
        path: lock_path.display().to_string(),
        source: e,
    })?;

    let mut drifts = Vec::new();

    let spec_bytes =
        fs::read(spec_path.as_ref()).map_err(|e| TesselError::io(spec_path.as_ref(), e))?;
    let actual_spec_sha = compute_sha256(&spec_bytes);
    if actual_spec_sha != lock.spec_sha256 {
        drifts.push(format!(
            "spec sha256 mismatch: lock={}, actual={}",
            lock.spec_sha256, actual_spec_sha
        ));
    }

    for (fname, expected) in &lock.files {
        let fpath = out.join(fname);
        if !fpath.is_file() {
            drifts.push(format!("tracked file `{fname}` is missing"));
            continue;
        }
        let bytes = fs::read(&fpath).map_err(|e| TesselError::io(&fpath, e))?;
        let actual_sha = compute_sha256(&bytes);
        if actual_sha != expected.sha256 {
            drifts.push(format!(
                "file `{fname}` digest drifted: lock={}, actual={}",
                expected.sha256, actual_sha
            ));
        }
        if bytes.len() != expected.byte_count {
            drifts.push(format!(
                "file `{fname}` length drifted: lock={}, actual={}",
                expected.byte_count,
                bytes.len()
            ));
        }
    }

    if drifts.is_empty() {
        Ok(())
    } else {
        Err(TesselError::Drift(drifts))
    }
}
