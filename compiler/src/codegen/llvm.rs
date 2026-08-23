use std::fmt::{Arguments, Write};

use crate::analyzer::checked::*;
use crate::analyzer::types::{IntWidth, ValueType};
use crate::common::Span;
use crate::parser::ast::{AssignmentOperator, BinaryOperator, UnaryOperator};

#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
    pub span: Option<Span>,
}

impl CodegenError {
    fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self { message: message.into(), span }
    }
}

pub type CodegenResult<T> = Result<T, CodegenError>;

pub fn emit_llvm(program: &CheckedProgram) -> CodegenResult<String> {
    let mut emitter = Emitter::default();

    for struct_ in &program.structs {
        emitter.emit_struct_type(struct_);
    }

    for function in &program.functions {
        emitter.emit_function(function)?;
    }

    if let Some(main) = program.functions.iter().find(|function| function.name == "main") {
        emitter.emit_entry(main)?;
    }

    Ok(emitter.finish())
}

#[derive(Default)]
struct Emitter {
    globals: String,
    functions: String,
    next_string: usize,
    next_value: usize,
    next_label: usize,
    current_block: String,
    terminated: bool,
    needs_trap: bool,
    needs_malloc: bool,
    needs_string_equal: bool,
}

impl Emitter {
    fn finish(self) -> String {
        let mut output = self.globals;

        if self.needs_trap {
            output.push_str("declare void @llvm.trap()\n\n");
        }

        if self.needs_malloc {
            writeln!(output, "declare ptr @malloc({})\n", Self::size_type()).unwrap();
        }

        if self.needs_string_equal {
            let size = Self::size_type();
            writeln!(output, "declare i32 @memcmp(ptr, ptr, {size})\n").unwrap();
            writeln!(output, "define internal i1 @tannin.runtime.string_equal(ptr %a, ptr %b) {{").unwrap();
            output.push_str("entry:\n");
            writeln!(output, "    %a.len = load {size}, ptr %a").unwrap();
            writeln!(output, "    %b.len = load {size}, ptr %b").unwrap();
            writeln!(output, "    %same.len = icmp eq {size} %a.len, %b.len").unwrap();
            output.push_str("    br i1 %same.len, label %compare, label %different\n");
            output.push_str("compare:\n");
            writeln!(output, "    %a.bytes = getelementptr {{ {size}, [0 x i8] }}, ptr %a, i32 0, i32 1").unwrap();
            writeln!(output, "    %b.bytes = getelementptr {{ {size}, [0 x i8] }}, ptr %b, i32 0, i32 1").unwrap();
            writeln!(output, "    %order = call i32 @memcmp(ptr %a.bytes, ptr %b.bytes, {size} %a.len)").unwrap();
            output.push_str("    %equal = icmp eq i32 %order, 0\n    ret i1 %equal\n");
            output.push_str("different:\n    ret i1 false\n}\n\n");
        }

        output.push_str(&self.functions);
        output
    }

    fn emit_function(&mut self, function: &CheckedFunction) -> CodegenResult<()> {
        self.next_value = 0;
        self.next_label = 0;
        let linkage = if function.pub_ { "" } else { "internal " };
        let return_type = Self::return_type(function.type_.return_type.as_ref());
        let parameters = function.type_.parameters.iter().enumerate()
            .map(|(index, type_)| format!("{} %arg.{index}", Self::type_(type_)))
            .collect::<Vec<_>>().join(", ");
        writeln!(self.functions, "define {linkage}{return_type} @tannin.{}({parameters}) {{", function.name).unwrap();
        self.start_block("entry");

        for (index, type_) in function.locals.iter().enumerate() {
            let type_ = Self::type_(type_);
            self.instruction(format_args!("%local.{index} = alloca {type_}"));
        }

        for (index, type_) in function.type_.parameters.iter().enumerate() {
            self.store(type_, &format!("%arg.{index}"), &format!("%local.{index}"));
        }

        let value = self.emit_block(&function.body)?;

        if !self.terminated {
            match value {
                Some(value) => self.terminate(format_args!("ret {return_type} {value}")),
                None => self.terminate(format_args!("ret void")),
            }
        }

        self.functions.push_str("}\n\n");

        Ok(())
    }

    fn emit_struct_type(&mut self, struct_: &CheckedStruct) {
        let fields = struct_.fields.iter().map(|(_, type_)| Self::type_(type_))
            .collect::<Vec<_>>().join(", ");

        writeln!(self.globals, "%tannin.struct.{} = type {{ {fields} }}", struct_.name).unwrap();
    }

