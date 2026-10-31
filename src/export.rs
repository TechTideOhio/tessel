use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::TesselError;
use crate::realize::RealizedCase;
use crate::spec::{CheckSpecTemplate, CheckTemplate, Severity};

#[derive(Debug, Clone, Serialize)]
pub struct RevalCaseFile {
    pub id: String,
    pub title: String,
    pub stratum: String,
    pub prompt: Option<String>,
    pub checks: Vec<RevalCheckSpec>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevalCheckSpec {
    pub id: String,
    pub check: RevalCheckKind,
    pub weight: f64,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RevalCheckKind {
    FileExists {
        path: String,
    },
    Contains {
        path: String,
        needle: String,
    },
    NotContains {
        path: String,
        needle: String,
    },
    RegexMatch {
        path: String,
        pattern: String,
    },
    NoSecretLeak {
        path: String,
    },
    JsonFieldEquals {
        path: String,
        field: String,
        expected_json: String,
    },
    MaxLineCount {
        path: String,
        max_lines: usize,
    },
}

impl From<&CheckSpecTemplate> for RevalCheckSpec {
    fn from(c: &CheckSpecTemplate) -> Self {
        let kind = match &c.check {
            CheckTemplate::FileExists { path } => RevalCheckKind::FileExists { path: path.clone() },
            CheckTemplate::Contains { path, needle } => RevalCheckKind::Contains {
                path: path.clone(),
                needle: needle.clone(),
            },
            CheckTemplate::NotContains { path, needle } => RevalCheckKind::NotContains {
                path: path.clone(),
                needle: needle.clone(),
            },
            CheckTemplate::RegexMatch { path, pattern } => RevalCheckKind::RegexMatch {
                path: path.clone(),
                pattern: pattern.clone(),
            },
            CheckTemplate::NoSecretLeak { path } => {
                RevalCheckKind::NoSecretLeak { path: path.clone() }
            }
            CheckTemplate::JsonFieldEquals {
                path,
                field,
                expected,
            } => RevalCheckKind::JsonFieldEquals {
                path: path.clone(),
                field: field.clone(),
                expected_json: expected.to_string(),
            },
            CheckTemplate::MaxLineCount { path, max_lines } => RevalCheckKind::MaxLineCount {
                path: path.clone(),
                max_lines: *max_lines,
            },
        };
        Self {
            id: c.id.clone(),
            check: kind,
            weight: c.weight,
            severity: c.severity,
        }
    }
}

pub fn export_reval_cases(
    cases: &[RealizedCase],
    target_dir: impl AsRef<Path>,
) -> Result<usize, TesselError> {
    let base = target_dir.as_ref();
    fs::create_dir_all(base).map_err(|e| TesselError::io(base, e))?;

    let cases_dir = base.join("cases");
    fs::create_dir_all(&cases_dir).map_err(|e| TesselError::io(&cases_dir, e))?;

    for c in cases {
        // 1. Hierarchical folder: cases/<id>/case.toml + prompt.md
        let case_folder = cases_dir.join(&c.id);
        fs::create_dir_all(&case_folder).map_err(|e| TesselError::io(&case_folder, e))?;

        let prompt_path = case_folder.join("prompt.md");
        fs::write(&prompt_path, &c.prompt).map_err(|e| TesselError::io(&prompt_path, e))?;

        let reval_checks: Vec<RevalCheckSpec> = c.checks.iter().map(RevalCheckSpec::from).collect();
        let reval_case_hierarchical = RevalCaseFile {
            id: c.id.clone(),
            title: c.title.clone(),
            stratum: c.stratum.clone(),
            prompt: Some("prompt.md".to_string()),
            checks: reval_checks.clone(),
        };

        let toml_str_hier =
            toml::to_string_pretty(&reval_case_hierarchical).map_err(|e| TesselError::TomlSer {
                path: format!("{}/case.toml", c.id),
                source: e,
            })?;

        let toml_path_hier = case_folder.join("case.toml");
        fs::write(&toml_path_hier, toml_str_hier)
            .map_err(|e| TesselError::io(&toml_path_hier, e))?;

        // 2. Flat file directly in cases_dir: cases/<id>.toml (consumed by `reval eval suite`)
        let reval_case_flat = RevalCaseFile {
            id: c.id.clone(),
            title: c.title.clone(),
            stratum: c.stratum.clone(),
            prompt: Some(c.prompt.clone()),
            checks: reval_checks,
        };

        let toml_str_flat =
            toml::to_string_pretty(&reval_case_flat).map_err(|e| TesselError::TomlSer {
                path: format!("{}.toml", c.id),
                source: e,
            })?;

        let toml_path_flat = cases_dir.join(format!("{}.toml", c.id));
        fs::write(&toml_path_flat, toml_str_flat)
            .map_err(|e| TesselError::io(&toml_path_flat, e))?;
    }

    Ok(cases.len())
}
