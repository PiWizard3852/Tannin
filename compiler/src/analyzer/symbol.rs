use crate::analyzer::types::ValueType;
use crate::common::Span;

#[derive(Debug, Clone)]
pub enum Symbol {
    Function {
        span: Span,
    },

    Struct {
        index: usize,
        span: Span,
    },

    Enum {
        span: Span,
    },

    Const {
        span: Span,
    },

    Variable {
        index: usize,
        type_: ValueType,
        mutable: bool,
        span: Span,
    },
}
