//! Wrappers around MPFR functions using the `rug` and `gmp_mpfr_sys` crates.
//! Uses unsafe functions for ease of implementation, and in cases such as
//! `exp_overflow_threshold`, for efficiency
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::raw::c_ulong;
use std::sync::OnceLock;

use gmp_mpfr_sys::mpfr;
use rug::Float;
use rug::float::Round;
use rug::ops::NegAssign;

pub(crate) const INEXACT_UNKNOWN: i32 = i32::MAX;

fn to_mpfr_round(rnd: Round) -> mpfr::rnd_t {
    match rnd {
        Round::Down => mpfr::rnd_t::RNDD,
        Round::Up => mpfr::rnd_t::RNDU,
        Round::Zero => mpfr::rnd_t::RNDZ,
        Round::Nearest => mpfr::rnd_t::RNDN,
        _ => mpfr::rnd_t::RNDN,
    }
}

macro_rules! mpfr_unary_op {
    ($name:ident, $func:path) => {
        pub fn $name(input: &Float, out: &mut Float, rnd: Round) -> i32 {
            unsafe { $func(out.as_raw_mut(), input.as_raw(), to_mpfr_round(rnd)) }
        }
    };
}

macro_rules! mpfr_binary_op {
    ($name:ident, $func:path) => {
        pub fn $name(lhs: &Float, rhs: &Float, out: &mut Float, rnd: Round) -> i32 {
            unsafe {
                $func(
                    out.as_raw_mut(),
                    lhs.as_raw(),
                    rhs.as_raw(),
                    to_mpfr_round(rnd),
                )
            }
        }
    };
}

// Mutating operations

// Basic operations
mpfr_unary_op!(mpfr_neg, mpfr::neg);
mpfr_unary_op!(mpfr_abs, mpfr::abs);

// Roots and powers
mpfr_unary_op!(mpfr_sqrt, mpfr::sqrt);
mpfr_unary_op!(mpfr_cbrt, mpfr::cbrt);
mpfr_unary_op!(mpfr_pow2, mpfr::sqr);

// Exponential functions
mpfr_unary_op!(mpfr_exp, mpfr::exp);
mpfr_unary_op!(mpfr_exp2, mpfr::exp2);
mpfr_unary_op!(mpfr_expm1, mpfr::expm1);

// Logarithmic functions
mpfr_unary_op!(mpfr_log, mpfr::log);
mpfr_unary_op!(mpfr_log2, mpfr::log2);
mpfr_unary_op!(mpfr_log10, mpfr::log10);
mpfr_unary_op!(mpfr_log1p, mpfr::log1p);

// Trigonometric functions
mpfr_unary_op!(mpfr_sin, mpfr::sin);
mpfr_unary_op!(mpfr_cos, mpfr::cos);
mpfr_unary_op!(mpfr_tan, mpfr::tan);
mpfr_unary_op!(mpfr_asin, mpfr::asin);
mpfr_unary_op!(mpfr_acos, mpfr::acos);
mpfr_unary_op!(mpfr_atan, mpfr::atan);

// Hyperbolic functions
mpfr_unary_op!(mpfr_sinh, mpfr::sinh);
mpfr_unary_op!(mpfr_cosh, mpfr::cosh);
mpfr_unary_op!(mpfr_tanh, mpfr::tanh);
mpfr_unary_op!(mpfr_asinh, mpfr::asinh);
mpfr_unary_op!(mpfr_acosh, mpfr::acosh);
mpfr_unary_op!(mpfr_atanh, mpfr::atanh);

// Error functions
mpfr_unary_op!(mpfr_erf, mpfr::erf);
mpfr_unary_op!(mpfr_erfc, mpfr::erfc);

// Gamma functions
pub fn mpfr_lgamma(input: &Float, out: &mut Float, rnd: Round) -> i32 {
    let mut sign = 0;
    unsafe {
        mpfr::lgamma(
            out.as_raw_mut(),
            &mut sign,
            input.as_raw(),
            to_mpfr_round(rnd),
        )
    }
}

// Rounding functions
mpfr_unary_op!(mpfr_rint, mpfr::rint);

// Binary operations
mpfr_binary_op!(mpfr_add, mpfr::add);
mpfr_binary_op!(mpfr_sub, mpfr::sub);
mpfr_binary_op!(mpfr_mul, mpfr::mul);
mpfr_binary_op!(mpfr_div, mpfr::div);
mpfr_binary_op!(mpfr_min, mpfr::min);
mpfr_binary_op!(mpfr_max, mpfr::max);
mpfr_binary_op!(mpfr_atan2, mpfr::atan2);
mpfr_binary_op!(mpfr_pow, mpfr::pow);
mpfr_binary_op!(mpfr_hypot, mpfr::hypot);
mpfr_binary_op!(mpfr_fmod, mpfr::fmod);
mpfr_binary_op!(mpfr_remainder, mpfr::remainder);

