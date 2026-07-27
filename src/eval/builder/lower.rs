//! Rewriting and lowering of expression graphs.
//!
//! Rewrites are applied at the root of an operation before its arguments are
//! visited, and are matched against the arguments as they were built. A
//! rewritten operation is never rewritten a second time.

use std::collections::HashMap;

use indexmap::IndexMap;
use rug::{Float, Rational};

use super::{Expression, ExpressionBuilder};
use crate::eval::instructions::{
    BinaryOp, ConstantOp, Instruction, InstructionData, TernaryOp, UnaryOp, UnaryParamOp,
};

/// A register program ready for evaluation.
pub(crate) struct Program {
    pub arguments: Vec<String>,
    pub instructions: Vec<Instruction>,
    pub outputs: Vec<usize>,
}

pub(super) fn lower(builder: &ExpressionBuilder, outputs: &[Expression]) -> Program {
    let mut optimizer = Optimizer::new(builder);
    let outputs = outputs
        .iter()
        .map(|output| optimizer.optimize(output.register))
        .collect();
    let instructions = optimizer
        .instructions
        .into_iter()
        .map(|(data, out)| Instruction { out, data })
        .collect();

    Program {
        arguments: builder.arguments.clone(),
        instructions,
        outputs,
    }
}

struct Optimizer<'a> {
    source: &'a ExpressionBuilder,
    optimized: HashMap<usize, usize>,
    promoted: HashMap<usize, usize>,
    instructions: IndexMap<InstructionData, usize>,
}

impl<'a> Optimizer<'a> {
    fn new(source: &'a ExpressionBuilder) -> Self {
        Self {
            source,
            optimized: HashMap::new(),
            promoted: HashMap::new(),
            instructions: IndexMap::new(),
        }
    }

    /// Lower a source register, rewriting the operation at its root.
    fn optimize(&mut self, register: usize) -> usize {
        if register < self.source.arguments.len() {
            return register;
        }
        if let Some(&optimized) = self.optimized.get(&register) {
            return optimized;
        }

        let optimized = match self.source_data(register) {
            InstructionData::Unary { op, arg } => self.optimize_unary(op, arg),
            InstructionData::Binary { op, lhs, rhs } => self.optimize_binary(op, lhs, rhs),
            InstructionData::Ternary {
                op,
                arg1,
                arg2,
                arg3,
            } => self.optimize_ternary(op, arg1, arg2, arg3),
            data => self.emit(data),
        };
        self.optimized.insert(register, optimized);
        optimized
    }

    /// Lower a source register without rewriting the operation at its root.
    ///
    /// A rewrite that replaces an operation by one of its arguments, such as
    /// `log(exp(x)) => x`, leaves that argument at the root of the result.
    fn promote(&mut self, register: usize) -> usize {
        if register < self.source.arguments.len() {
            return register;
        }
        if let Some(&promoted) = self.promoted.get(&register) {
            return promoted;
        }

        let data = self.source_data(register);
        let promoted = self.emit(data);
        self.promoted.insert(register, promoted);
        promoted
    }

    /// Add an instruction whose arguments are still source registers.
    fn emit(&mut self, data: InstructionData) -> usize {
        let data = match data {
            InstructionData::Unary { op, arg } => InstructionData::unary(op, self.optimize(arg)),
            InstructionData::UnaryParam { op, param, arg } => {
                InstructionData::unary_param(op, param, self.optimize(arg))
            }
            InstructionData::Binary { op, lhs, rhs } => {
                let lhs = self.optimize(lhs);
                let rhs = self.optimize(rhs);
                InstructionData::binary(op, lhs, rhs)
            }
            InstructionData::Ternary {
                op,
                arg1,
                arg2,
                arg3,
            } => {
                let arg1 = self.optimize(arg1);
                let arg2 = self.optimize(arg2);
                let arg3 = self.optimize(arg3);
                InstructionData::ternary(op, arg1, arg2, arg3)
            }
            leaf => leaf,
        };
        self.insert(data)
    }

