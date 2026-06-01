use rival::Expr;
use rug::{Float, Integer, Rational};
use std::ffi::CStr;
use std::os::raw::c_char;

#[derive(Clone)]
pub(crate) enum ArenaNode {
    Var(String),
    Literal(Float),
    Rational(Rational),
    BigInt(Integer),
    Pi,
    E,
    Neg(u32),
    Fabs(u32),
    Sqrt(u32),
    Cbrt(u32),
    Pow2(u32),
    Exp(u32),
    Exp2(u32),
    Expm1(u32),
    Log(u32),
    Log2(u32),
    Log10(u32),
    Log1p(u32),
    Logb(u32),
    Sin(u32),
    Cos(u32),
    Tan(u32),
    Asin(u32),
    Acos(u32),
    Atan(u32),
    Sinh(u32),
    Cosh(u32),
    Tanh(u32),
    Asinh(u32),
    Acosh(u32),
    Atanh(u32),
    Erf(u32),
    Erfc(u32),
    Lgamma(u32),
    Tgamma(u32),
    Rint(u32),
    Round(u32),
    Ceil(u32),
    Floor(u32),
    Trunc(u32),
    Not(u32),
    Assert(u32),
    Error(u32),
    Sinu(u64, u32),
    Cosu(u64, u32),
    Tanu(u64, u32),
    Add(u32, u32),
    Sub(u32, u32),
    Mul(u32, u32),
    Div(u32, u32),
    Pow(u32, u32),
    Hypot(u32, u32),
    Fmin(u32, u32),
    Fmax(u32, u32),
    Fdim(u32, u32),
    Copysign(u32, u32),
    Fmod(u32, u32),
    Remainder(u32, u32),
    Atan2(u32, u32),
    And(u32, u32),
    Or(u32, u32),
    Eq(u32, u32),
    Ne(u32, u32),
    Lt(u32, u32),
    Le(u32, u32),
    Gt(u32, u32),
    Ge(u32, u32),
    Fma(u32, u32, u32),
    If(u32, u32, u32),
}

pub struct RivalExprArena {
    pub(crate) nodes: Vec<ArenaNode>,
}

pub const RIVAL_EXPR_INVALID: u32 = u32::MAX;