pub fn mpfr_cosu(x: &Float, n: u64, out: &mut Float, rnd: Round) -> i32 {
    unsafe {
        mpfr::cosu(
            out.as_raw_mut(),
            x.as_raw(),
            n as c_ulong,
            to_mpfr_round(rnd),
        )
    }
}

pub fn mpfr_sinu(x: &Float, n: u64, out: &mut Float, rnd: Round) -> i32 {
    unsafe {
        mpfr::sinu(
            out.as_raw_mut(),
            x.as_raw(),
            n as c_ulong,
            to_mpfr_round(rnd),
        )
    }
}

pub fn mpfr_tanu(x: &Float, n: u64, out: &mut Float, rnd: Round) -> i32 {
    unsafe {
        mpfr::tanu(
            out.as_raw_mut(),
            x.as_raw(),
            n as c_ulong,
            to_mpfr_round(rnd),
        )
    }
}

pub fn mpfr_pi(out: &mut Float, rnd: Round) -> i32 {
    unsafe { mpfr::const_pi(out.as_raw_mut(), to_mpfr_round(rnd)) }
}

pub fn mpfr_integer(x: &Float) -> bool {
    unsafe { mpfr::integer_p(x.as_raw()) != 0 }
}

pub fn mpfr_even(x: &Float) -> bool {
    if !mpfr_integer(x) {
        return false;
    }
    crate::interval::scratch::with_float(x.prec(), |half| {
        unsafe {
            mpfr::mul_2si(
                half.as_raw_mut(),
                x.as_raw(),
                -1,
                to_mpfr_round(Round::Nearest),
            );
        }
        mpfr_integer(half)
    })
}

pub fn mpfr_odd(x: &Float) -> bool {
    mpfr_integer(x) && !mpfr_even(x)
}

pub fn mpfr_e(out: &mut Float, rnd: Round) -> i32 {
    unsafe {
        mpfr::set_ui(out.as_raw_mut(), 1, to_mpfr_round(Round::Nearest));
        mpfr::exp(out.as_raw_mut(), out.as_raw(), to_mpfr_round(rnd))
    }
}

pub fn mpfr_get_exp(x: &Float) -> i64 {
    unsafe { mpfr::get_exp(x.as_raw()) as i64 }
}

pub fn mpfr_floor_inplace(x: &mut Float) -> i32 {
    unsafe { mpfr::floor(x.as_raw_mut(), x.as_raw()) }
}

pub fn mpfr_floor(input: &Float, out: &mut Float, _rnd: Round) -> i32 {
    unsafe { mpfr::floor(out.as_raw_mut(), input.as_raw()) }
}

pub fn mpfr_ceil(input: &Float, out: &mut Float, _rnd: Round) -> i32 {
    unsafe { mpfr::ceil(out.as_raw_mut(), input.as_raw()) }
}

pub fn mpfr_round_inplace(x: &mut Float) -> i32 {
    unsafe { mpfr::rint(x.as_raw_mut(), x.as_raw(), to_mpfr_round(Round::Nearest)) }
}

pub fn mpfr_round(input: &Float, out: &mut Float, _rnd: Round) -> i32 {
    unsafe {
        mpfr::rint(
            out.as_raw_mut(),
            input.as_raw(),
            to_mpfr_round(Round::Nearest),
        )
    }
}

pub fn mpfr_trunc(input: &Float, out: &mut Float, _rnd: Round) -> i32 {
    unsafe { mpfr::trunc(out.as_raw_mut(), input.as_raw()) }
}

pub fn mpfr_cmpabs(x: &Float, y: &Float) -> i32 {
    unsafe { mpfr::cmpabs(x.as_raw(), y.as_raw()) }
}

pub fn mpfr_sign(x: &Float) -> i32 {
    unsafe { mpfr::sgn(x.as_raw()) }
}

pub(crate) fn set_prec_raw(x: &mut Float, prec: u32) {
    unsafe { mpfr::set_prec(x.as_raw_mut(), prec as mpfr::prec_t) }
}

pub(crate) fn mpfr_can_round(b: &Float, err: i64, rnd1: Round, rnd2: Round, prec: u32) -> bool {
    unsafe {
        mpfr::can_round(
            b.as_raw(),
            err as mpfr::exp_t,
            to_mpfr_round(rnd1),
            to_mpfr_round(rnd2),
            prec as mpfr::prec_t,
        ) != 0
    }
}

pub(crate) fn mpfr_set(src: &Float, out: &mut Float, rnd: Round) -> i32 {
    unsafe { mpfr::set(out.as_raw_mut(), src.as_raw(), to_mpfr_round(rnd)) }
}