    fn emit_unary(&mut self, op: UnaryOp, arg: usize) -> usize {
        self.emit(InstructionData::unary(op, arg))
    }

    fn emit_binary(&mut self, op: BinaryOp, lhs: usize, rhs: usize) -> usize {
        self.emit(InstructionData::binary(op, lhs, rhs))
    }

    /// Add an instruction whose arguments are already lowered.
    fn insert(&mut self, data: InstructionData) -> usize {
        let next_register = self.source.arguments.len() + self.instructions.len();
        *self.instructions.entry(data).or_insert(next_register)
    }

    fn optimize_unary(&mut self, op: UnaryOp, arg: usize) -> usize {
        match op {
            UnaryOp::Sqrt => self.optimize_sqrt(arg),
            UnaryOp::Exp => self.optimize_exp(arg),
            UnaryOp::Log => self.optimize_log(arg),
            UnaryOp::Sin => self.optimize_trig(UnaryOp::Sin, UnaryParamOp::Sinu, arg),
            UnaryOp::Cos => self.optimize_trig(UnaryOp::Cos, UnaryParamOp::Cosu, arg),
            UnaryOp::Tan => self.optimize_trig(UnaryOp::Tan, UnaryParamOp::Tanu, arg),
            _ => self.emit_unary(op, arg),
        }
    }

    fn optimize_binary(&mut self, op: BinaryOp, lhs: usize, rhs: usize) -> usize {
        match op {
            BinaryOp::Pow => self.optimize_pow(lhs, rhs),
            BinaryOp::Sub => self.optimize_sub(lhs, rhs),
            _ => self.emit_binary(op, lhs, rhs),
        }
    }

    fn optimize_ternary(&mut self, op: TernaryOp, arg1: usize, arg2: usize, arg3: usize) -> usize {
        match op {
            // fma(x, y, z) => x * y + z
            TernaryOp::Fma => {
                let lhs = self.optimize(arg1);
                let rhs = self.optimize(arg2);
                let product = self.insert(InstructionData::binary(BinaryOp::Mul, lhs, rhs));
                let addend = self.optimize(arg3);
                self.insert(InstructionData::binary(BinaryOp::Add, product, addend))
            }
            TernaryOp::If => self.emit(InstructionData::ternary(op, arg1, arg2, arg3)),
        }
    }

    fn optimize_sqrt(&mut self, arg: usize) -> usize {
        if let Some((lhs, rhs)) = self.binary_args(arg, BinaryOp::Add) {
            // sqrt(x^2 + y^2) => hypot(x, y)
            if let (Some(x), Some(y)) = (self.square_base(lhs), self.square_base(rhs)) {
                return self.emit_binary(BinaryOp::Hypot, x, y);
            }
            // sqrt(x^2 + 1) => hypot(x, 1)
            if let Some(x) = self.square_base(lhs)
                && self.is_exact_literal(rhs, 1.0)
            {
                return self.emit_binary(BinaryOp::Hypot, x, rhs);
            }
            // sqrt(1 + x^2) => hypot(1, x)
            if self.is_exact_literal(lhs, 1.0)
                && let Some(y) = self.square_base(rhs)
            {
                return self.emit_binary(BinaryOp::Hypot, lhs, y);
            }
        }

        self.emit_unary(UnaryOp::Sqrt, arg)
    }

    fn optimize_exp(&mut self, arg: usize) -> usize {
        // exp(log(x)) => x, which is invalid for non-positive x
        if let Some(value) = self.unary_arg(arg, UnaryOp::Log) {
            let value = self.optimize(value);
            let zero = self.insert(InstructionData::literal(Float::with_val(53, 0)));
            let positive = self.insert(InstructionData::binary(BinaryOp::Gt, value, zero));
            let valid = self.insert(InstructionData::unary(UnaryOp::Assert, positive));
            return self.insert(InstructionData::ternary(TernaryOp::If, valid, value, value));
        }

        self.emit_unary(UnaryOp::Exp, arg)
    }

