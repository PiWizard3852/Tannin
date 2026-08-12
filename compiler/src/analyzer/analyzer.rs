use std::collections::HashMap;

use crate::analyzer::checked::*;
use crate::analyzer::error::{AnalyzeError, AnalyzeResult};
use crate::analyzer::scope::Scope;
use crate::analyzer::symbol::Symbol;
use crate::analyzer::types::{FunctionType, ValueType};
use crate::common::Span;
use crate::parser::ast::*;

pub struct Analyzer {
    globals: Scope,
    scopes: Vec<Scope>,
    signatures: HashMap<String, FunctionType>,
    constants: HashMap<String, (CheckedLiteral, ValueType)>,
    locals: Vec<ValueType>,
    return_type: Option<ValueType>,
}

impl Analyzer {
    pub fn analyze(program: &Program) -> AnalyzeResult<CheckedProgram> {
        let mut analyzer = Self {
            globals: Scope::new(),
            scopes: Vec::new(),
            signatures: HashMap::new(),
            constants: HashMap::new(),
            locals: Vec::new(),
            return_type: None,
        };

        analyzer.collect_declarations(program)?;
        analyzer.collect_signatures(program)?;
        analyzer.collect_constants(program)?;

        let mut functions = Vec::new();

        for declaration in &program.declarations {
            if let Declaration::Function(function) = declaration {
                functions.push(analyzer.check_function(function)?);
            }
        }

        Ok(CheckedProgram { functions })
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

    fn collect_signatures(&mut self, program: &Program) -> AnalyzeResult<()> {
        for declaration in &program.declarations {
            if let Declaration::Function(function) = declaration {
                let parameters = function.parameters.iter()
                    .map(|parameter| self.resolve_type(&parameter.type_))
                    .collect::<AnalyzeResult<_>>()?;
                let return_type = function.return_type.as_ref()
                    .map(|type_| self.resolve_type(type_))
                    .transpose()?;

                self.signatures.insert(function.name.clone(), FunctionType {
                    parameters,
                    return_type,
                });
            }
        }

        Ok(())
    }

    fn collect_constants(&mut self, program: &Program) -> AnalyzeResult<()> {
        for declaration in &program.declarations {
            let Declaration::Const(const_) = declaration else {
                continue;
            };

            let type_ = self.resolve_type(&const_.type_)?;
            let is_literal = match &const_.value {
                Expression::Literal { .. } => true,
                Expression::Unary { operator: UnaryOperator::Negate, operand, .. } => {
                    matches!(**operand, Expression::Literal {
                        value: Literal::Integer(_) | Literal::Float(_), ..
                    })
                }
                _ => false,
            };

            if !is_literal {
                return Err(AnalyzeError::new(
                    "constant initializer must be a literal",
                    const_.value.span(),
                ));
            }

            let value = self.check_value(&const_.value, Some(&type_))?;

            if let ExpressionKind::Literal(value) = value.kind {
                self.constants.insert(const_.name.clone(), (value, type_));
            }
        }

        Ok(())
    }

    fn resolve_type(&self, type_: &Type) -> AnalyzeResult<ValueType> {
        match type_ {
            Type::Named { name, span } => ValueType::named(name).ok_or_else(|| {
                AnalyzeError::new(format!("unknown type '{name}'"), *span)
            }),
            Type::Array { span, .. } => Err(AnalyzeError::new("unsupported type", *span)),
        }
    }

    fn check_function(&mut self, function: &FunctionDecl) -> AnalyzeResult<CheckedFunction> {
        let type_ = self.signatures[&function.name].clone();
        self.return_type = type_.return_type.clone();
        self.scopes.push(Scope::new());

        for (parameter, type_) in function.parameters.iter().zip(&type_.parameters) {
            self.declare_local(&parameter.name, type_.clone(), false, parameter.span)?;
        }

        let body = self.check_block(&function.body, type_.return_type.as_ref())?;

        match (&type_.return_type, &body.flow) {
            (_, Flow::Returns) | (Some(_), Flow::Value(_)) | (None, Flow::NoValue) => {}
            (Some(_), Flow::NoValue) => {
                return Err(AnalyzeError::new("missing return value", function.body.span));
            }
            (None, Flow::Value(_)) => {
                return Err(AnalyzeError::new(
                    "function does not return a value",
                    function.body.span,
                ));
            }
        }

        self.scopes.pop();

        Ok(CheckedFunction {
            name: function.name.clone(),
            pub_: function.pub_,
            type_,
            locals: std::mem::take(&mut self.locals),
            body,
        })
    }

    fn check_block(
        &mut self,
        block: &Block,
        expected: Option<&ValueType>,
    ) -> AnalyzeResult<CheckedBlock> {
        self.scopes.push(Scope::new());
        let result = self.check_block_body(block, expected);
        self.scopes.pop();

        result
    }

    fn check_block_body(
        &mut self,
        block: &Block,
        expected: Option<&ValueType>,
    ) -> AnalyzeResult<CheckedBlock> {
        let mut statements = Vec::new();
        let mut returns = false;

        for statement in &block.statements {
            let statement = self.check_statement(statement)?;
            returns |= statement.returns();
            statements.push(statement);
        }

        let tail = block.tail.as_ref()
            .map(|expression| self.check_expression(expression, expected).map(Box::new))
            .transpose()?;
        let flow = if returns {
            Flow::Returns
        } else {
            tail.as_ref().map_or(Flow::NoValue, |expression| expression.flow.clone())
        };

        Ok(CheckedBlock { statements, tail, flow })
    }

    fn check_statement(&mut self, statement: &Statement) -> AnalyzeResult<CheckedStatement> {
        match statement {
            Statement::Block(block) => Ok(CheckedStatement::Block(self.check_block(block, None)?)),

            Statement::Let { name, mutable, type_, initializer, span } => {
                let type_ = type_.as_ref().map(|type_| self.resolve_type(type_)).transpose()?;
                let initializer = initializer.as_ref().ok_or_else(|| {
                    AnalyzeError::new("variable needs an initializer", *span)
                })?;
                let initializer = self.check_value(initializer, type_.as_ref())?;
                let index = self.declare_local(
                    name,
                    initializer.value_type().clone(),
                    *mutable,
                    *span,
                )?;

                Ok(CheckedStatement::Let { index, initializer })
            }

            Statement::Return { value, span } => {
                let return_type = self.return_type.clone();
                let value = match (value, return_type.as_ref()) {
                    (Some(value), Some(type_)) => Some(self.check_value(value, Some(type_))?),
                    (None, None) => None,
                    (Some(value), None) => {
                        return Err(AnalyzeError::new("function does not return a value", value.span()));
                    }
                    (None, Some(_)) => return Err(AnalyzeError::new("missing return value", *span)),
                };

                Ok(CheckedStatement::Return(value))
            }

            Statement::Expression { expression, .. } => {
                Ok(CheckedStatement::Expression(self.check_expression(expression, None)?))
            }

            Statement::While { span, .. } => Err(AnalyzeError::new("unsupported while statement", *span)),
        }
    }

    fn declare_local(
        &mut self,
        name: &str,
        type_: ValueType,
        mutable: bool,
        span: Span,
    ) -> AnalyzeResult<usize> {
        let index = self.locals.len();
        let symbol = Symbol::Variable { index, type_: type_.clone(), mutable, span };
        let scope = self.scopes.last_mut().expect("local scope");

        if !scope.declare(name.to_owned(), symbol) {
            return Err(AnalyzeError::new(format!("duplicate binding '{name}'"), span));
        }

        self.locals.push(type_);

        Ok(index)
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev()
            .find_map(|scope| scope.lookup(name))
            .or_else(|| self.globals.lookup(name))
    }

    fn check_value(
        &mut self,
        expression: &Expression,
        expected: Option<&ValueType>,
    ) -> AnalyzeResult<CheckedExpression> {
        let expression = self.check_expression(expression, expected)?;

        if expression.flow.value().is_none() {
            return Err(AnalyzeError::new("expected a value", expression.span));
        }

        Ok(expression)
    }

    fn check_expression(
        &mut self,
        expression: &Expression,
        expected: Option<&ValueType>,
    ) -> AnalyzeResult<CheckedExpression> {
        let span = expression.span();
        let result = match expression {
            Expression::Literal { value, .. } => self.check_literal(value, false, expected, span)?,
            Expression::Identifier { value, .. } => self.check_identifier(value, span)?,
            Expression::Call { callee, arguments, .. } => self.check_call(callee, arguments, span)?,
            Expression::Unary { operator, operand, .. } => {
                self.check_unary(*operator, operand, expected, span)?
            }
            Expression::Binary { left, operator, right, .. } => {
                self.check_binary(left, *operator, right, expected, span)?
            }
            Expression::Block(block) => {
                let block = self.check_block(block, expected)?;
                let flow = block.flow.clone();
                CheckedExpression::new(ExpressionKind::Block(block), flow, span)
            }
            _ => return Err(AnalyzeError::new("unsupported expression", span)),
        };

        if let (Some(expected), Some(actual)) = (expected, result.flow.value()) {
            if expected != actual {
                return Err(AnalyzeError::new(format!("expected {expected}, found {actual}"), span));
            }
        }

        Ok(result)
    }

    fn check_identifier(&self, name: &str, span: Span) -> AnalyzeResult<CheckedExpression> {
        let (kind, type_) = match self.lookup(name) {
            Some(Symbol::Variable { index, type_, .. }) => (ExpressionKind::Local(*index), type_.clone()),
            Some(Symbol::Const { .. }) => {
                let (value, type_) = &self.constants[name];
                (ExpressionKind::Literal(value.clone()), type_.clone())
            }
            Some(_) => return Err(AnalyzeError::new(format!("'{name}' is not a value"), span)),
            None => return Err(AnalyzeError::new(format!("unknown name '{name}'"), span)),
        };

        Ok(CheckedExpression::new(kind, Flow::Value(type_), span))
    }

    fn check_literal(
        &self,
        value: &Literal,
        negative: bool,
        expected: Option<&ValueType>,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        let (value, type_) = match value {
            Literal::Integer(value) => {
                let type_ = expected.filter(|type_| type_.is_integer())
                    .cloned()
                    .unwrap_or(ValueType::ISIZE);
                let ValueType::Integer { width, signed } = type_ else {
                    unreachable!();
                };
                let bits = width.bits() - u32::from(signed);
                let bound = 1_i128 << bits;
                let number = value.replace('_', "").parse::<i128>().ok().map(|number| {
                    if negative { -number } else { number }
                });
                let lower = if signed { -bound } else { 0 };

                let number = number.filter(|number| {
                    (!negative || signed) && (lower..bound).contains(number)
                }).ok_or_else(|| AnalyzeError::new(format!("integer literal out of range for {type_}"), span))?;

                (CheckedLiteral::Integer(number), type_)
            }

            Literal::Float(value) => {
                let value = value.replace('_', "");
                let (number, type_) = if expected == Some(&ValueType::Float(32)) {
                    (value.parse::<f32>().map(f64::from), ValueType::Float(32))
                } else {
                    (value.parse::<f64>(), ValueType::Float(64))
                };
                let number = number.ok().filter(|number| number.is_finite()).ok_or_else(|| {
                    AnalyzeError::new(format!("floating point literal out of range for {type_}"), span)
                })?;
                let number = if negative { -number } else { number };

                (CheckedLiteral::Float(number), type_)
            }

            Literal::Bool(value) => (CheckedLiteral::Bool(*value), ValueType::Bool),
            Literal::Char(value) => (CheckedLiteral::Char(*value), ValueType::Char),
            Literal::String(value) => (CheckedLiteral::String(value.clone()), ValueType::String),
        };

        Ok(CheckedExpression::new(ExpressionKind::Literal(value), Flow::Value(type_), span))
    }

    fn check_call(
        &mut self,
        callee: &Expression,
        arguments: &[Expression],
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        let Expression::Identifier { value: function, .. } = callee else {
            return Err(AnalyzeError::new("expected a function name", callee.span()));
        };

        match self.lookup(function) {
            Some(Symbol::Function { .. }) => {}
            Some(_) => return Err(AnalyzeError::new(format!("'{function}' is not a function"), callee.span())),
            None => return Err(AnalyzeError::new(format!("unknown function '{function}'"), callee.span())),
        }

        let type_ = self.signatures[function].clone();

        if arguments.len() != type_.parameters.len() {
            return Err(AnalyzeError::new(
                format!("expected {} arguments, found {}", type_.parameters.len(), arguments.len()),
                span,
            ));
        }

        let arguments = arguments.iter().zip(&type_.parameters)
            .map(|(argument, type_)| self.check_value(argument, Some(type_)))
            .collect::<AnalyzeResult<_>>()?;
        let kind = ExpressionKind::Call { function: function.clone(), arguments };
        let flow = type_.return_type.map_or(Flow::NoValue, Flow::Value);

        Ok(CheckedExpression::new(kind, flow, span))
    }

    fn check_unary(
        &mut self,
        operator: UnaryOperator,
        operand: &Expression,
        expected: Option<&ValueType>,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        if operator == UnaryOperator::Negate {
            if let Expression::Literal { value: value @ (Literal::Integer(_) | Literal::Float(_)), .. } = operand {
                return self.check_literal(value, true, expected, span);
            }
        }

        let operand = self.check_value(operand, expected)?;
        let type_ = operand.value_type().clone();
        let valid = match operator {
            UnaryOperator::Negate => matches!(type_, ValueType::Integer { signed: true, .. } | ValueType::Float(_)),
            UnaryOperator::Not => type_ == ValueType::Bool,
            UnaryOperator::BitwiseNot => type_.is_integer(),
            _ => false,
        };

        if !valid {
            return Err(AnalyzeError::new(format!("invalid unary operator for {type_}"), span));
        }

        let kind = ExpressionKind::Unary { operator, operand: Box::new(operand) };

        Ok(CheckedExpression::new(kind, Flow::Value(type_), span))
    }

    fn check_binary(
        &mut self,
        left: &Expression,
        operator: BinaryOperator,
        right: &Expression,
        expected: Option<&ValueType>,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        use BinaryOperator::*;

        let comparison = matches!(operator, Equal | NotEqual | Less | LessEqual | Greater | GreaterEqual);
        let expected_operand = match operator {
            LogicalAnd | LogicalOr => Some(&ValueType::Bool),
            _ if comparison => None,
            _ => expected.filter(|type_| type_.is_numeric()),
        };
        let (left, right) = if expected_operand.is_none()
            && Self::untyped_numeric(left) && !Self::untyped_numeric(right)
        {
            let right = self.check_value(right, None)?;
            let left = self.check_value(left, Some(right.value_type()))?;
            (left, right)
        } else {
            let left = self.check_value(left, expected_operand)?;
            let right = self.check_value(right, Some(left.value_type()))?;
            (left, right)
        };

        let type_ = left.value_type();
        let valid = match operator {
            Add | Subtract | Multiply | Divide | Modulo => type_.is_numeric(),
            BitwiseAnd | BitwiseOr | BitwiseXor | LeftShift | RightShift => type_.is_integer(),
            Less | LessEqual | Greater | GreaterEqual => type_.is_numeric() || *type_ == ValueType::Char,
            Equal | NotEqual => true,
            LogicalAnd | LogicalOr => *type_ == ValueType::Bool,
        };

        if !valid {
            return Err(AnalyzeError::new(format!("invalid binary operator for {type_}"), span));
        }

        let type_ = if comparison { ValueType::Bool } else { type_.clone() };
        let kind = ExpressionKind::Binary { left: Box::new(left), operator, right: Box::new(right) };

        Ok(CheckedExpression::new(kind, Flow::Value(type_), span))
    }

    fn untyped_numeric(expression: &Expression) -> bool {
        use BinaryOperator::*;

        match expression {
            Expression::Literal { value: Literal::Integer(_) | Literal::Float(_), .. } => true,
            Expression::Unary { operator: UnaryOperator::Negate | UnaryOperator::BitwiseNot, operand, .. } => {
                Self::untyped_numeric(operand)
            }
            Expression::Binary { left, operator, right, .. }
                if matches!(operator, Add | Subtract | Multiply | Divide | Modulo
                    | BitwiseAnd | BitwiseOr | BitwiseXor | LeftShift | RightShift) =>
            {
                Self::untyped_numeric(left) && Self::untyped_numeric(right)
            }
            _ => false,
        }
    }
}