    fn emit_entry(&mut self, main: &CheckedFunction) -> CodegenResult<()> {
        let result = main.type_.return_type.as_ref();

        if !main.type_.parameters.is_empty()
            || result.is_some_and(|type_| *type_ != ValueType::Integer { width: IntWidth::Bits(32), signed: true })
        {
            return Err(CodegenError::new("main must take no parameters and return i32 or no value", None));
        }

        self.functions.push_str("define i32 @main() {\nentry:\n");

        if result.is_some() {
            self.functions.push_str("    %result = call i32 @tannin.main()\n    ret i32 %result\n");
        } else {
            self.functions.push_str("    call void @tannin.main()\n    ret i32 0\n");
        }

        self.functions.push_str("}\n");
        Ok(())
    }

    fn emit_block(&mut self, block: &CheckedBlock) -> CodegenResult<Option<String>> {
        for statement in &block.statements {
            self.emit_statement(statement)?;

            if self.terminated {
                return Ok(None);
            }
        }

        match &block.tail {
            Some(expression) => self.emit_expression(expression),
            None => Ok(None),
        }
    }

    fn emit_statement(&mut self, statement: &CheckedStatement) -> CodegenResult<()> {
        match statement {
            CheckedStatement::Block(block) => {
                self.emit_block(block)?;
            }

            CheckedStatement::Let { index, initializer } => {
                let value = self.emit_value(initializer)?;
                self.store(initializer.value_type(), &value, &format!("%local.{index}"));
            }

            CheckedStatement::Return(expression) => match expression {
                Some(expression) => {
                    let value = self.emit_value(expression)?;
                    let type_ = Self::type_(expression.value_type());
                    self.terminate(format_args!("ret {type_} {value}"));
                }

                None => self.terminate(format_args!("ret void")),
            },

            CheckedStatement::Expression(expression) => {
                self.emit_expression(expression)?;
            }

            CheckedStatement::While { condition, body } => self.emit_while(condition, body)?,
        }

        Ok(())
    }

    fn emit_value(&mut self, expression: &CheckedExpression) -> CodegenResult<String> {
        self.emit_expression(expression)?.ok_or_else(|| {
            CodegenError::new("expected a lowered value", Some(expression.span))
        })
    }

    fn emit_expression(&mut self, expression: &CheckedExpression) -> CodegenResult<Option<String>> {
        let value = match &expression.kind {
            ExpressionKind::Literal(value) => self.literal(value),
            ExpressionKind::Local(_) | ExpressionKind::Member { .. } => {
                let address = self.emit_place(expression)?;
                self.load(expression.value_type(), &address)
            }
            ExpressionKind::Block(block) => return self.emit_block(block),
            ExpressionKind::Call { function, arguments } => {
                return self.emit_call(function, arguments, expression.flow.value());
            }
            ExpressionKind::Unary { operator, operand } => self.emit_unary(*operator, operand)?,
            ExpressionKind::Binary { left, operator, right } => {
                if matches!(operator, BinaryOperator::LogicalAnd | BinaryOperator::LogicalOr) {
                    return self.emit_logical(left, *operator, right).map(Some);
                }

                let left_value = self.emit_value(left)?;
                let right_value = self.emit_value(right)?;
                self.binary(*operator, left.value_type(), &left_value, &right_value)
            }
            ExpressionKind::Assignment { target, operator, value } => {
                self.emit_assignment(target, *operator, value)?
            }
            ExpressionKind::If { condition, then_branch, else_branch } => {
                return self.emit_if(
                    condition,
                    then_branch,
                    else_branch.as_deref(),
                    expression.flow.value(),
                );
            }
            ExpressionKind::Struct { name, fields } => self.emit_struct(name, fields)?,
        };

        Ok(Some(value))
    }

    fn emit_while(&mut self, condition: &CheckedExpression, body: &CheckedBlock) -> CodegenResult<()> {
        let test = self.label();
        let repeat = self.label();
        let done = self.label();

        self.branch(&test);
        self.start_block(&test);
        let condition = self.emit_value(condition)?;
        self.branch_if(&condition, &repeat, &done);

        self.start_block(&repeat);
        self.emit_block(body)?;

        if !self.terminated {
            self.branch(&test);
        }

        self.start_block(&done);
        Ok(())
    }

