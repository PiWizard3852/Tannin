use crate::common::Span;

#[derive(Debug, Clone)]
pub struct AnalyzeError {
    pub message: String,
    pub span: Span,
}

impl AnalyzeError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

pub type AnalyzeResult<T> = Result<T, AnalyzeError>;