impl RivalExprArena {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(256),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, node: ArenaNode) -> u32 {
        let idx = self.nodes.len();
        if idx >= RIVAL_EXPR_INVALID as usize {
            return RIVAL_EXPR_INVALID;
        }
        self.nodes.push(node);
        idx as u32
    }

    #[inline]
    pub(crate) fn is_valid(&self, handle: u32) -> bool {
        handle != RIVAL_EXPR_INVALID && (handle as usize) < self.nodes.len()
    }

    pub(crate) fn materialize(&self, handle: u32) -> Option<Expr> {
        if !self.is_valid(handle) {
            return None;
        }

        let node = &self.nodes[handle as usize];
        Some(match node {
            ArenaNode::Var(s) => Expr::Var(s.clone()),
            ArenaNode::Literal(f) => Expr::Literal(f.clone()),
            ArenaNode::Rational(r) => Expr::Rational(r.clone()),
            ArenaNode::BigInt(i) => {
                // Use enough precision to exactly represent the integer.
                let bits = i.significant_bits().max(53);
                Expr::Literal(Float::with_val(bits, i))
            }
            ArenaNode::Pi => Expr::Pi,
            ArenaNode::E => Expr::E,
            ArenaNode::Neg(x) => Expr::Neg(Box::new(self.materialize(*x)?)),
            ArenaNode::Fabs(x) => Expr::Fabs(Box::new(self.materialize(*x)?)),
            ArenaNode::Sqrt(x) => Expr::Sqrt(Box::new(self.materialize(*x)?)),
            ArenaNode::Cbrt(x) => Expr::Cbrt(Box::new(self.materialize(*x)?)),
            ArenaNode::Pow2(x) => Expr::Pow2(Box::new(self.materialize(*x)?)),
            ArenaNode::Exp(x) => Expr::Exp(Box::new(self.materialize(*x)?)),
            ArenaNode::Exp2(x) => Expr::Exp2(Box::new(self.materialize(*x)?)),
            ArenaNode::Expm1(x) => Expr::Expm1(Box::new(self.materialize(*x)?)),
            ArenaNode::Log(x) => Expr::Log(Box::new(self.materialize(*x)?)),
            ArenaNode::Log2(x) => Expr::Log2(Box::new(self.materialize(*x)?)),
            ArenaNode::Log10(x) => Expr::Log10(Box::new(self.materialize(*x)?)),
            ArenaNode::Log1p(x) => Expr::Log1p(Box::new(self.materialize(*x)?)),
            ArenaNode::Logb(x) => Expr::Logb(Box::new(self.materialize(*x)?)),
            ArenaNode::Sin(x) => Expr::Sin(Box::new(self.materialize(*x)?)),
            ArenaNode::Cos(x) => Expr::Cos(Box::new(self.materialize(*x)?)),
            ArenaNode::Tan(x) => Expr::Tan(Box::new(self.materialize(*x)?)),
            ArenaNode::Asin(x) => Expr::Asin(Box::new(self.materialize(*x)?)),
            ArenaNode::Acos(x) => Expr::Acos(Box::new(self.materialize(*x)?)),
            ArenaNode::Atan(x) => Expr::Atan(Box::new(self.materialize(*x)?)),
            ArenaNode::Sinh(x) => Expr::Sinh(Box::new(self.materialize(*x)?)),
            ArenaNode::Cosh(x) => Expr::Cosh(Box::new(self.materialize(*x)?)),
            ArenaNode::Tanh(x) => Expr::Tanh(Box::new(self.materialize(*x)?)),
            ArenaNode::Asinh(x) => Expr::Asinh(Box::new(self.materialize(*x)?)),
            ArenaNode::Acosh(x) => Expr::Acosh(Box::new(self.materialize(*x)?)),
            ArenaNode::Atanh(x) => Expr::Atanh(Box::new(self.materialize(*x)?)),
            ArenaNode::Erf(x) => Expr::Erf(Box::new(self.materialize(*x)?)),
            ArenaNode::Erfc(x) => Expr::Erfc(Box::new(self.materialize(*x)?)),
            ArenaNode::Lgamma(x) => Expr::Lgamma(Box::new(self.materialize(*x)?)),
            ArenaNode::Tgamma(x) => Expr::Tgamma(Box::new(self.materialize(*x)?)),
            ArenaNode::Rint(x) => Expr::Rint(Box::new(self.materialize(*x)?)),
            ArenaNode::Round(x) => Expr::Round(Box::new(self.materialize(*x)?)),
            ArenaNode::Ceil(x) => Expr::Ceil(Box::new(self.materialize(*x)?)),
            ArenaNode::Floor(x) => Expr::Floor(Box::new(self.materialize(*x)?)),
            ArenaNode::Trunc(x) => Expr::Trunc(Box::new(self.materialize(*x)?)),
            ArenaNode::Not(x) => Expr::Not(Box::new(self.materialize(*x)?)),
            ArenaNode::Assert(x) => Expr::Assert(Box::new(self.materialize(*x)?)),
            ArenaNode::Error(x) => Expr::Error(Box::new(self.materialize(*x)?)),
            ArenaNode::Sinu(n, x) => Expr::Sinu(*n, Box::new(self.materialize(*x)?)),
            ArenaNode::Cosu(n, x) => Expr::Cosu(*n, Box::new(self.materialize(*x)?)),
            ArenaNode::Tanu(n, x) => Expr::Tanu(*n, Box::new(self.materialize(*x)?)),
            ArenaNode::Add(a, b) => Expr::Add(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Sub(a, b) => Expr::Sub(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Mul(a, b) => Expr::Mul(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Div(a, b) => Expr::Div(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Pow(a, b) => Expr::Pow(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Hypot(a, b) => Expr::Hypot(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Fmin(a, b) => Expr::Fmin(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Fmax(a, b) => Expr::Fmax(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Fdim(a, b) => Expr::Fdim(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Copysign(a, b) => Expr::Copysign(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Fmod(a, b) => Expr::Fmod(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Remainder(a, b) => Expr::Remainder(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Atan2(a, b) => Expr::Atan2(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::And(a, b) => Expr::And(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Or(a, b) => Expr::Or(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Eq(a, b) => Expr::Eq(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Ne(a, b) => Expr::Ne(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Lt(a, b) => Expr::Lt(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Le(a, b) => Expr::Le(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Gt(a, b) => Expr::Gt(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Ge(a, b) => Expr::Ge(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
            ),
            ArenaNode::Fma(a, b, c) => Expr::Fma(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
                Box::new(self.materialize(*c)?),
            ),
            ArenaNode::If(a, b, c) => Expr::If(
                Box::new(self.materialize(*a)?),
                Box::new(self.materialize(*b)?),
                Box::new(self.materialize(*c)?),
            ),
        })
    }

    pub fn materialize_all(&self, handles: &[u32]) -> Option<Vec<Expr>> {
        handles.iter().map(|&h| self.materialize(h)).collect()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rival_expr_arena_new() -> *mut RivalExprArena {
    Box::into_raw(Box::new(RivalExprArena::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn rival_expr_arena_with_capacity(capacity: usize) -> *mut RivalExprArena {
    Box::into_raw(Box::new(RivalExprArena::with_capacity(capacity)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_arena_free(arena: *mut RivalExprArena) {
    if !arena.is_null() {
        unsafe { drop(Box::from_raw(arena)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_arena_len(arena: *const RivalExprArena) -> usize {
    if arena.is_null() {
        0
    } else {
        unsafe { (*arena).len() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_arena_clear(arena: *mut RivalExprArena) {
    if !arena.is_null() {
        unsafe { (*arena).nodes.clear() };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_var(arena: *mut RivalExprArena, name: *const c_char) -> u32 {
    if arena.is_null() || name.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    unsafe {
        let name_str = match CStr::from_ptr(name).to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => return RIVAL_EXPR_INVALID,
        };
        (*arena).push(ArenaNode::Var(name_str))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_f64(arena: *mut RivalExprArena, value: f64) -> u32 {
    if arena.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    unsafe { (*arena).push(ArenaNode::Literal(Float::with_val(53, value))) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_rational(
    arena: *mut RivalExprArena,
    num: i64,
    den: i64,
) -> u32 {
    if arena.is_null() || den == 0 {
        return RIVAL_EXPR_INVALID;
    }
    unsafe { (*arena).push(ArenaNode::Rational(Rational::from((num, den)))) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_bigint(
    arena: *mut RivalExprArena,
    value_str: *const c_char,
) -> u32 {
    if arena.is_null() || value_str.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    unsafe {
        let s = match CStr::from_ptr(value_str).to_str() {
            Ok(s) => s,
            Err(_) => return RIVAL_EXPR_INVALID,
        };
        let int = match Integer::parse(s) {
            Ok(parsed) => Integer::from(parsed),
            Err(_) => return RIVAL_EXPR_INVALID,
        };
        (*arena).push(ArenaNode::BigInt(int))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_bigrational(
    arena: *mut RivalExprArena,
    num_str: *const c_char,
    den_str: *const c_char,
) -> u32 {
    if arena.is_null() || num_str.is_null() || den_str.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    unsafe {
        let num_s = match CStr::from_ptr(num_str).to_str() {
            Ok(s) => s,
            Err(_) => return RIVAL_EXPR_INVALID,
        };
        let den_s = match CStr::from_ptr(den_str).to_str() {
            Ok(s) => s,
            Err(_) => return RIVAL_EXPR_INVALID,
        };
        let num = match Integer::parse(num_s) {
            Ok(parsed) => Integer::from(parsed),
            Err(_) => return RIVAL_EXPR_INVALID,
        };
        let den = match Integer::parse(den_s) {
            Ok(parsed) => Integer::from(parsed),
            Err(_) => return RIVAL_EXPR_INVALID,
        };
        if den.cmp0() == std::cmp::Ordering::Equal {
            return RIVAL_EXPR_INVALID;
        }
        (*arena).push(ArenaNode::Rational(Rational::from((num, den))))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_pi(arena: *mut RivalExprArena) -> u32 {
    if arena.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    unsafe { (*arena).push(ArenaNode::Pi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_e(arena: *mut RivalExprArena) -> u32 {
    if arena.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    unsafe { (*arena).push(ArenaNode::E) }
}

#[inline]
unsafe fn arena_unary(arena: *mut RivalExprArena, x: u32, node: fn(u32) -> ArenaNode) -> u32 {
    if arena.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let arena = unsafe { &mut *arena };
    if !arena.is_valid(x) {
        return RIVAL_EXPR_INVALID;
    }
    arena.push(node(x))
}

#[inline]
unsafe fn arena_binary(
    arena: *mut RivalExprArena,
    x: u32,
    y: u32,
    node: fn(u32, u32) -> ArenaNode,
) -> u32 {
    if arena.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let arena = unsafe { &mut *arena };
    if !arena.is_valid(x) || !arena.is_valid(y) {
        return RIVAL_EXPR_INVALID;
    }
    arena.push(node(x, y))
}

#[inline]
unsafe fn arena_ternary(
    arena: *mut RivalExprArena,
    a: u32,
    b: u32,
    c: u32,
    node: fn(u32, u32, u32) -> ArenaNode,
) -> u32 {
    if arena.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let arena = unsafe { &mut *arena };
    if !arena.is_valid(a) || !arena.is_valid(b) || !arena.is_valid(c) {
        return RIVAL_EXPR_INVALID;
    }
    arena.push(node(a, b, c))
}

#[inline]
unsafe fn arena_param_unary(
    arena: *mut RivalExprArena,
    n: u64,
    x: u32,
    node: fn(u64, u32) -> ArenaNode,
) -> u32 {
    if arena.is_null() {
        return RIVAL_EXPR_INVALID;
    }
    let arena = unsafe { &mut *arena };
    if !arena.is_valid(x) {
        return RIVAL_EXPR_INVALID;
    }
    arena.push(node(n, x))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_neg(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Neg) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_fabs(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Fabs) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_sqrt(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Sqrt) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_cbrt(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Cbrt) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_pow2(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Pow2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_exp(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Exp) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_exp2(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Exp2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_expm1(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Expm1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_log(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Log) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_log2(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Log2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_log10(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Log10) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_log1p(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Log1p) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_logb(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Logb) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_sin(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Sin) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_cos(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Cos) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_tan(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Tan) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_asin(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Asin) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_acos(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Acos) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_atan(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Atan) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_sinh(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Sinh) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_cosh(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Cosh) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_tanh(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Tanh) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_asinh(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Asinh) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_acosh(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Acosh) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_atanh(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Atanh) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_erf(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Erf) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_erfc(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Erfc) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_lgamma(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Lgamma) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_tgamma(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Tgamma) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_rint(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Rint) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_round(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Round) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_ceil(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Ceil) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_floor(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Floor) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_trunc(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Trunc) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_not(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Not) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_assert(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Assert) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_error(arena: *mut RivalExprArena, x: u32) -> u32 {
    unsafe { arena_unary(arena, x, ArenaNode::Error) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_add(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Add) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_sub(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Sub) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_mul(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Mul) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_div(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Div) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_pow(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Pow) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_hypot(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Hypot) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_fmin(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Fmin) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_fmax(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Fmax) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_fdim(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Fdim) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_copysign(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Copysign) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_fmod(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Fmod) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_remainder(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Remainder) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_atan2(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Atan2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_and(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::And) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_or(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Or) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_eq(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Eq) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_ne(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Ne) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_lt(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Lt) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_le(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Le) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_gt(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Gt) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_ge(arena: *mut RivalExprArena, x: u32, y: u32) -> u32 {
    unsafe { arena_binary(arena, x, y, ArenaNode::Ge) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_fma(arena: *mut RivalExprArena, a: u32, b: u32, c: u32) -> u32 {
    unsafe { arena_ternary(arena, a, b, c, ArenaNode::Fma) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_if(arena: *mut RivalExprArena, a: u32, b: u32, c: u32) -> u32 {
    unsafe { arena_ternary(arena, a, b, c, ArenaNode::If) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_sinu(arena: *mut RivalExprArena, n: u64, x: u32) -> u32 {
    unsafe { arena_param_unary(arena, n, x, ArenaNode::Sinu) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_cosu(arena: *mut RivalExprArena, n: u64, x: u32) -> u32 {
    unsafe { arena_param_unary(arena, n, x, ArenaNode::Cosu) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_expr_tanu(arena: *mut RivalExprArena, n: u64, x: u32) -> u32 {
    unsafe { arena_param_unary(arena, n, x, ArenaNode::Tanu) }
}
