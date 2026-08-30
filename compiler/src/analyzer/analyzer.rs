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
    structs: Vec<CheckedStruct>,
    enums: Vec<Vec<String>>,
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
            structs: Vec::new(),
            enums: Vec::new(),
            locals: Vec::new(),
            return_type: None,
        };

        analyzer.collect_declarations(program)?;
        analyzer.collect_structs(program)?;
        analyzer.collect_signatures(program)?;
        analyzer.collect_constants(program)?;

        let mut functions = Vec::new();

        for declaration in &program.declarations {
            if let Declaration::Function(function) = declaration {
                functions.push(analyzer.check_function(function)?);
            }
        }

        Ok(CheckedProgram { functions, structs: analyzer.structs })
    }

    fn collect_declarations(&mut self, program: &Program) -> AnalyzeResult<()> {
        for declaration in &program.declarations {
            let (name, span, symbol) = match declaration {
                Declaration::Function(function) => (
                    &function.name,
                    function.span,
                    Symbol::Function { span: function.span },
                ),

                Declaration::Struct(struct_) => {
                    if ValueType::named(&struct_.name).is_some() {
                        return Err(AnalyzeError::new(
                            format!("'{}' is a built-in type", struct_.name),
                            struct_.span,
                        ));
                    }

                    let index = self.structs.len();
                    self.structs.push(CheckedStruct {
                        name: struct_.name.clone(),
                        fields: Vec::new(),
                    });

                    (&struct_.name, struct_.span, Symbol::Struct { index, span: struct_.span })
                }

                Declaration::Enum(enum_) => {
                    if ValueType::named(&enum_.name).is_some() {
                        return Err(AnalyzeError::new(
                            format!("'{}' is a built-in type", enum_.name),
                            enum_.span,
                        ));
                    }

                    let mut variants = Vec::new();

                    for variant in &enum_.variants {
                        if variants.contains(&variant.name) {
                            return Err(AnalyzeError::new(
                                format!("duplicate variant '{}'", variant.name),
                                variant.span,
                            ));
                        }

                        variants.push(variant.name.clone());
                    }

                    let index = self.enums.len();
                    self.enums.push(variants);

                    (&enum_.name, enum_.span, Symbol::Enum { index, span: enum_.span })
                }

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

    fn collect_structs(&mut self, program: &Program) -> AnalyzeResult<()> {
        for declaration in &program.declarations {
            let Declaration::Struct(struct_) = declaration else {
                continue;
            };

            let index = self.struct_index(&struct_.name, struct_.span)?;
            let mut fields = Vec::new();

            for field in &struct_.fields {
                if fields.iter().any(|(name, _)| *name == field.name) {
                    return Err(AnalyzeError::new(
                        format!("duplicate field '{}'", field.name),
                        field.span,
                    ));
                }

                fields.push((field.name.clone(), self.resolve_type(&field.type_)?));
            }

            self.structs[index].fields = fields;
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
                Expression::Literal { .. } | Expression::EnumVariant { .. } => true,
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
            Type::Named { name, span } => {
                if let Some(type_) = ValueType::named(name) {
                    return Ok(type_);
                }

                match self.globals.lookup(name) {
                    Some(Symbol::Struct { .. }) => Ok(ValueType::Struct(name.clone())),
                    Some(Symbol::Enum { .. }) => Ok(ValueType::Enum(name.clone())),
                    _ => Err(AnalyzeError::new(format!("unknown type '{name}'"), *span)),
                }
            }
            Type::Array { element, .. } => Ok(ValueType::Array(Box::new(self.resolve_type(element)?))),
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

            Statement::While { condition, body, .. } => {
                let condition = self.check_value(condition, Some(&ValueType::Bool))?;
                let body = self.check_block(body, None)?;

                Ok(CheckedStatement::While { condition, body })
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
            Expression::Assignment { target, operator, value, .. } => {
                self.check_assignment(target, *operator, value, span)?
            }
            Expression::If { condition, then_branch, else_branch, .. } => {
                self.check_if(condition, then_branch, else_branch.as_deref(), expected, span)?
            }
            Expression::Struct { name, fields, .. } => self.check_struct(name, fields, span)?,
            Expression::Member { object, member, .. } => self.check_member(object, member, span)?,
            Expression::Array { elements, .. } => self.check_array(elements, expected, span)?,
            Expression::Index { object, index, .. } => self.check_index(object, index, span)?,
            Expression::EnumVariant { enum_, variant, .. } => {
                let literal = self.enum_variant(enum_, variant, span)?;
                CheckedExpression::new(
                    ExpressionKind::Literal(literal),
                    Flow::Value(ValueType::Enum(enum_.clone())),
                    span,
                )
            }
            Expression::Block(block) => {
                let block = self.check_block(block, expected)?;
                let flow = block.flow.clone();
                CheckedExpression::new(ExpressionKind::Block(block), flow, span)
            }
            Expression::Match { expression, arms, .. } => {
                self.check_match(expression, arms, expected, span)?
            }
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

        let update = matches!(operator,
            UnaryOperator::PreIncrement | UnaryOperator::PreDecrement
                | UnaryOperator::PostIncrement | UnaryOperator::PostDecrement
        );
        let operand = if update {
            self.check_target(operand)?
        } else {
            self.check_value(operand, expected)?
        };
        let type_ = operand.value_type().clone();
        let valid = match operator {
            UnaryOperator::Negate => matches!(type_, ValueType::Integer { signed: true, .. } | ValueType::Float(_)),
            UnaryOperator::Not => type_ == ValueType::Bool,
            UnaryOperator::BitwiseNot => type_.is_integer(),
            UnaryOperator::PreIncrement | UnaryOperator::PreDecrement
                | UnaryOperator::PostIncrement | UnaryOperator::PostDecrement => type_.is_numeric(),
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
            Equal | NotEqual => !matches!(type_, ValueType::Struct(_) | ValueType::Array(_)),
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
            Expression::Block(block) => block.tail.as_deref().is_some_and(Self::untyped_numeric),
            _ => false,
        }
    }

    fn check_target(&mut self, target: &Expression) -> AnalyzeResult<CheckedExpression> {
        let mut root = target;

        while let Expression::Member { object, .. } | Expression::Index { object, .. } = root {
            root = object;
        }

        let Expression::Identifier { value: name, span } = root else {
            return Err(AnalyzeError::new("expected an assignment target", target.span()));
        };

        match self.lookup(name) {
            Some(Symbol::Variable { mutable: true, .. }) => self.check_value(target, None),
            Some(Symbol::Variable { mutable: false, .. } | Symbol::Const { .. }) => {
                Err(AnalyzeError::new(format!("cannot assign to immutable binding '{name}'"), *span))
            }
            Some(_) => Err(AnalyzeError::new(format!("'{name}' is not a variable"), *span)),
            None => Err(AnalyzeError::new(format!("unknown name '{name}'"), *span)),
        }
    }

    fn check_assignment(
        &mut self,
        target: &Expression,
        operator: AssignmentOperator,
        value: &Expression,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        use AssignmentOperator::*;

        let target = self.check_target(target)?;
        let type_ = target.value_type().clone();
        let valid = match operator {
            Assign => true,
            AddAssign | SubAssign | MulAssign | DivAssign | ModAssign => type_.is_numeric(),
            AndAssign | OrAssign | XorAssign | LeftShiftAssign | RightShiftAssign => type_.is_integer(),
        };

        if !valid {
            return Err(AnalyzeError::new(format!("invalid assignment operator for {type_}"), span));
        }

        let value = self.check_value(value, Some(&type_))?;
        let kind = ExpressionKind::Assignment {
            target: Box::new(target),
            operator,
            value: Box::new(value),
        };

        Ok(CheckedExpression::new(kind, Flow::Value(type_), span))
    }

    fn check_if(
        &mut self,
        condition: &Expression,
        then_branch: &Expression,
        else_branch: Option<&Expression>,
        expected: Option<&ValueType>,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        let condition = self.check_value(condition, Some(&ValueType::Bool))?;
        let (then_branch, else_branch) = match else_branch {
            Some(else_branch)
                if expected.is_none() && Self::untyped_numeric(then_branch)
                    && !Self::untyped_numeric(else_branch) =>
            {
                let else_branch = self.check_expression(else_branch, None)?;
                let then_branch = self.check_expression(then_branch, else_branch.flow.value())?;
                (then_branch, Some(else_branch))
            }

            _ => {
                let then_branch = self.check_expression(then_branch, expected)?;
                let else_branch = else_branch.map(|branch| {
                    self.check_expression(branch, expected.or_else(|| then_branch.flow.value()))
                }).transpose()?;
                (then_branch, else_branch)
            }
        };
        let flow = Self::merge_flows(
            &then_branch.flow,
            else_branch.as_ref().map_or(&Flow::NoValue, |branch| &branch.flow),
            span,
        )?;
        let kind = ExpressionKind::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        };

        Ok(CheckedExpression::new(kind, flow, span))
    }

    fn merge_flows(left: &Flow, right: &Flow, span: Span) -> AnalyzeResult<Flow> {
        match (left, right) {
            (Flow::Returns, flow) | (flow, Flow::Returns) => Ok(flow.clone()),
            (left, right) if left == right => Ok(left.clone()),
            _ => Err(AnalyzeError::new("branches must produce the same type", span)),
        }
    }

    fn struct_index(&self, name: &str, span: Span) -> AnalyzeResult<usize> {
        match self.globals.lookup(name) {
            Some(Symbol::Struct { index, .. }) => Ok(*index),
            _ => Err(AnalyzeError::new(format!("unknown struct '{name}'"), span)),
        }
    }

    fn struct_field(&self, struct_: usize, name: &str, span: Span) -> AnalyzeResult<(usize, ValueType)> {
        let struct_ = &self.structs[struct_];
        struct_.fields.iter().enumerate()
            .find(|(_, (field, _))| field == name)
            .map(|(index, (_, type_))| (index, type_.clone()))
            .ok_or_else(|| AnalyzeError::new(format!("'{}' has no field '{name}'", struct_.name), span))
    }

    fn check_struct(
        &mut self,
        name: &str,
        fields: &[FieldInitializer],
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        let struct_ = self.struct_index(name, span)?;
        let mut initialized = vec![false; self.structs[struct_].fields.len()];
        let mut values = Vec::new();

        for field in fields {
            let (index, type_) = self.struct_field(struct_, &field.name, field.span)?;

            if initialized[index] {
                return Err(AnalyzeError::new(
                    format!("duplicate initializer for '{}'", field.name),
                    field.span,
                ));
            }

            initialized[index] = true;
            values.push((index, self.check_value(&field.value, Some(&type_))?));
        }

        if let Some(index) = initialized.iter().position(|&initialized| !initialized) {
            let (field, _) = &self.structs[struct_].fields[index];
            return Err(AnalyzeError::new(format!("missing initializer for '{field}'"), span));
        }

        let kind = ExpressionKind::Struct { name: name.to_owned(), fields: values };

        Ok(CheckedExpression::new(kind, Flow::Value(ValueType::Struct(name.to_owned())), span))
    }

    fn check_member(
        &mut self,
        object: &Expression,
        member: &str,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        let object = self.check_value(object, None)?;
        let ValueType::Struct(name) = object.value_type() else {
            return Err(AnalyzeError::new("field access requires a struct", span));
        };
        let (index, type_) = self.struct_field(self.struct_index(name, span)?, member, span)?;
        let kind = ExpressionKind::Member { object: Box::new(object), index };

        Ok(CheckedExpression::new(kind, Flow::Value(type_), span))
    }

    fn check_array(
        &mut self,
        elements: &[Expression],
        expected: Option<&ValueType>,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        let (element_type, mut inferred) = if let Some(ValueType::Array(element)) = expected {
            ((**element).clone(), None)
        } else {
            let (index, seed) = elements.iter().enumerate()
                .find(|(_, element)| !Self::contextual_element(element))
                .or_else(|| elements.iter().enumerate().find(|(_, element)| {
                    !matches!(element, Expression::Array { elements, .. } if elements.is_empty())
                }))
                .ok_or_else(|| AnalyzeError::new("empty array needs an element type", span))?;
            let seed = self.check_value(seed, None)?;

            (seed.value_type().clone(), Some((index, seed)))
        };
        let mut values = Vec::new();

        for (index, element) in elements.iter().enumerate() {
            if inferred.as_ref().is_some_and(|(seed, _)| *seed == index) {
                values.push(inferred.take().expect("inferred element").1);
            } else {
                values.push(self.check_value(element, Some(&element_type))?);
            }
        }

        let type_ = ValueType::Array(Box::new(element_type));

        Ok(CheckedExpression::new(ExpressionKind::Array(values), Flow::Value(type_), span))
    }

    fn contextual_element(expression: &Expression) -> bool {
        match expression {
            Expression::Array { elements, .. } => elements.iter().all(Self::contextual_element),
            _ => Self::untyped_numeric(expression),
        }
    }

    fn check_index(
        &mut self,
        object: &Expression,
        index: &Expression,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        let object = self.check_value(object, None)?;
        let ValueType::Array(element) = object.value_type() else {
            return Err(AnalyzeError::new("indexing requires an array", span));
        };
        let index = self.check_value(index, None)?;

        if !index.value_type().is_integer() {
            return Err(AnalyzeError::new("array index must be an integer", index.span));
        }

        let type_ = (**element).clone();
        let kind = ExpressionKind::Index { object: Box::new(object), index: Box::new(index) };

        Ok(CheckedExpression::new(kind, Flow::Value(type_), span))
    }

    fn enum_variants(&self, name: &str, span: Span) -> AnalyzeResult<&[String]> {
        match self.globals.lookup(name) {
            Some(Symbol::Enum { index, .. }) => Ok(&self.enums[*index]),
            _ => Err(AnalyzeError::new(format!("unknown enum '{name}'"), span)),
        }
    }

    fn enum_variant(&self, name: &str, variant: &str, span: Span) -> AnalyzeResult<CheckedLiteral> {
        self.enum_variants(name, span)?.iter()
            .position(|known| known == variant)
            .map(CheckedLiteral::EnumVariant)
            .ok_or_else(|| AnalyzeError::new(format!("'{name}' has no variant '{variant}'"), span))
    }

    fn check_match(
        &mut self,
        expression: &Expression,
        arms: &[MatchArm],
        expected: Option<&ValueType>,
        span: Span,
    ) -> AnalyzeResult<CheckedExpression> {
        if arms.is_empty() {
            return Err(AnalyzeError::new("match needs at least one arm", span));
        }

        let expression = self.check_value(expression, None)?;
        let type_ = expression.value_type();
        let alternatives = match type_ {
            ValueType::Bool => Some(2),
            ValueType::Enum(name) => Some(self.enum_variants(name, span)?.len()),
            _ => None,
        };
        let mut inferred = if expected.is_none() {
            arms.iter().enumerate()
                .find(|(_, arm)| !Self::contextual_element(&arm.value))
                .map(|(index, arm)| self.check_match_arm(arm, type_, None).map(|arm| (index, arm)))
                .transpose()?
        } else {
            None
        };
        let inferred_type = inferred.as_ref().and_then(|(_, arm)| arm.value.flow.value()).cloned();
        let mut covered = Vec::new();
        let mut exhaustive = false;
        let mut values = Vec::new();
        let mut flow = Flow::Returns;

        for (index, arm) in arms.iter().enumerate() {
            if exhaustive {
                return Err(AnalyzeError::new("unreachable match arm", arm.span));
            }

            let expected = expected.or(inferred_type.as_ref()).or_else(|| flow.value());
            let checked = if inferred.as_ref().is_some_and(|(seed, _)| *seed == index) {
                inferred.take().expect("inferred arm").1
            } else {
                self.check_match_arm(arm, type_, expected)?
            };

            match &checked.pattern {
                CheckedPattern::Literal(literal) => {
                    if covered.contains(literal) {
                        return Err(AnalyzeError::new("duplicate match pattern", arm.pattern.span()));
                    }

                    covered.push(literal.clone());
                    exhaustive = alternatives == Some(covered.len());
                }

                CheckedPattern::Binding(_) | CheckedPattern::Wildcard => exhaustive = true,
            }

            flow = Self::merge_flows(&flow, &checked.value.flow, arm.value.span())?;
            values.push(checked);
        }

        if !exhaustive {
            return Err(AnalyzeError::new("non-exhaustive match", span));
        }

        let kind = ExpressionKind::Match { expression: Box::new(expression), arms: values };

        Ok(CheckedExpression::new(kind, flow, span))
    }

    fn check_match_arm(
        &mut self,
        arm: &MatchArm,
        scrutinee: &ValueType,
        expected: Option<&ValueType>,
    ) -> AnalyzeResult<CheckedArm> {
        self.scopes.push(Scope::new());
        let result = self.check_pattern(&arm.pattern, scrutinee).and_then(|pattern| {
            let value = self.check_expression(&arm.value, expected)?;
            Ok(CheckedArm { pattern, value })
        });
        self.scopes.pop();

        result
    }

    fn check_pattern(&mut self, pattern: &Pattern, type_: &ValueType) -> AnalyzeResult<CheckedPattern> {
        let literal = match pattern {
            Pattern::Wildcard { .. } => return Ok(CheckedPattern::Wildcard),

            Pattern::Identifier { value, span } => {
                let index = self.declare_local(value, type_.clone(), false, *span)?;
                return Ok(CheckedPattern::Binding(index));
            }

            Pattern::Literal { value, span } => {
                let expression = self.check_literal(value, false, Some(type_), *span)?;
                let actual = expression.value_type();

                if actual != type_ {
                    return Err(AnalyzeError::new(format!("expected {type_} pattern, found {actual}"), *span));
                }

                let ExpressionKind::Literal(literal) = expression.kind else {
                    unreachable!("checked literal pattern");
                };
                literal
            }

            Pattern::EnumVariant { enum_, variant, span } => {
                if !matches!(type_, ValueType::Enum(name) if name == enum_) {
                    return Err(AnalyzeError::new(format!("expected {type_} pattern, found {enum_}"), *span));
                }

                self.enum_variant(enum_, variant, *span)?
            }
        };

        Ok(CheckedPattern::Literal(literal))
    }
}
