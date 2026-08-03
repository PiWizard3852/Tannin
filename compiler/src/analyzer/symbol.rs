use crate::common::Span;

#[derive(Debug, Clone)]
pub enum Symbol {
    Function {
        span: Span,
    },

    Struct {
        span: Span,
    },

    Enum {
        span: Span,
    },

    Const {
        span: Span,
    },

    Variable {
        mutable: bool,
        span: Span,
    },
}