use crate::analyzer::error::{AnalyzeError, AnalyzeResult};
use crate::analyzer::scope::Scope;
use crate::analyzer::symbol::Symbol;
use crate::parser::ast::{Declaration, Program};

pub struct Analyzer {
    globals: Scope,
}

impl Analyzer {
    pub fn analyze(program: &Program) -> AnalyzeResult<()> {
        let mut analyzer = Self {
            globals: Scope::new(),
        };

        analyzer.collect_declarations(program)
    }

    fn collect_declarations(&mut self, program: &Program) -> AnalyzeResult<()> {
        for declaration in &program.declarations {
            let (name, span, symbol) = match declaration {
                Declaration::Function(function) => (
                    &function.name,
                    function.span,
                    Symbol::Function { span: function.span },
                ),

                Declaration::Struct(struct_) => (
                    &struct_.name,
                    struct_.span,
                    Symbol::Struct { span: struct_.span },
                ),

                Declaration::Enum(enum_) => (
                    &enum_.name,
                    enum_.span,
                    Symbol::Enum { span: enum_.span },
                ),

                Declaration::Const(const_) => (
                    &const_.name,
                    const_.span,
                    Symbol::Const { span: const_.span },
                ),
            };

            if !self.globals.declare(name.clone(), symbol) {
                return Err(AnalyzeError::new(
                    format!("duplicate declaration of '{name}'"),
                    span,
                ));
            }
        }

        Ok(())
    }
}