    fn emit_if(
        &mut self,
        condition: &CheckedExpression,
        then_branch: &CheckedExpression,
        else_branch: Option<&CheckedExpression>,
        type_: Option<&ValueType>,
    ) -> CodegenResult<Option<String>> {
        let condition = self.emit_value(condition)?;
        let then_label = self.label();
        let else_label = self.label();
        let end = self.label();
        let mut incoming = Vec::new();
        self.branch_if(&condition, &then_label, &else_label);

        self.start_block(&then_label);
        let then_value = self.emit_expression(then_branch)?;
        self.connect(&end, then_value, &mut incoming);

        self.start_block(&else_label);
        let else_value = match else_branch {
            Some(branch) => self.emit_expression(branch)?,
            None => None,
        };
        self.connect(&end, else_value, &mut incoming);

        Ok(self.join(&end, type_, incoming))
    }

    fn emit_logical(
        &mut self,
        left: &CheckedExpression,
        operator: BinaryOperator,
        right: &CheckedExpression,
    ) -> CodegenResult<String> {
        let value = self.emit_value(left)?;
        let test_right = self.label();
        let end = self.label();
        let shortcut = operator == BinaryOperator::LogicalOr;
        let mut incoming = vec![(Some(shortcut.to_string()), self.current_block.clone())];

        if shortcut {
            self.branch_if(&value, &end, &test_right);
        } else {
            self.branch_if(&value, &test_right, &end);
        }

        self.start_block(&test_right);
        let value = self.emit_value(right)?;
        self.connect(&end, Some(value), &mut incoming);

        Ok(self.join(&end, Some(&ValueType::Bool), incoming).expect("logical result"))
    }

    fn connect(&mut self, end: &str, value: Option<String>, incoming: &mut Vec<(Option<String>, String)>) {
        if !self.terminated {
            incoming.push((value, self.current_block.clone()));
            self.branch(end);
        }
    }

    fn join(
        &mut self,
        end: &str,
        type_: Option<&ValueType>,
        incoming: Vec<(Option<String>, String)>,
    ) -> Option<String> {
        if incoming.is_empty() {
            return None;
        }

        self.start_block(end);

        let type_ = Self::type_(type_?);
        let arguments = incoming.into_iter()
            .map(|(value, label)| format!("[ {}, %{label} ]", value.expect("branch value")))
            .collect::<Vec<_>>().join(", ");

        Some(self.value(format_args!("phi {type_} {arguments}")))
    }

    fn emit_call(
        &mut self,
        function: &str,
        arguments: &[CheckedExpression],
        return_type: Option<&ValueType>,
    ) -> CodegenResult<Option<String>> {
        let arguments = arguments.iter().map(|argument| {
            let type_ = Self::type_(argument.value_type());
            let value = self.emit_value(argument)?;
            Ok(format!("{type_} {value}"))
        }).collect::<CodegenResult<Vec<_>>>()?.join(", ");

        match return_type {
            Some(type_) => {
                let type_ = Self::type_(type_);
                Ok(Some(self.value(format_args!("call {type_} @tannin.{function}({arguments})"))))
            }

            None => {
                self.instruction(format_args!("call void @tannin.{function}({arguments})"));
                Ok(None)
            }
        }
    }

    fn emit_place(&mut self, expression: &CheckedExpression) -> CodegenResult<String> {
        match &expression.kind {
            ExpressionKind::Local(index) => Ok(format!("%local.{index}")),
            ExpressionKind::Member { object, index } => {
                let ValueType::Struct(name) = object.value_type() else {
                    unreachable!("checked field access");
                };
                let object = self.emit_value(object)?;

                Ok(self.field_address(name, &object, *index))
            }
            _ => Err(CodegenError::new("expected a storage location", Some(expression.span))),
        }
    }

    fn emit_struct(
        &mut self,
        name: &str,
        fields: &[(usize, CheckedExpression)],
    ) -> CodegenResult<String> {
        let object = self.allocate(&format!("%tannin.struct.{name}"));

        for (index, field) in fields {
            let value = self.emit_value(field)?;
            let address = self.field_address(name, &object, *index);
            self.store(field.value_type(), &value, &address);
        }

        Ok(object)
    }

    fn field_address(&mut self, name: &str, object: &str, index: usize) -> String {
        self.value(format_args!("getelementptr %tannin.struct.{name}, ptr {object}, i32 0, i32 {index}"))
    }

    fn allocate(&mut self, layout: &str) -> String {
        self.needs_malloc = true;
        let size = Self::size_type();
        let end = self.value(format_args!("getelementptr {layout}, ptr null, i32 1"));
        let bytes = self.value(format_args!("ptrtoint ptr {end} to {size}"));
        let empty = self.value(format_args!("icmp eq {size} {bytes}, 0"));
        let bytes = self.value(format_args!("select i1 {empty}, {size} 1, {size} {bytes}"));
        let object = self.value(format_args!("call ptr @malloc({size} {bytes})"));
        let failed = self.value(format_args!("icmp eq ptr {object}, null"));
        self.trap_if(&failed);

        object
    }