    fn optimize_log(&mut self, arg: usize) -> usize {
        // log(exp(x)) => x
        if let Some(value) = self.unary_arg(arg, UnaryOp::Exp) {
            return self.promote(value);
        }
        // log(1 + x) or log(x + 1) => log1p(x)
        if let Some((lhs, rhs)) = self.binary_args(arg, BinaryOp::Add) {
            if self.is_exact_literal(lhs, 1.0) {
                return self.emit_unary(UnaryOp::Log1p, rhs);
            }
            if self.is_exact_literal(rhs, 1.0) {
                return self.emit_unary(UnaryOp::Log1p, lhs);
            }
        }

        self.emit_unary(UnaryOp::Log, arg)
    }

    // sin(PI * x) => sinu(2, x), and likewise for cos and tan
    fn optimize_trig(&mut self, op: UnaryOp, unit_op: UnaryParamOp, arg: usize) -> usize {
        if let Some((unit, value)) = self.trig_unit(arg) {
            let value = self.optimize(value);
            self.insert(InstructionData::unary_param(unit_op, unit, value))
        } else {
            self.emit_unary(op, arg)
        }
    }

    fn optimize_pow(&mut self, base: usize, exponent: usize) -> usize {
        // pow(x, 2) => pow2(x)
        if self.literal_rounds_to(exponent, 2.0) {
            return self.emit_unary(UnaryOp::Pow2, base);
        }
        // pow(x, 0.5) => sqrt(x)
        if self.literal_rounds_to(exponent, 0.5) {
            return self.emit_unary(UnaryOp::Sqrt, base);
        }

        if let Some(value) = self.rational_value(exponent) {
            let numerator = value.numer();
            let denominator = value.denom();
            // pow(x, 1/3) => cbrt(x)
            if *numerator == 1 && *denominator == 3 {
                return self.emit_unary(UnaryOp::Cbrt, base);
            }
            // pow(x, 1/2) => sqrt(x)
            if *numerator == 1 && *denominator == 2 {
                return self.emit_unary(UnaryOp::Sqrt, base);
            }
            // pow(x, 2) => pow2(x)
            if *numerator == 2 && *denominator == 1 {
                return self.emit_unary(UnaryOp::Pow2, base);
            }
            // Integer exponents and non-negative bases need no sign handling.
            if *denominator == 1 || self.is_unary(base, UnaryOp::Fabs) {
                return self.emit_binary(BinaryOp::Pow, base, exponent);
            }
            // pow(x, p/q) => pow(fabs(x), p/q), with the sign of x restored for odd p
            if denominator.is_odd() {
                let optimized_base = self.optimize(base);
                let magnitude = self.insert(InstructionData::unary(UnaryOp::Fabs, optimized_base));
                let exponent = self.optimize(exponent);
                let power =
                    self.insert(InstructionData::binary(BinaryOp::Pow, magnitude, exponent));
                if numerator.is_odd() {
                    return self.insert(InstructionData::binary(
                        BinaryOp::Copysign,
                        power,
                        optimized_base,
                    ));
                }
                return power;
            }
        }

        // pow(2, x) => exp2(x)
        if self.literal_rounds_to(base, 2.0) {
            return self.emit_unary(UnaryOp::Exp2, exponent);
        }
        // pow(E, x) => exp(x)
        if self.is_constant(base, ConstantOp::E) {
            return self.emit_unary(UnaryOp::Exp, exponent);
        }

        self.emit_binary(BinaryOp::Pow, base, exponent)
    }

    fn optimize_sub(&mut self, lhs: usize, rhs: usize) -> usize {
        // exp(x) - 1 => expm1(x)
        if let Some(value) = self.unary_arg(lhs, UnaryOp::Exp)
            && self.is_exact_literal(rhs, 1.0)
        {
            return self.emit_unary(UnaryOp::Expm1, value);
        }
        // 1 - exp(x) => -expm1(x)
        if self.is_exact_literal(lhs, 1.0)
            && let Some(value) = self.unary_arg(rhs, UnaryOp::Exp)
        {
            let value = self.optimize(value);
            let expm1 = self.insert(InstructionData::unary(UnaryOp::Expm1, value));
            return self.insert(InstructionData::unary(UnaryOp::Neg, expm1));
        }

        self.emit_binary(BinaryOp::Sub, lhs, rhs)
    }