pub(crate) fn mpfr_singular(x: &Float) -> bool {
    unsafe { mpfr::regular_p(x.as_raw()) == 0 }
}

pub(crate) fn mpfr_nextabove(x: &mut Float) {
    unsafe { mpfr::nextabove(x.as_raw_mut()) }
}

pub(crate) fn mpfr_nextbelow(x: &mut Float) {
    unsafe { mpfr::nextbelow(x.as_raw_mut()) }
}

pub(crate) fn set_max_finite(out: &mut Float, negative: bool) {
    unsafe {
        mpfr::set_inf(out.as_raw_mut(), 1);
        mpfr::nextbelow(out.as_raw_mut());
        if negative {
            mpfr::neg(out.as_raw_mut(), out.as_raw(), mpfr::rnd_t::RNDN);
        }
    }
}

pub(crate) fn set_min_positive(out: &mut Float, negative: bool) {
    unsafe {
        mpfr::set_zero(out.as_raw_mut(), 1);
        mpfr::nextabove(out.as_raw_mut());
        if negative {
            mpfr::neg(out.as_raw_mut(), out.as_raw(), mpfr::rnd_t::RNDN);
        }
    }
}

pub(crate) fn set_inf(out: &mut Float, negative: bool) {
    unsafe { mpfr::set_inf(out.as_raw_mut(), if negative { -1 } else { 1 }) }
}

pub(crate) fn set_zero(out: &mut Float, negative: bool) {
    unsafe { mpfr::set_zero(out.as_raw_mut(), if negative { -1 } else { 1 }) }
}

fn constant(cell: &'static OnceLock<Float>, value: f64) -> &'static Float {
    cell.get_or_init(|| Float::with_val(2, value))
}

static ZERO: OnceLock<Float> = OnceLock::new();
static ONE: OnceLock<Float> = OnceLock::new();
static NEG_ONE: OnceLock<Float> = OnceLock::new();
static INF: OnceLock<Float> = OnceLock::new();

pub(crate) fn const_zero() -> &'static Float {
    constant(&ZERO, 0.0)
}

pub(crate) fn const_one() -> &'static Float {
    constant(&ONE, 1.0)
}

pub(crate) fn const_neg_one() -> &'static Float {
    constant(&NEG_ONE, -1.0)
}

pub(crate) fn const_inf() -> &'static Float {
    constant(&INF, f64::INFINITY)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Threshold {
    Exp,
    Exp2,
    Sinh,
    Acosh,
}

thread_local! {
    static THRESHOLDS: RefCell<HashMap<(Threshold, u32), (Float, Float)>> = RefCell::new(HashMap::new());
}

pub(crate) fn with_threshold<R>(
    kind: Threshold,
    prec: u32,
    f: impl FnOnce(&Float, &Float) -> R,
) -> R {
    THRESHOLDS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= 4096 {
            cache.clear();
        }
        let (neg, pos) = cache.entry((kind, prec)).or_insert_with(|| {
            let pos = match kind {
                Threshold::Exp => exp_overflow_threshold(prec),
                Threshold::Exp2 => exp2_overflow_threshold(prec),
                Threshold::Sinh => sinh_overflow_threshold(prec),
                Threshold::Acosh => acosh_overflow_threshold(prec),
            };
            let mut neg = pos.clone();
            neg.neg_assign();
            (neg, pos)
        });
        f(neg, pos)
    })
}

// Non-mutating functions

pub fn zero(prec: u32) -> Float {
    Float::with_val(prec, 0)
}

fn overflow_threshold(
    prec: u32,
    f: unsafe extern "C" fn(*mut mpfr::mpfr_t, *const mpfr::mpfr_t, mpfr::rnd_t) -> i32,
) -> Float {
    let mut threshold = Float::with_val(prec, f64::INFINITY);
    unsafe {
        mpfr::nextbelow(threshold.as_raw_mut());
        f(
            threshold.as_raw_mut(),
            threshold.as_raw(),
            mpfr::rnd_t::RNDN,
        );
        mpfr::add_ui(
            threshold.as_raw_mut(),
            threshold.as_raw(),
            1,
            mpfr::rnd_t::RNDN,
        );
    }
    threshold
}

pub fn exp_overflow_threshold(prec: u32) -> Float {
    overflow_threshold(prec, mpfr::log)
}

pub fn exp2_overflow_threshold(prec: u32) -> Float {
    overflow_threshold(prec, mpfr::log2)
}

pub fn sinh_overflow_threshold(prec: u32) -> Float {
    overflow_threshold(prec, mpfr::asinh)
}

pub fn acosh_overflow_threshold(prec: u32) -> Float {
    overflow_threshold(prec, mpfr::acosh)
}
