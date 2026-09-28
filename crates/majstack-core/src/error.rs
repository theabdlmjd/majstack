use thiserror::Error;

#[derive(Debug, Error)]
pub enum MajstackError {
    #[error("state: {0}")]
    State(String),
    #[error("config: {0}")]
    Config(String),
    #[error("provider: {0}")]
    Provider(String),
    #[error("tool: {0}")]
    Tool(String),
    #[error("policy denied: {0}")]
    Policy(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("blocked: {0}")]
    Blocked(String),
    #[error("verification failed: {0}")]
    Verification(String),
    #[error("database: {0}")]
    Database(String),
    #[error("network: {0}")]
    Network(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml: {0}")]
    Toml(String),
    #[error("other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, MajstackError>;
