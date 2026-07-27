use rival::{Expression, ExpressionBuilder};
use rug::{Float, Integer, Rational};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

pub const RIVAL_EXPR_INVALID: u32 = u32::MAX;

pub struct RivalExprBuilder {
    pub(crate) expressions: ExpressionBuilder,
    handles: Vec<Expression>,
    handle_of: HashMap<Expression, u32>,
}

impl RivalExprBuilder {
    fn new(arguments: Vec<String>) -> Self {
        Self {
            expressions: ExpressionBuilder::new(arguments),
            handles: Vec::new(),
            handle_of: HashMap::new(),
        }
    }

    fn expression(&self, handle: u32) -> Option<Expression> {
        self.handles.get(handle as usize).copied()
    }

    fn intern(&mut self, expression: Expression) -> u32 {
        if let Some(&handle) = self.handle_of.get(&expression) {
            return handle;
        }
        let handle = self.handles.len();
        if handle >= RIVAL_EXPR_INVALID as usize {
            return RIVAL_EXPR_INVALID;
        }
        let handle = handle as u32;
        self.handles.push(expression);
        self.handle_of.insert(expression, handle);
        handle
    }

    pub(crate) fn outputs(&self, handles: &[u32]) -> Option<Vec<Expression>> {
        handles
            .iter()
            .map(|&handle| self.expression(handle))
            .collect()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_builder_new(
    vars: *const *const c_char,
    n_vars: usize,
) -> *mut RivalExprBuilder {
    if vars.is_null() && n_vars > 0 {
        return std::ptr::null_mut();
    }

    let arguments = if n_vars == 0 {
        Vec::new()
    } else {
        let vars = unsafe { slice::from_raw_parts(vars, n_vars) };
        let Some(arguments) = vars
            .iter()
            .map(|&var| {
                if var.is_null() {
                    return None;
                }
                unsafe { CStr::from_ptr(var) }
                    .to_str()
                    .ok()
                    .map(str::to_owned)
            })
            .collect()
        else {
            return std::ptr::null_mut();
        };
        arguments
    };

    Box::into_raw(Box::new(RivalExprBuilder::new(arguments)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_builder_free(builder: *mut RivalExprBuilder) {
    if !builder.is_null() {
        unsafe { drop(Box::from_raw(builder)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_var(
    builder: *mut RivalExprBuilder,
    name: *const c_char,
) -> u32 {
    if builder.is_null() || name.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let builder = unsafe { &mut *builder };
    let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else {
        return RIVAL_EXPR_INVALID;
    };
    let Some(expression) = builder.expressions.variable(name) else {
        return RIVAL_EXPR_INVALID;
    };
    builder.intern(expression)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_f64(builder: *mut RivalExprBuilder, value: f64) -> u32 {
    unsafe {
        expression_leaf(builder, |expressions| {
            expressions.literal(Float::with_val(53, value))
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_rational(
    builder: *mut RivalExprBuilder,
    num: i64,
    den: i64,
) -> u32 {
    if den == 0 {
        return RIVAL_EXPR_INVALID;
    }
    unsafe {
        expression_leaf(builder, |expressions| {
            expressions.rational(Rational::from((num, den)))
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_bigint(
    builder: *mut RivalExprBuilder,
    value: *const c_char,
) -> u32 {
    if builder.is_null() || value.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let Ok(value) = unsafe { CStr::from_ptr(value) }.to_str() else {
        return RIVAL_EXPR_INVALID;
    };
    let Ok(value) = Integer::parse(value).map(Integer::from) else {
        return RIVAL_EXPR_INVALID;
    };
    let precision = value.significant_bits().max(53);
    unsafe {
        expression_leaf(builder, |expressions| {
            expressions.literal(Float::with_val(precision, value))
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_bigrational(
    builder: *mut RivalExprBuilder,
    numerator: *const c_char,
    denominator: *const c_char,
) -> u32 {
    if builder.is_null() || numerator.is_null() || denominator.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let Ok(numerator) = unsafe { CStr::from_ptr(numerator) }.to_str() else {
        return RIVAL_EXPR_INVALID;
    };
    let Ok(denominator) = unsafe { CStr::from_ptr(denominator) }.to_str() else {
        return RIVAL_EXPR_INVALID;
    };
    let Ok(numerator) = Integer::parse(numerator).map(Integer::from) else {
        return RIVAL_EXPR_INVALID;
    };
    let Ok(denominator) = Integer::parse(denominator).map(Integer::from) else {
        return RIVAL_EXPR_INVALID;
    };
    if denominator.cmp0().is_eq() {
        return RIVAL_EXPR_INVALID;
    }

    unsafe {
        expression_leaf(builder, |expressions| {
            expressions.rational(Rational::from((numerator, denominator)))
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_pi(builder: *mut RivalExprBuilder) -> u32 {
    unsafe { expression_leaf(builder, ExpressionBuilder::pi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_e(builder: *mut RivalExprBuilder) -> u32 {
    unsafe { expression_leaf(builder, ExpressionBuilder::e) }
}

unsafe fn expression_leaf(
    builder: *mut RivalExprBuilder,
    build: impl FnOnce(&mut ExpressionBuilder) -> Expression,
) -> u32 {
    if builder.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let builder = unsafe { &mut *builder };
    let expression = build(&mut builder.expressions);
    builder.intern(expression)
}

unsafe fn expression_unary(
    builder: *mut RivalExprBuilder,
    arg: u32,
    build: impl FnOnce(&mut ExpressionBuilder, Expression) -> Expression,
) -> u32 {
    if builder.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let builder = unsafe { &mut *builder };
    let Some(arg) = builder.expression(arg) else {
        return RIVAL_EXPR_INVALID;
    };
    let expression = build(&mut builder.expressions, arg);
    builder.intern(expression)
}

unsafe fn expression_unary_param(
    builder: *mut RivalExprBuilder,
    param: u64,
    arg: u32,
    build: impl FnOnce(&mut ExpressionBuilder, u64, Expression) -> Expression,
) -> u32 {
    if builder.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let builder = unsafe { &mut *builder };
    let Some(arg) = builder.expression(arg) else {
        return RIVAL_EXPR_INVALID;
    };
    let expression = build(&mut builder.expressions, param, arg);
    builder.intern(expression)
}

unsafe fn expression_binary(
    builder: *mut RivalExprBuilder,
    lhs: u32,
    rhs: u32,
    build: impl FnOnce(&mut ExpressionBuilder, Expression, Expression) -> Expression,
) -> u32 {
    if builder.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let builder = unsafe { &mut *builder };
    let (Some(lhs), Some(rhs)) = (builder.expression(lhs), builder.expression(rhs)) else {
        return RIVAL_EXPR_INVALID;
    };
    let expression = build(&mut builder.expressions, lhs, rhs);
    builder.intern(expression)
}

unsafe fn expression_ternary(
    builder: *mut RivalExprBuilder,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    build: impl FnOnce(&mut ExpressionBuilder, Expression, Expression, Expression) -> Expression,
) -> u32 {
    if builder.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let builder = unsafe { &mut *builder };
    let (Some(arg1), Some(arg2), Some(arg3)) = (
        builder.expression(arg1),
        builder.expression(arg2),
        builder.expression(arg3),
    ) else {
        return RIVAL_EXPR_INVALID;
    };
    let expression = build(&mut builder.expressions, arg1, arg2, arg3);
    builder.intern(expression)
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum RivalUnaryOp {
    Neg,
    Fabs,
    Sqrt,
    Cbrt,
    Pow2,
    Exp,
    Exp2,
    Expm1,
    Log,
    Log2,
    Log10,
    Log1p,
    Logb,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Erf,
    Erfc,
    Lgamma,
    Tgamma,
    Rint,
    Round,
    Ceil,
    Floor,
    Trunc,
    Not,
    Assert,
    Error,
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum RivalUnaryParamOp {
    Cosu,
    Sinu,
    Tanu,
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum RivalBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Hypot,
    Fmin,
    Fmax,
    Fdim,
    Copysign,
    Fmod,
    Remainder,
    Atan2,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum RivalTernaryOp {
    Fma,
    If,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_unary(
    builder: *mut RivalExprBuilder,
    op: RivalUnaryOp,
    arg: u32,
) -> u32 {
    unsafe {
        expression_unary(builder, arg, |expressions, arg| match op {
            RivalUnaryOp::Neg => expressions.neg(arg),
            RivalUnaryOp::Fabs => expressions.fabs(arg),
            RivalUnaryOp::Sqrt => expressions.sqrt(arg),
            RivalUnaryOp::Cbrt => expressions.cbrt(arg),
            RivalUnaryOp::Pow2 => expressions.pow2(arg),
            RivalUnaryOp::Exp => expressions.exp(arg),
            RivalUnaryOp::Exp2 => expressions.exp2(arg),
            RivalUnaryOp::Expm1 => expressions.expm1(arg),
            RivalUnaryOp::Log => expressions.log(arg),
            RivalUnaryOp::Log2 => expressions.log2(arg),
            RivalUnaryOp::Log10 => expressions.log10(arg),
            RivalUnaryOp::Log1p => expressions.log1p(arg),
            RivalUnaryOp::Logb => expressions.logb(arg),
            RivalUnaryOp::Sin => expressions.sin(arg),
            RivalUnaryOp::Cos => expressions.cos(arg),
            RivalUnaryOp::Tan => expressions.tan(arg),
            RivalUnaryOp::Asin => expressions.asin(arg),
            RivalUnaryOp::Acos => expressions.acos(arg),
            RivalUnaryOp::Atan => expressions.atan(arg),
            RivalUnaryOp::Sinh => expressions.sinh(arg),
            RivalUnaryOp::Cosh => expressions.cosh(arg),
            RivalUnaryOp::Tanh => expressions.tanh(arg),
            RivalUnaryOp::Asinh => expressions.asinh(arg),
            RivalUnaryOp::Acosh => expressions.acosh(arg),
            RivalUnaryOp::Atanh => expressions.atanh(arg),
            RivalUnaryOp::Erf => expressions.erf(arg),
            RivalUnaryOp::Erfc => expressions.erfc(arg),
            RivalUnaryOp::Lgamma => expressions.lgamma(arg),
            RivalUnaryOp::Tgamma => expressions.tgamma(arg),
            RivalUnaryOp::Rint => expressions.rint(arg),
            RivalUnaryOp::Round => expressions.round(arg),
            RivalUnaryOp::Ceil => expressions.ceil(arg),
            RivalUnaryOp::Floor => expressions.floor(arg),
            RivalUnaryOp::Trunc => expressions.trunc(arg),
            RivalUnaryOp::Not => expressions.not(arg),
            RivalUnaryOp::Assert => expressions.assert(arg),
            RivalUnaryOp::Error => expressions.error(arg),
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_unary_param(
    builder: *mut RivalExprBuilder,
    op: RivalUnaryParamOp,
    param: u64,
    arg: u32,
) -> u32 {
    unsafe {
        expression_unary_param(builder, param, arg, |expressions, param, arg| match op {
            RivalUnaryParamOp::Cosu => expressions.cosu(param, arg),
            RivalUnaryParamOp::Sinu => expressions.sinu(param, arg),
            RivalUnaryParamOp::Tanu => expressions.tanu(param, arg),
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_binary(
    builder: *mut RivalExprBuilder,
    op: RivalBinaryOp,
    lhs: u32,
    rhs: u32,
) -> u32 {
    unsafe {
        expression_binary(builder, lhs, rhs, |expressions, lhs, rhs| match op {
            RivalBinaryOp::Add => expressions.add(lhs, rhs),
            RivalBinaryOp::Sub => expressions.sub(lhs, rhs),
            RivalBinaryOp::Mul => expressions.mul(lhs, rhs),
            RivalBinaryOp::Div => expressions.div(lhs, rhs),
            RivalBinaryOp::Pow => expressions.pow(lhs, rhs),
            RivalBinaryOp::Hypot => expressions.hypot(lhs, rhs),
            RivalBinaryOp::Fmin => expressions.fmin(lhs, rhs),
            RivalBinaryOp::Fmax => expressions.fmax(lhs, rhs),
            RivalBinaryOp::Fdim => expressions.fdim(lhs, rhs),
            RivalBinaryOp::Copysign => expressions.copysign(lhs, rhs),
            RivalBinaryOp::Fmod => expressions.fmod(lhs, rhs),
            RivalBinaryOp::Remainder => expressions.remainder(lhs, rhs),
            RivalBinaryOp::Atan2 => expressions.atan2(lhs, rhs),
            RivalBinaryOp::And => expressions.and(lhs, rhs),
            RivalBinaryOp::Or => expressions.or(lhs, rhs),
            RivalBinaryOp::Eq => expressions.eq(lhs, rhs),
            RivalBinaryOp::Ne => expressions.ne(lhs, rhs),
            RivalBinaryOp::Lt => expressions.lt(lhs, rhs),
            RivalBinaryOp::Le => expressions.le(lhs, rhs),
            RivalBinaryOp::Gt => expressions.gt(lhs, rhs),
            RivalBinaryOp::Ge => expressions.ge(lhs, rhs),
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_ternary(
    builder: *mut RivalExprBuilder,
    op: RivalTernaryOp,
    arg1: u32,
    arg2: u32,
    arg3: u32,
) -> u32 {
    unsafe {
        expression_ternary(
            builder,
            arg1,
            arg2,
            arg3,
            |expressions, arg1, arg2, arg3| match op {
                RivalTernaryOp::Fma => expressions.fma(arg1, arg2, arg3),
                RivalTernaryOp::If => expressions.if_else(arg1, arg2, arg3),
            },
        )
    }
}
