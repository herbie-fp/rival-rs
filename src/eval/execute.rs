//! Evaluator for individual instructions

use crate::eval::instructions::{Instruction, InstructionData::*};
use crate::eval::ops;
use crate::interval::{ErrorFlags, Ival};
use rug::float::Round;
use rug::ops::AssignRound;

/// Evaluate an instruction into its output register using the provided precision
pub fn evaluate_instruction(instruction: &Instruction, registers: &mut [Ival], precision: u32) {
    let out = instruction.out;

    // Split registers so we can mutate out_reg while reading inputs.
    let (before, rest) = registers.split_at_mut(out);
    let (out_reg, after) = rest.split_first_mut().expect("Invalid register index");
    out_reg.set_prec_raw(precision);

    // Helper to access any register despite the split.
    let get_reg = |idx: usize| -> &Ival {
        if idx < out {
            &before[idx]
        } else {
            &after[idx - out - 1]
        }
    };

    match &instruction.data {
        Literal { value } => {
            let val = &value.0;
            out_reg.set_prec_raw(precision.max(val.prec()));
            out_reg.lo.as_float_mut().assign_round(val, Round::Down);
            out_reg.hi.as_float_mut().assign_round(val, Round::Up);
            out_reg.err = if val.is_finite() {
                ErrorFlags::none()
            } else {
                ErrorFlags::error()
            };
            let exact = val.is_nan() || out_reg.lo.as_float() == out_reg.hi.as_float();
            out_reg.lo.immovable = exact;
            out_reg.hi.immovable = exact;
        }
        Rational { val } => {
            let rat = &val.0;
            out_reg.lo.as_float_mut().assign_round(rat, Round::Down);
            out_reg.hi.as_float_mut().assign_round(rat, Round::Up);
            out_reg.err = ErrorFlags::none();
            let exact = out_reg.lo.as_float() == out_reg.hi.as_float();
            out_reg.lo.immovable = exact;
            out_reg.hi.immovable = exact;
        }
        Constant { op } => ops::execute_constant(*op, out_reg),
        Unary { op, arg } => ops::execute_unary(*op, out_reg, get_reg(*arg)),
        UnaryParam { op, param, arg } => {
            ops::execute_unary_param(*op, *param, out_reg, get_reg(*arg))
        }
        Binary { op, lhs, rhs } => ops::execute_binary(*op, out_reg, get_reg(*lhs), get_reg(*rhs)),
        Ternary {
            op,
            arg1,
            arg2,
            arg3,
        } => ops::execute_ternary(*op, out_reg, get_reg(*arg1), get_reg(*arg2), get_reg(*arg3)),
    }
}