    fn emit_assignment(
        &mut self,
        target: &CheckedExpression,
        operator: AssignmentOperator,
        value: &CheckedExpression,
    ) -> CodegenResult<String> {
        use AssignmentOperator::*;

        let address = self.emit_place(target)?;
        let type_ = target.value_type();
        let operator = match operator {
            Assign => None,
            AddAssign => Some(BinaryOperator::Add),
            SubAssign => Some(BinaryOperator::Subtract),
            MulAssign => Some(BinaryOperator::Multiply),
            DivAssign => Some(BinaryOperator::Divide),
            ModAssign => Some(BinaryOperator::Modulo),
            AndAssign => Some(BinaryOperator::BitwiseAnd),
            OrAssign => Some(BinaryOperator::BitwiseOr),
            XorAssign => Some(BinaryOperator::BitwiseXor),
            LeftShiftAssign => Some(BinaryOperator::LeftShift),
            RightShiftAssign => Some(BinaryOperator::RightShift),
        };
        let value = if let Some(operator) = operator {
            let old = self.load(type_, &address);
            let right = self.emit_value(value)?;
            self.binary(operator, type_, &old, &right)
        } else {
            self.emit_value(value)?
        };
        self.store(type_, &value, &address);

        Ok(value)
    }

    fn emit_unary(&mut self, operator: UnaryOperator, operand: &CheckedExpression) -> CodegenResult<String> {
        use UnaryOperator::*;

        let type_ = operand.value_type();
        let llvm_type = Self::type_(type_);
        let value = match operator {
            Negate => {
                let operand = self.emit_value(operand)?;

                if matches!(type_, ValueType::Float(_)) {
                    self.value(format_args!("fneg {llvm_type} {operand}"))
                } else {
                    self.value(format_args!("sub {llvm_type} 0, {operand}"))
                }
            }

            Not | BitwiseNot => {
                let operand = self.emit_value(operand)?;
                self.value(format_args!("xor {llvm_type} {operand}, -1"))
            }

            PreIncrement | PreDecrement | PostIncrement | PostDecrement => {
                let address = self.emit_place(operand)?;
                let old = self.load(type_, &address);
                let one = if matches!(type_, ValueType::Float(_)) { "0x3FF0000000000000" } else { "1" };
                let op = if matches!(operator, PreIncrement | PostIncrement) {
                    BinaryOperator::Add
                } else {
                    BinaryOperator::Subtract
                };
                let new = self.binary(op, type_, &old, one);
                self.store(type_, &new, &address);

                if matches!(operator, PostIncrement | PostDecrement) { old } else { new }
            }
        };

        Ok(value)
    }

    fn binary(&mut self, operator: BinaryOperator, type_: &ValueType, left: &str, right: &str) -> String {
        use BinaryOperator::*;

        let llvm_type = Self::type_(type_);

        if *type_ == ValueType::String {
            self.needs_string_equal = true;
            let equal = self.value(format_args!("call i1 @tannin.runtime.string_equal(ptr {left}, ptr {right})"));
            return if operator == NotEqual {
                self.value(format_args!("xor i1 {equal}, true"))
            } else {
                equal
            };
        }

        let instruction = if matches!(type_, ValueType::Float(_)) {
            match operator {
                Add => "fadd",
                Subtract => "fsub",
                Multiply => "fmul",
                Divide => "fdiv",
                Modulo => "frem",
                Equal => "fcmp oeq",
                NotEqual => "fcmp une",
                Less => "fcmp olt",
                LessEqual => "fcmp ole",
                Greater => "fcmp ogt",
                GreaterEqual => "fcmp oge",
                _ => unreachable!("checked floating-point operator"),
            }
        } else {
            let signed = matches!(type_, ValueType::Integer { signed: true, .. });

            if let ValueType::Integer { width, .. } = type_ {
                let bits = width.bits();
                if matches!(operator, Divide | Modulo) {
                    let zero = self.value(format_args!("icmp eq {llvm_type} {right}, 0"));
                    let invalid = if signed {
                        let minimum = -(1_i128 << (bits - 1));
                        let min_left = self.value(format_args!("icmp eq {llvm_type} {left}, {minimum}"));
                        let neg_right = self.value(format_args!("icmp eq {llvm_type} {right}, -1"));
                        let overflow = self.value(format_args!("and i1 {min_left}, {neg_right}"));
                        self.value(format_args!("or i1 {zero}, {overflow}"))
                    } else {
                        zero
                    };
                    self.trap_if(&invalid);
                } else if matches!(operator, LeftShift | RightShift) {
                    let invalid = self.value(format_args!("icmp uge {llvm_type} {right}, {bits}"));
                    self.trap_if(&invalid);
                }
            }

            match operator {
                Add => "add",
                Subtract => "sub",
                Multiply => "mul",
                Divide if signed => "sdiv",
                Divide => "udiv",
                Modulo if signed => "srem",
                Modulo => "urem",
                Equal => "icmp eq",
                NotEqual => "icmp ne",
                Less if signed => "icmp slt",
                Less => "icmp ult",
                LessEqual if signed => "icmp sle",
                LessEqual => "icmp ule",
                Greater if signed => "icmp sgt",
                Greater => "icmp ugt",
                GreaterEqual if signed => "icmp sge",
                GreaterEqual => "icmp uge",
                BitwiseAnd => "and",
                BitwiseOr => "or",
                BitwiseXor => "xor",
                LeftShift => "shl",
                RightShift if signed => "ashr",
                RightShift => "lshr",
                _ => unreachable!("checked integer operator"),
            }
        };

        self.value(format_args!("{instruction} {llvm_type} {left}, {right}"))
    }

