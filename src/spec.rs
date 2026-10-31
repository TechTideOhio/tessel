use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::TesselError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageMode {
    #[default]
    Pairwise,
    Full,
    Sample,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub id: String,
    pub version: String,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default)]
    pub coverage: CoverageMode,
    #[serde(default = "default_sample")]
    pub sample: usize,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_seed() -> u64 {
    357
}

fn default_sample() -> usize {
    100
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    #[default]
    Graded,
    Blocker,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckTemplate {
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
        expected: serde_json::Value,
    },
    MaxLineCount {
        path: String,
        max_lines: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckSpecTemplate {
    pub id: String,
    pub check: CheckTemplate,
    #[serde(default = "unit_weight")]
    pub weight: f64,
    #[serde(default)]
    pub severity: Severity,
}

fn unit_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Value {
    pub id: String,
    #[serde(default = "default_weight_u32")]
    pub weight: u32,
    #[serde(default)]
    pub hint: String,
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub checks: Vec<CheckSpecTemplate>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub stratum: Option<String>,
}

fn default_weight_u32() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dimension {
    pub name: String,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constraint {
    pub forbid: BTreeMap<String, String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub spec: Meta,
    #[serde(rename = "dimension")]
    pub dimensions: Vec<Dimension>,
    #[serde(default, rename = "constraint")]
    pub constraints: Vec<Constraint>,
}

fn is_valid_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    bytes.iter().all(|&b| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'_' || b == b'-'
    })
}

impl Spec {
    pub fn load_file(path: impl AsRef<Path>) -> Result<Self, TesselError> {
        let p = path.as_ref();
        let content = fs::read_to_string(p).map_err(|e| TesselError::io(p, e))?;
        Self::from_toml(&content, p)
    }

    pub fn from_toml(content: &str, path: impl AsRef<Path>) -> Result<Self, TesselError> {
        let spec: Self = toml::from_str(content).map_err(|e| TesselError::Toml {
            path: path.as_ref().display().to_string(),
            source: e,
        })?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), TesselError> {
        let mut errors = Vec::new();

        if !is_valid_id(&self.spec.id) {
            errors.push(format!(
                "spec.id `{}` is invalid; must match [a-z0-9][a-z0-9._-]*",
                self.spec.id
            ));
        }

        if self.spec.version.trim().is_empty() {
            errors.push("spec.version must not be empty".to_string());
        }

        if self.dimensions.is_empty() {
            errors.push("spec must declare at least one dimension".to_string());
        }

        let mut dim_names = BTreeSet::new();
        let mut dim_values_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for dim in &self.dimensions {
            if !is_valid_id(&dim.name) {
                errors.push(format!(
                    "dimension name `{}` is invalid; must match [a-z0-9][a-z0-9._-]*",
                    dim.name
                ));
            }
            if !dim_names.insert(dim.name.clone()) {
                errors.push(format!("duplicate dimension name `{}`", dim.name));
            }

            if dim.values.is_empty() {
                errors.push(format!("dimension `{}` has no values", dim.name));
            }

            let mut val_ids = BTreeSet::new();
            for val in &dim.values {
                if !is_valid_id(&val.id) {
                    errors.push(format!(
                        "value id `{}` in dimension `{}` is invalid",
                        val.id, dim.name
                    ));
                }
                if !val_ids.insert(val.id.clone()) {
                    errors.push(format!(
                        "duplicate value id `{}` in dimension `{}`",
                        val.id, dim.name
                    ));
                }
                if val.weight == 0 {
                    errors.push(format!(
                        "value `{}` in dimension `{}` has weight 0",
                        val.id, dim.name
                    ));
                }
                for chk in &val.checks {
                    if !is_valid_id(&chk.id) {
                        errors.push(format!("check id `{}` in `{}` is invalid", chk.id, val.id));
                    }
                    if let CheckTemplate::RegexMatch { pattern, .. } = &chk.check {
                        if let Err(e) = Regex::new(pattern) {
                            errors.push(format!(
                                "invalid regex `{pattern}` in check `{}`: {e}",
                                chk.id
                            ));
                        }
                    }
                }
            }
            dim_values_map.insert(dim.name.clone(), val_ids);
        }

        for (idx, c) in self.constraints.iter().enumerate() {
            if c.forbid.len() < 2 {
                errors.push(format!(
                    "constraint #{} must specify at least two coordinates",
                    idx + 1
                ));
            }
            for (dim, val) in &c.forbid {
                match dim_values_map.get(dim) {
                    None => errors.push(format!(
                        "constraint #{} references unknown dimension `{dim}`",
                        idx + 1
                    )),
                    Some(vals) => {
                        if !vals.contains(val) {
                            errors.push(format!(
                                "constraint #{} references unknown value `{val}` in dimension `{dim}`",
                                idx + 1
                            ));
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(TesselError::Spec(errors))
        }
    }

    #[must_use]
    pub fn get_dimension(&self, name: &str) -> Option<&Dimension> {
        self.dimensions.iter().find(|d| d.name == name)
    }
}
