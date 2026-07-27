//! Incremental construction of expression graphs.

mod lower;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use indexmap::IndexMap;
use rug::{Float, Rational};

use super::instructions::{
    BinaryOp, ConstantOp, InstructionData, TernaryOp, UnaryOp, UnaryParamOp,
};

pub(crate) use lower::Program;

/// A reference to an expression in an [`ExpressionBuilder`].
///
/// Expression references are cheap to copy and may be reused as inputs to any
/// number of later expressions built by the same builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Expression {
    builder: u64,
    register: usize,
}

/// Incrementally builds a shared expression graph.
///
/// Rival supports a simple language of real-number expressions containing
/// variables, rational literals, common mathematical functions, and common
/// mathematical constants. Each method on the builder adds one operation and
/// returns an [`Expression`] that later operations can use as an input.
///
/// Expressions largely follow the semantics of `math.h`, not Racket, when it
/// comes to, for example, the order of arguments to `atan2` or the naming of
/// the exponential function.
///
/// Some inputs are invalid to some operations, such as division by zero,
/// square roots of negative numbers, and similar. For `pow`, Rival considers
/// `pow(0, x)` valid for non-negative `x`, and `pow(x, y)` invalid for
/// negative `x` and non-integer `y`. In general these conventions again follow
/// those in `math.h`. Colloquially we say that these expressions "throw" on
/// invalid points, though note that internally Rival uses error intervals to
/// soundly track whether an input is invalid or not.
///
/// Expressions that mix boolean and real-number operations must type-check in
/// the expected way, and variables must have consistent types. Rival does not
/// perform typechecking; that is a user responsibility, and Rival may return
/// undefined results if passed ill-typed formulas.
///
/// The `assert` and `error` operations need additional explanation; these
/// control the definition of a "valid" input to an expression. The `assert`
/// operation takes in a boolean input and returns a boolean output. If the
/// input is false, `assert` throws. Its output is always true. `error` has the
/// opposite behavior. This operation never throws, and instead returns true if
/// its argument throws and false if it doesn't. `assert` and `error` can be
/// used to model constructs like preconditions, tests, try/catch blocks, and
/// others.
pub struct ExpressionBuilder {
    id: u64,
    arguments: Vec<String>,
    variables: HashMap<String, Expression>,
    instructions: IndexMap<InstructionData, usize>,
}

static NEXT_BUILDER_ID: AtomicU64 = AtomicU64::new(1);

impl ExpressionBuilder {
    /// Create an expression builder with the given input arguments.
    pub fn new<I, S>(arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let id = NEXT_BUILDER_ID.fetch_add(1, Ordering::Relaxed);
        let arguments: Vec<String> = arguments.into_iter().map(Into::into).collect();
        let variables = arguments
            .iter()
            .enumerate()
            .map(|(register, name)| {
                (
                    name.clone(),
                    Expression {
                        builder: id,
                        register,
                    },
                )
            })
            .collect();

        Self {
            id,
            arguments,
            variables,
            instructions: IndexMap::new(),
        }
    }

    /// Look up an input argument by name.
    pub fn variable(&self, name: &str) -> Option<Expression> {
        self.variables.get(name).copied()
    }

    /// Add a floating-point literal.
    pub fn literal(&mut self, value: Float) -> Expression {
        self.insert(InstructionData::literal(value))
    }

    /// Add an exact rational literal.
    pub fn rational(&mut self, value: Rational) -> Expression {
        self.insert(InstructionData::rational(value))
    }

    /// Add pi.
    pub fn pi(&mut self) -> Expression {
        self.insert(InstructionData::constant(ConstantOp::Pi))
    }

    /// Add e.
    pub fn e(&mut self) -> Expression {
        self.insert(InstructionData::constant(ConstantOp::E))
    }

    /// Return the number of input arguments.
    pub fn argument_count(&self) -> usize {
        self.arguments.len()
    }

    pub fn fma(&mut self, lhs: Expression, rhs: Expression, addend: Expression) -> Expression {
        self.ternary_raw(TernaryOp::Fma, lhs, rhs, addend)
    }

    pub fn if_else(
        &mut self,
        condition: Expression,
        if_true: Expression,
        if_false: Expression,
    ) -> Expression {
        self.ternary_raw(TernaryOp::If, condition, if_true, if_false)
    }