    fn literal(&mut self, literal: &CheckedLiteral) -> String {
        match literal {
            CheckedLiteral::Integer(value) => value.to_string(),
            CheckedLiteral::Float(value) => format!("0x{:016X}", value.to_bits()),
            CheckedLiteral::Bool(value) => value.to_string(),
            CheckedLiteral::Char(value) => u32::from(*value).to_string(),
            CheckedLiteral::String(value) => {
                let name = format!("@tannin.str.{}", self.next_string);
                self.next_string += 1;
                let size = Self::size_type();
                let length = value.len();
                let bytes = value.bytes().fold(String::new(), |mut bytes, byte| {
                    write!(bytes, "\\{byte:02X}").unwrap();
                    bytes
                });
                writeln!(
                    self.globals,
                    "{name} = private unnamed_addr constant {{ {size}, [{length} x i8] }} {{ {size} {length}, [{length} x i8] c\"{bytes}\" }}",
                ).unwrap();

                name
            }
        }
    }

    fn type_(type_: &ValueType) -> String {
        match type_ {
            ValueType::Integer { width, .. } => format!("i{}", width.bits()),
            ValueType::Float(32) => "float".to_owned(),
            ValueType::Float(_) => "double".to_owned(),
            ValueType::Bool => "i1".to_owned(),
            ValueType::Char => "i32".to_owned(),
            ValueType::String | ValueType::Struct(_) => "ptr".to_owned(),
        }
    }

    fn return_type(type_: Option<&ValueType>) -> String {
        type_.map_or_else(|| "void".to_owned(), Self::type_)
    }

    fn size_type() -> String {
        format!("i{}", usize::BITS)
    }

    fn load(&mut self, type_: &ValueType, address: &str) -> String {
        let type_ = Self::type_(type_);
        self.value(format_args!("load {type_}, ptr {address}"))
    }

    fn store(&mut self, type_: &ValueType, value: &str, address: &str) {
        let type_ = Self::type_(type_);
        self.instruction(format_args!("store {type_} {value}, ptr {address}"));
    }

    fn value(&mut self, instruction: Arguments<'_>) -> String {
        let name = format!("%v.{}", self.next_value);
        self.next_value += 1;
        self.instruction(format_args!("{name} = {instruction}"));

        name
    }

    fn instruction(&mut self, instruction: Arguments<'_>) {
        writeln!(self.functions, "    {instruction}").unwrap();
    }

    fn terminate(&mut self, instruction: Arguments<'_>) {
        self.instruction(instruction);
        self.terminated = true;
    }

    fn label(&mut self) -> String {
        let label = format!("bb.{}", self.next_label);
        self.next_label += 1;
        label
    }

    fn start_block(&mut self, label: &str) {
        writeln!(self.functions, "{label}:").unwrap();
        self.current_block = label.to_owned();
        self.terminated = false;
    }

    fn branch(&mut self, label: &str) {
        self.terminate(format_args!("br label %{label}"));
    }

    fn branch_if(&mut self, condition: &str, then_label: &str, else_label: &str) {
        self.terminate(format_args!("br i1 {condition}, label %{then_label}, label %{else_label}"));
    }

    fn trap_if(&mut self, condition: &str) {
        self.needs_trap = true;
        let trap = self.label();
        let next = self.label();
        self.branch_if(condition, &trap, &next);
        self.start_block(&trap);
        self.instruction(format_args!("call void @llvm.trap()"));
        self.terminate(format_args!("unreachable"));
        self.start_block(&next);
    }
}
