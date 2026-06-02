use thiserror::Error;

#[derive(Debug, Error)]
pub enum QbError {
    #[error("Lex error at line {line}: {msg}")]
    Lex { line: u32, msg: String },

    #[error("Parse error at line {line}: {msg}")]
    Parse { line: u32, msg: String },

    #[error("Analyze error: {0}")]
    #[allow(dead_code)]
    Analyze(String),

    #[error("Emit error: {0}")]
    #[allow(dead_code)]
    Emit(String),
}