    fn source_data(&self, register: usize) -> InstructionData {
        self.instruction(register)
            .expect("invalid expression register")
            .clone()
    }

    fn instruction(&self, register: usize) -> Option<&InstructionData> {
        let index = register.checked_sub(self.source.arguments.len())?;
        self.source
            .instructions
            .get_index(index)
            .map(|(data, _)| data)
    }

    fn unary_arg(&self, register: usize, op: UnaryOp) -> Option<usize> {
        match self.instruction(register)? {
            InstructionData::Unary { op: actual, arg } if *actual == op => Some(*arg),
            _ => None,
        }
    }

    fn binary_args(&self, register: usize, op: BinaryOp) -> Option<(usize, usize)> {
        match self.instruction(register)? {
            InstructionData::Binary {
                op: actual,
                lhs,
                rhs,
            } if *actual == op => Some((*lhs, *rhs)),
            _ => None,
        }
    }

    // TODO: Consider pow(x, 2) in addition to x * x
    fn square_base(&self, register: usize) -> Option<usize> {
        let (lhs, rhs) = self.binary_args(register, BinaryOp::Mul)?;
        (lhs == rhs).then_some(lhs)
    }

    fn is_exact_literal(&self, register: usize, expected: f64) -> bool {
        matches!(
            self.instruction(register),
            Some(InstructionData::Literal { value }) if value.0 == expected
        )
    }

    fn literal_rounds_to(&self, register: usize, expected: f64) -> bool {
        matches!(
            self.instruction(register),
            Some(InstructionData::Literal { value }) if value.0.to_f64() == expected
        )
    }

    fn is_constant(&self, register: usize, expected: ConstantOp) -> bool {
        matches!(
            self.instruction(register),
            Some(InstructionData::Constant { op }) if *op == expected
        )
    }

    fn is_unary(&self, register: usize, expected: UnaryOp) -> bool {
        matches!(
            self.instruction(register),
            Some(InstructionData::Unary { op, .. }) if *op == expected
        )
    }

    fn rational_value(&self, register: usize) -> Option<Rational> {
        match self.instruction(register)? {
            InstructionData::Rational { val } => Some(val.0.clone()),
            _ => None,
        }
    }

    /// Match an angle in units of a full turn divided by the returned `unit`.
    fn trig_unit(&self, register: usize) -> Option<(u64, usize)> {
        let (lhs, rhs) = self.binary_args(register, BinaryOp::Mul)?;

        // PI * (x / n) and PI * x
        if self.is_constant(lhs, ConstantOp::Pi) {
            if let Some((value, divisor)) = self.binary_args(rhs, BinaryOp::Div) {
                return self
                    .positive_integer(divisor)
                    .map(|divisor| (2 * divisor, value));
            }
            return Some((2, rhs));
        }

        // (x / n) * PI and x * PI
        if self.is_constant(rhs, ConstantOp::Pi) {
            if let Some((value, divisor)) = self.binary_args(lhs, BinaryOp::Div) {
                return self
                    .positive_integer(divisor)
                    .map(|divisor| (2 * divisor, value));
            }
            return Some((2, lhs));
        }

        // (2 * PI) * x and x * (2 * PI)
        if self.is_two_pi(lhs) {
            return Some((1, rhs));
        }
        if self.is_two_pi(rhs) {
            return Some((1, lhs));
        }

        None
    }

    fn positive_integer(&self, register: usize) -> Option<u64> {
        let InstructionData::Literal { value } = self.instruction(register)? else {
            return None;
        };
        let float = value.0.to_f64();
        let integer = float as u64;
        (integer > 0 && integer as f64 == float).then_some(integer)
    }

    fn is_two_pi(&self, register: usize) -> bool {
        self.binary_args(register, BinaryOp::Mul)
            .is_some_and(|(lhs, rhs)| {
                self.literal_rounds_to(lhs, 2.0) && self.is_constant(rhs, ConstantOp::Pi)
            })
    }
}