    /// Rewrite and lower the expressions reachable from `outputs`.
    pub(crate) fn finish(&self, outputs: &[Expression]) -> Program {
        assert!(
            outputs.iter().all(|output| self.contains(*output)),
            "output expression does not belong to this builder"
        );
        lower::lower(self, outputs)
    }

    fn contains(&self, expression: Expression) -> bool {
        expression.builder == self.id
            && expression.register < self.arguments.len() + self.instructions.len()
    }

    fn insert(&mut self, data: InstructionData) -> Expression {
        let next_register = self.arguments.len() + self.instructions.len();
        let register = *self.instructions.entry(data).or_insert(next_register);
        Expression {
            builder: self.id,
            register,
        }
    }

    fn unary_raw(&mut self, op: UnaryOp, arg: Expression) -> Expression {
        self.require(arg);
        self.insert(InstructionData::unary(op, arg.register))
    }

    fn unary_param_raw(&mut self, op: UnaryParamOp, param: u64, arg: Expression) -> Expression {
        self.require(arg);
        self.insert(InstructionData::unary_param(op, param, arg.register))
    }

    fn binary_raw(&mut self, op: BinaryOp, lhs: Expression, rhs: Expression) -> Expression {
        self.require(lhs);
        self.require(rhs);
        self.insert(InstructionData::binary(op, lhs.register, rhs.register))
    }

    fn ternary_raw(
        &mut self,
        op: TernaryOp,
        arg1: Expression,
        arg2: Expression,
        arg3: Expression,
    ) -> Expression {
        self.require(arg1);
        self.require(arg2);
        self.require(arg3);
        self.insert(InstructionData::ternary(
            op,
            arg1.register,
            arg2.register,
            arg3.register,
        ))
    }

    fn require(&self, expression: Expression) {
        assert!(
            self.contains(expression),
            "expression does not belong to this builder"
        );
    }
}

macro_rules! unary_methods {
    ($(($method:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $method(&mut self, arg: Expression) -> Expression {
                self.unary_raw(UnaryOp::$op, arg)
            }
        )*
    };
}

macro_rules! unary_param_methods {
    ($(($method:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $method(&mut self, param: u64, arg: Expression) -> Expression {
                self.unary_param_raw(UnaryParamOp::$op, param, arg)
            }
        )*
    };
}

macro_rules! binary_methods {
    ($(($method:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $method(&mut self, lhs: Expression, rhs: Expression) -> Expression {
                self.binary_raw(BinaryOp::$op, lhs, rhs)
            }
        )*
    };
}

impl ExpressionBuilder {
    unary_methods!(
        (pow2, Pow2),
        (fabs, Fabs),
        (neg, Neg),
        (sqrt, Sqrt),
        (cbrt, Cbrt),
        (exp, Exp),
        (exp2, Exp2),
        (expm1, Expm1),
        (log, Log),
        (log2, Log2),
        (log10, Log10),
        (log1p, Log1p),
        (logb, Logb),
        (sin, Sin),
        (cos, Cos),
        (tan, Tan),
        (asin, Asin),
        (acos, Acos),
        (atan, Atan),
        (sinh, Sinh),
        (cosh, Cosh),
        (tanh, Tanh),
        (asinh, Asinh),
        (acosh, Acosh),
        (atanh, Atanh),
        (erf, Erf),
        (erfc, Erfc),
        (lgamma, Lgamma),
        (tgamma, Tgamma),
        (rint, Rint),
        (round, Round),
        (ceil, Ceil),
        (floor, Floor),
        (trunc, Trunc),
        (not, Not),
        (error, Error),
        (assert, Assert),
    );

    unary_param_methods!((cosu, Cosu), (sinu, Sinu), (tanu, Tanu));

    binary_methods!(
        (fdim, Fdim),
        (hypot, Hypot),
        (add, Add),
        (sub, Sub),
        (mul, Mul),
        (div, Div),
        (pow, Pow),
        (and, And),
        (or, Or),
        (eq, Eq),
        (ne, Ne),
        (lt, Lt),
        (le, Le),
        (gt, Gt),
        (ge, Ge),
        (fmin, Fmin),
        (fmax, Fmax),
        (copysign, Copysign),
        (atan2, Atan2),
        (fmod, Fmod),
        (remainder, Remainder),
    );
}
