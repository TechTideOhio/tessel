use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::TesselError;

#[derive(Debug, Deserialize)]
struct TraceLine {
    #[serde(default)]
    open_code: Option<String>,
    #[serde(default)]
    axial_category: Option<String>,
    #[serde(default)]
    diagnostic_excerpt: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HarvestedSeed {
    pub id: String,
    pub count: usize,
    pub source: String,
    pub axial_category: String,
    pub hint: String,
}

pub fn harvest_seeds_from_jsonl(path: impl AsRef<Path>) -> Result<Vec<HarvestedSeed>, TesselError> {
    let p = path.as_ref();
    let content = fs::read_to_string(p).map_err(|e| TesselError::io(p, e))?;

    let mut counts: BTreeMap<String, (usize, String, String)> = BTreeMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(trace) = serde_json::from_str::<TraceLine>(trimmed) {
            if let Some(code) = trace.open_code {
                if !code.is_empty() {
                    let axial = trace
                        .axial_category
                        .unwrap_or_else(|| "unclassified".to_string());
                    let hint = trace.diagnostic_excerpt.unwrap_or_default();
                    let entry = counts.entry(code.clone()).or_insert((0, axial, hint));
                    entry.0 += 1;
                }
            }
        }
    }

    let mut seeds = Vec::new();
    for (code, (cnt, axial, hint)) in counts {
        let slug = code.to_lowercase().replace('_', "-");
        seeds.push(HarvestedSeed {
            id: slug,
            count: cnt,
            source: format!("reval:{code}"),
            axial_category: axial,
            hint,
        });
    }

    seeds.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.cmp(&b.id)));
    Ok(seeds)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn approx_weight(count: usize) -> u32 {
    let f = count as f64;
    (f.sqrt().round() as u32).max(1)
}

#[must_use]
pub fn format_proposed_toml(seeds: &[HarvestedSeed]) -> String {
    let mut out = String::new();
    out.push_str("# Proposed edge_case dimension values derived from trace corpus\n");
    out.push_str("[[dimension]]\nname = \"edge_case\"\nvalues = [\n");
    for s in seeds {
        let weight = approx_weight(s.count);
        out.push_str(&format!(
            "  {{ id = \"{}\", weight = {}, source = \"{}\", hint = \"Axial: {}\" }},\n",
            s.id, weight, s.source, s.axial_category
        ));
    }
    out.push_str("]\n");
    out
}
