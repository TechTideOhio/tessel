use thiserror::Error;

/// Domain errors. Every variant is fail-closed: nothing here is ever downgraded to a pass.
#[derive(Debug, Error)]
pub enum TesselError {
    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("toml error at {path}: {source}")]
    Toml {
        path: String,
        source: toml::de::Error,
    },
    #[error("toml serialization error at {path}: {source}")]
    TomlSer {
        path: String,
        source: toml::ser::Error,
    },
    #[error("json error at {path}: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },
    #[error("invalid spec:\n  - {}", .0.join("\n  - "))]
    Spec(Vec<String>),
    #[error("unreachable value pairs (no valid tuple can cover them):\n  - {}", .0.join("\n  - "))]
    Infeasible(Vec<String>),
    #[error("full product has {count} tuples, above the cap of {cap}; use pairwise or sample")]
    TooLarge { count: u128, cap: usize },
    #[error("realization failed: {0}")]
    Realize(String),
    #[error("{0}")]
    Invalid(String),
    #[error("verification failed:\n  - {}", .0.join("\n  - "))]
    Drift(Vec<String>),
}

impl TesselError {
    pub fn io(path: impl AsRef<std::path::Path>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }
}
