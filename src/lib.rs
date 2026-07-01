//! # Rival 3: Real Computation via Interval Arithmetic
//!
//! Rival is an advanced interval arithmetic library for
//! arbitrary-precision computation of complex mathematical expressions.
//! Its interval arithmetic is valid and attempts to be tight.
//! Besides the standard intervals, Rival also supports boolean intervals,
//! error intervals, and movability flags, as described in
//! ["An Interval Arithmetic for Robust Error Estimation"](https://arxiv.org/abs/2107.05784).
//!
//! Rival is a part of the [Herbie project](https://herbie.uwplse.org),
//! and is developed [on Github](https://github.com/herbie-fp/rival3).
//!
//! # Real Computation
//!
//! Rival is built to evaluate real-number expressions to
//! correctly-rounded floating-point outputs. To do so, you first compile
//! your real-number expression to a [`Machine`] via [`MachineBuilder`],
//! and then apply that machine to specific inputs.
//!
//! ```no_run
//! use rival::{Expr, MachineBuilder, Ival};
//! use rug::Float;
//!
//! struct Fp64Disc;
//! impl rival::Discretization for Fp64Disc {
//!     fn target(&self) -> u32 { 53 }
//!     fn convert(&self, _: usize, v: &Float) -> Float { Float::with_val(53, v) }
//!     fn distance(&self, _: usize, lo: &Float, hi: &Float) -> usize { 0 }
//! }
//! impl Clone for Fp64Disc { fn clone(&self) -> Self { Fp64Disc } }
//!
//! let expr = Expr::Sub(
//!     Box::new(Expr::Sin(Box::new(Expr::Var("x".into())))),
//!     Box::new(Expr::Sub(
//!         Box::new(Expr::Var("x".into())),
//!         Box::new(Expr::Div(
//!             Box::new(Expr::Pow(
//!                 Box::new(Expr::Var("x".into())),
//!                 Box::new(Expr::Literal(Float::with_val(53, 3))),
//!             )),
//!             Box::new(Expr::Literal(Float::with_val(53, 6))),
//!         )),
//!     )),
//! );
//!
//! let machine = MachineBuilder::new(Fp64Disc).build(vec![expr], vec!["x".into()]);
//! ```
//!
//! Rival works by evaluating the expression with high-precision interval
//! arithmetic, repeating the evaluation with ever-higher precision until
//! a narrow-enough output interval is found.
//!
//! Detailed profiling information can be accessed via [`Execution`].
//!
//! Rival also exposes the underlying interval-arithmetic library:
//!
//! ```
//! use rival::Ival;
//!
//! let x = Ival::from_lo_hi(
//!     rug::Float::with_val(20, 2),
//!     rug::Float::with_val(20, 3),
//! );
//! let mut result = Ival::zero(20);
//! result.sqrt_assign(&x);
//! ```
//!
//! Rival is fast, accurate, and sound. We believe it to be a
//! state-of-the-art implementation, competitive with Sollya/MPFI,
//! Calcium/Arb, and Mathematica.

mod eval;
mod interval;
mod mpfr;

pub use eval::ast::Expr;
pub use eval::machine::{Discretization, Hint, Machine, MachineBuilder};
pub use eval::optimal::OptimalPrecisionResult;
pub use eval::profile::Execution;
pub use eval::run::RivalError;
pub use interval::{ErrorFlags, Ival};
