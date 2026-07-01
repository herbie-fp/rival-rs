//! Interval expression evaluation pipeline.

pub(crate) mod machine;
pub(crate) mod optimal;
pub(crate) mod profile;
pub(crate) mod run;

pub(crate) mod ast {
    pub use super::ops::Expr;
}

pub(crate) mod adjust;
pub(crate) mod execute;
pub(crate) mod instructions;
pub(crate) mod macros;
pub(crate) mod ops;
pub(crate) mod tricks;
