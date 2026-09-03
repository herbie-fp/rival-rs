//! Core interval operations.

use super::scratch::{with_float, with_float2, with_ival, with_ival2};
use super::value::{Endpoint, ErrorFlags, Ival, IvalClass, classify};
use crate::mpfr::{
    INEXACT_UNKNOWN, Threshold, const_inf, const_neg_one, const_one, const_zero, mpfr_abs,
    mpfr_acos, mpfr_acosh, mpfr_asin, mpfr_asinh, mpfr_atan, mpfr_atan2, mpfr_atanh,
    mpfr_can_round, mpfr_cbrt, mpfr_ceil, mpfr_cmpabs, mpfr_cosh, mpfr_erf, mpfr_erfc, mpfr_exp,
    mpfr_exp2, mpfr_expm1, mpfr_floor, mpfr_get_exp, mpfr_log, mpfr_log1p, mpfr_log2, mpfr_log10,
    mpfr_max, mpfr_min, mpfr_neg, mpfr_nextabove, mpfr_nextbelow, mpfr_pi, mpfr_rint, mpfr_round,
    mpfr_set, mpfr_sign, mpfr_singular, mpfr_sinh, mpfr_sqrt, mpfr_tanh, mpfr_trunc, set_inf,
    set_max_finite, set_min_positive, set_zero, with_threshold,
};
use rug::{Assign, Float, float::Round, ops::NegAssign};

const SHARE_EXTRA_BITS: u32 = 64;
const SHARE_MAX_EXP: i64 = 1 << 29;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Share {
    Never,
    Algebraic,
    Transcendental,
}

impl Share {
    #[inline]
    fn applies(self, prec: u32) -> bool {
        match self {
            Share::Never => false,
            Share::Algebraic => prec >= 512,
            Share::Transcendental => true,
        }
    }
}

#[inline]
pub(crate) fn same_value(a: &Float, b: &Float) -> bool {
    a == b && a.is_sign_negative() == b.is_sign_negative()
}

fn round_both(
    b: &Float,
    t: i32,
    prec: u32,
    lo: &mut Float,
    hi: &mut Float,
) -> Option<(bool, bool)> {
    if t == 0 {
        return Some((
            mpfr_set(b, lo, Round::Down) == 0,
            mpfr_set(b, hi, Round::Up) == 0,
        ));
    }
    if mpfr_singular(b) {
        let negative = b.is_sign_negative();
        if b.is_infinite() {
            if negative {
                set_inf(lo, true);
                set_max_finite(hi, true);
            } else {
                set_max_finite(lo, false);
                set_inf(hi, false);
            }
            return Some((false, false));
        }
        if b.is_zero() {
            if negative {
                set_min_positive(lo, true);
                set_zero(hi, true);
            } else {
                set_zero(lo, false);
                set_min_positive(hi, false);
            }
            return Some((false, false));
        }
        return None;
    }
    if mpfr_get_exp(b).abs() >= SHARE_MAX_EXP {
        return None;
    }
    let err = (b.prec() + 1) as i64;
    if mpfr_can_round(b, err, Round::Nearest, Round::Down, prec)
        && mpfr_can_round(b, err, Round::Nearest, Round::Up, prec)
    {
        mpfr_set(b, lo, Round::Down);
        mpfr_set(b, hi, Round::Up);
        return Some((false, false));
    }
    if t != INEXACT_UNKNOWN && mpfr_set(b, lo, Round::Down) == 0 {
        mpfr_set(b, hi, Round::Up);
        if t < 0 {
            mpfr_nextabove(hi);
        } else {
            mpfr_nextbelow(lo);
        }
        return Some((false, false));
    }
    None
}

pub(crate) fn unary_pair<F>(
    f: &F,
    x: &Float,
    lo: &mut Float,
    hi: &mut Float,
    share: Share,
) -> (bool, bool)
where
    F: Fn(&Float, &mut Float, Round) -> i32,
{
    let prec = lo.prec();
    if share.applies(prec) {
        let shared = with_float(prec + SHARE_EXTRA_BITS, |b| {
            let t = f(x, b, Round::Nearest);
            round_both(b, t, prec, lo, hi)
        });
        if let Some(r) = shared {
            return r;
        }
    }
    (f(x, lo, Round::Down) == 0, f(x, hi, Round::Up) == 0)
}

pub(crate) fn binary_pair<F>(
    f: &F,
    x: &Float,
    y: &Float,
    lo: &mut Float,
    hi: &mut Float,
    share: Share,
) -> (bool, bool)
where
    F: Fn(&Float, &Float, &mut Float, Round) -> i32,
{
    let prec = lo.prec();
    if share.applies(prec) {
        let shared = with_float(prec + SHARE_EXTRA_BITS, |b| {
            let t = f(x, y, b, Round::Nearest);
            round_both(b, t, prec, lo, hi)
        });
        if let Some(r) = shared {
            return r;
        }
    }
    (f(x, y, lo, Round::Down) == 0, f(x, y, hi, Round::Up) == 0)
}

#[inline]
pub(crate) fn apply_unary<F>(
    f: &F,
    x_lo: &Float,
    lo_imm: bool,
    x_hi: &Float,
    hi_imm: bool,
    out: &mut Ival,
    share: Share,
) where
    F: Fn(&Float, &mut Float, Round) -> i32,
{
    let (lo_exact, hi_exact) = if share != Share::Never && same_value(x_lo, x_hi) {
        let Ival { lo, hi, .. } = out;
        unary_pair(f, x_lo, lo.as_float_mut(), hi.as_float_mut(), share)
    } else {
        (
            f(x_lo, out.lo.as_float_mut(), Round::Down) == 0,
            f(x_hi, out.hi.as_float_mut(), Round::Up) == 0,
        )
    };
    out.lo.immovable = lo_imm && lo_exact;
    out.hi.immovable = hi_imm && hi_exact;
}

#[inline]
fn binary_immovable(ep1: &Endpoint, ep2: &Endpoint, exact: bool) -> bool {
    (ep1.immovable && ep1.as_float().is_infinite())
        || (ep2.immovable && ep2.as_float().is_infinite())
        || (ep1.immovable && ep2.immovable && exact)
}

#[inline]
pub(crate) fn apply_binary<F>(
    f: &F,
    lo_a: &Endpoint,
    lo_b: &Endpoint,
    hi_a: &Endpoint,
    hi_b: &Endpoint,
    out: &mut Ival,
    share: Share,
) where
    F: Fn(&Float, &Float, &mut Float, Round) -> i32,
{
    let (lo_exact, hi_exact) = if share != Share::Never
        && same_value(lo_a.as_float(), hi_a.as_float())
        && same_value(lo_b.as_float(), hi_b.as_float())
    {
        let Ival { lo, hi, .. } = out;
        binary_pair(
            f,
            lo_a.as_float(),
            lo_b.as_float(),
            lo.as_float_mut(),
            hi.as_float_mut(),
            share,
        )
    } else {
        (
            f(
                lo_a.as_float(),
                lo_b.as_float(),
                out.lo.as_float_mut(),
                Round::Down,
            ) == 0,
            f(
                hi_a.as_float(),
                hi_b.as_float(),
                out.hi.as_float_mut(),
                Round::Up,
            ) == 0,
        )
    };
    out.lo.immovable = binary_immovable(lo_a, lo_b, lo_exact);
    out.hi.immovable = binary_immovable(hi_a, hi_b, hi_exact);
}

struct Clamped<'a> {
    lo: &'a Float,
    hi: &'a Float,
    err: ErrorFlags,
}

fn clamp_view<'a>(a: &'a Ival, lo: &'a Float, hi: &'a Float) -> Clamped<'a> {
    let x_lo = a.lo.as_float();
    let x_hi = a.hi.as_float();
    let err = ErrorFlags::new(
        a.err.partial || x_lo < lo || x_hi > hi,
        a.err.total || x_hi < lo || x_lo > hi,
    );
    if lo.is_zero() && x_hi.is_zero() {
        Clamped {
            lo: const_zero(),
            hi: const_zero(),
            err,
        }
    } else {
        Clamped {
            lo: if x_lo < lo { lo } else { x_lo },
            hi: if x_hi > hi { hi } else { x_hi },
            err,
        }
    }
}

fn clamp_strict_view<'a>(a: &'a Ival, lo: &'a Float, hi: &'a Float) -> Clamped<'a> {
    let x_lo = a.lo.as_float();
    let x_hi = a.hi.as_float();
    let err = ErrorFlags::new(
        a.err.partial || x_lo <= lo || x_hi >= hi,
        a.err.total || x_hi <= lo || x_lo >= hi,
    );
    Clamped {
        lo: if x_lo < lo { lo } else { x_lo },
        hi: if x_hi > hi { hi } else { x_hi },
        err,
    }
}

impl Ival {
    /// Lift a (weakly) monotonic MPFR function to a function on intervals.
    ///
    /// A weakly monotonic function is one where larger inputs produce
    /// larger (or equal) outputs. Note that if a non-monotonic function
    /// is passed, the results will not be sound.
    pub fn monotonic_assign<F>(&mut self, mpfr_func: &F, a: &Ival)
    where
        F: Fn(&Float, &mut Float, Round) -> bool,
    {
        self.monotonic_with(&ternary_of(mpfr_func), a, Share::Transcendental);
    }

    /// Lift a (weakly) co-monotonic MPFR function to a function on intervals.
    ///
    /// A weakly co-monotonic function is one where larger inputs produce
    /// smaller (or equal) outputs. Note that if a non-co-monotonic function
    /// is passed, the results will not be sound.
    pub fn comonotonic_assign<F>(&mut self, mpfr_func: &F, a: &Ival)
    where
        F: Fn(&Float, &mut Float, Round) -> bool,
    {
        self.comonotonic_with(&ternary_of(mpfr_func), a, Share::Transcendental);
    }

    #[inline]
    pub(crate) fn monotonic_with<F>(&mut self, f: &F, a: &Ival, share: Share)
    where
        F: Fn(&Float, &mut Float, Round) -> i32,
    {
        apply_unary(
            f,
            a.lo.as_float(),
            a.lo.immovable,
            a.hi.as_float(),
            a.hi.immovable,
            self,
            share,
        );
        self.err = a.err;
    }

    #[inline]
    pub(crate) fn comonotonic_with<F>(&mut self, f: &F, a: &Ival, share: Share)
    where
        F: Fn(&Float, &mut Float, Round) -> i32,
    {
        apply_unary(
            f,
            a.hi.as_float(),
            a.hi.immovable,
            a.lo.as_float(),
            a.lo.immovable,
            self,
            share,
        );
        self.err = a.err;
    }

    fn clamped_monotonic<F>(&mut self, f: &F, a: &Ival, c: Clamped, share: Share)
    where
        F: Fn(&Float, &mut Float, Round) -> i32,
    {
        apply_unary(f, c.lo, a.lo.immovable, c.hi, a.hi.immovable, self, share);
        self.err = c.err;
    }

    fn clamped_comonotonic<F>(&mut self, f: &F, a: &Ival, c: Clamped, share: Share)
    where
        F: Fn(&Float, &mut Float, Round) -> i32,
    {
        apply_unary(f, c.hi, a.hi.immovable, c.lo, a.lo.immovable, self, share);
        self.err = c.err;
    }

    pub(crate) fn overflows_loose_at(&mut self, a: &Ival, lo: &Float, hi: &Float) {
        let x_lo = a.lo.as_float();
        let x_hi = a.hi.as_float();

        self.lo.immovable = self.lo.immovable || x_hi <= lo || (x_lo <= lo && a.lo.immovable);
        self.hi.immovable = self.hi.immovable || x_lo >= hi || (x_hi >= hi && a.hi.immovable);
        self.err = a.err;
    }

    /// Compute the interval negation of `a`.
    pub fn neg_assign(&mut self, a: &Ival) {
        self.comonotonic_with(&mpfr_neg, a, Share::Never);
    }

    pub(crate) fn exact_neg_assign(&mut self, a: &Ival) {
        self.set_prec_raw(a.prec());
        self.neg_assign(a);
    }

    /// Compute the interval absolute value of `x`.
    pub fn fabs_assign(&mut self, x: &Ival) {
        match classify(x, false) {
            IvalClass::Neg => self.comonotonic_with(&mpfr_abs, x, Share::Never),
            IvalClass::Pos => self.monotonic_with(&mpfr_abs, x, Share::Never),
            IvalClass::Mix => {
                with_float2(x.prec(), |tmp1, tmp2| {
                    let abs_lo_imm = endpoint_unary(mpfr_abs, &x.lo, tmp1, Round::Up);
                    let abs_hi_imm = endpoint_unary(mpfr_abs, &x.hi, tmp2, Round::Up);

                    self.lo.as_float_mut().assign(0);
                    self.lo.immovable = x.lo.immovable && x.hi.immovable;

                    if *tmp1 > *tmp2 {
                        self.hi.as_float_mut().assign(&*tmp1);
                        self.hi.immovable = abs_lo_imm;
                    } else {
                        self.hi.as_float_mut().assign(&*tmp2);
                        self.hi.immovable = if *tmp1 == *tmp2 {
                            abs_lo_imm || abs_hi_imm
                        } else {
                            abs_hi_imm
                        };
                    }
                });
                self.err = x.err;
            }
        }
    }

    pub(crate) fn exact_fabs_assign(&mut self, a: &Ival) {
        self.set_prec_raw(a.prec());
        self.fabs_assign(a);
    }

    pub(crate) fn pre_fabs_assign(&mut self, x: &Ival) {
        match classify(x, false) {
            IvalClass::Pos => {
                self.assign_from(x);
            }
            IvalClass::Neg => {
                self.lo.as_float_mut().assign(x.hi.as_float());
                self.lo.immovable = x.hi.immovable;
                self.hi.as_float_mut().assign(x.lo.as_float());
                self.hi.immovable = x.lo.immovable;
                self.err = x.err;
            }
            IvalClass::Mix => {
                self.lo.as_float_mut().assign(0);
                self.lo.immovable = x.lo.immovable && x.hi.immovable;

                if mpfr_cmpabs(x.lo.as_float(), x.hi.as_float()) > 0 {
                    self.hi.as_float_mut().assign(x.lo.as_float());
                    self.hi.immovable = x.lo.immovable;
                } else {
                    self.hi.as_float_mut().assign(x.hi.as_float());
                    self.hi.immovable = x.hi.immovable;
                }
                self.err = x.err;
            }
        }
    }

    /// Compute the interval square root of `a`.
    pub fn sqrt_assign(&mut self, a: &Ival) {
        let c = clamp_view(a, const_zero(), const_inf());
        self.clamped_monotonic(&mpfr_sqrt, a, c, Share::Algebraic);
    }

    /// Compute the interval cube root of `a`.
    pub fn cbrt_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_cbrt, a, Share::Transcendental);
    }

    /// Compute the interval exponential of `a`.
    pub fn exp_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_exp, a, Share::Transcendental);
        with_threshold(Threshold::Exp, self.prec(), |neg, pos| {
            self.overflows_loose_at(a, neg, pos)
        });
    }

    /// Compute the interval base-2 exponential of `a`.
    pub fn exp2_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_exp2, a, Share::Transcendental);
        with_threshold(Threshold::Exp2, self.prec(), |neg, pos| {
            self.overflows_loose_at(a, neg, pos)
        });
    }

    /// Compute the interval `exp(a) - 1`.
    pub fn expm1_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_expm1, a, Share::Transcendental);
        with_threshold(Threshold::Exp, self.prec(), |neg, pos| {
            self.overflows_at(a, neg, pos)
        });
    }

    /// Compute the interval natural logarithm of `a`.
    pub fn log_assign(&mut self, a: &Ival) {
        let c = clamp_strict_view(a, const_zero(), const_inf());
        self.clamped_monotonic(&mpfr_log, a, c, Share::Transcendental);
    }

    /// Compute the interval base-2 logarithm of `a`.
    pub fn log2_assign(&mut self, a: &Ival) {
        let c = clamp_strict_view(a, const_zero(), const_inf());
        self.clamped_monotonic(&mpfr_log2, a, c, Share::Transcendental);
    }

    /// Compute the interval base-10 logarithm of `a`.
    pub fn log10_assign(&mut self, a: &Ival) {
        let c = clamp_strict_view(a, const_zero(), const_inf());
        self.clamped_monotonic(&mpfr_log10, a, c, Share::Transcendental);
    }

    /// Compute the interval `log(1 + a)`.
    pub fn log1p_assign(&mut self, a: &Ival) {
        let c = clamp_strict_view(a, const_neg_one(), const_inf());
        self.clamped_monotonic(&mpfr_log1p, a, c, Share::Transcendental);
    }

    /// Compute the interval `logb` (exponent extraction) of `a`.
    pub fn logb_assign(&mut self, a: &Ival) {
        with_ival(a.prec(), |abs_a| {
            abs_a.exact_fabs_assign(a);
            with_ival(self.prec(), |tmp| {
                tmp.log2_assign(abs_a);
                self.floor_assign(tmp);
            });
        });
    }

    /// Compute the interval arcsine of `a`.
    pub fn asin_assign(&mut self, a: &Ival) {
        let c = clamp_view(a, const_neg_one(), const_one());
        self.clamped_monotonic(&mpfr_asin, a, c, Share::Transcendental);
    }

    /// Compute the interval arccosine of `a`.
    pub fn acos_assign(&mut self, a: &Ival) {
        let c = clamp_view(a, const_neg_one(), const_one());
        self.clamped_comonotonic(&mpfr_acos, a, c, Share::Transcendental);
    }

    /// Compute the interval arctangent of `a`.
    pub fn atan_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_atan, a, Share::Transcendental);
    }

    /// Compute the interval two-argument arctangent `atan2(y, x)`.
    pub fn atan2_assign(&mut self, y: &Ival, x: &Ival) {
        let class_x = classify(x, true);
        let class_y = classify(y, true);

        let err = y.err.union(&x.err);
        self.err = err;

        let mut mkatan = |a: &Endpoint, b: &Endpoint, c: &Endpoint, d: &Endpoint| {
            apply_binary(&mpfr_atan2, a, b, c, d, self, Share::Transcendental);
            self.err = err;
        };

        match (class_x, class_y) {
            (IvalClass::Neg, IvalClass::Neg) => mkatan(&y.hi, &x.lo, &y.lo, &x.hi),
            (IvalClass::Mix, IvalClass::Neg) => mkatan(&y.hi, &x.lo, &y.hi, &x.hi),
            (IvalClass::Pos, IvalClass::Neg) => mkatan(&y.lo, &x.lo, &y.hi, &x.hi),
            (IvalClass::Pos, IvalClass::Mix) => mkatan(&y.lo, &x.lo, &y.hi, &x.lo),
            (IvalClass::Pos, IvalClass::Pos) => mkatan(&y.lo, &x.hi, &y.hi, &x.lo),
            (IvalClass::Mix, IvalClass::Pos) => mkatan(&y.lo, &x.hi, &y.lo, &x.lo),
            (IvalClass::Neg, IvalClass::Pos) => mkatan(&y.hi, &x.hi, &y.lo, &x.lo),
            (_, IvalClass::Mix) => {
                mpfr_pi(self.hi.as_float_mut(), Round::Up);
                mpfr_pi(self.lo.as_float_mut(), Round::Up);
                self.lo.as_float_mut().neg_assign();
                self.lo.immovable = false;
                self.hi.immovable = false;

                let x_lo = x.lo.as_float();
                let x_hi = x.hi.as_float();
                let y_lo = y.lo.as_float();
                let y_hi = y.hi.as_float();

                self.err.partial = err.partial || *x_hi >= 0;
                self.err.total = err.total
                    || (x_lo.is_zero() && x_hi.is_zero() && y_lo.is_zero() && y_hi.is_zero());
            }
        }
    }

    /// Compute the interval hyperbolic sine of `a`.
    pub fn sinh_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_sinh, a, Share::Transcendental);
        with_threshold(Threshold::Sinh, self.prec(), |neg, pos| {
            self.overflows_at(a, neg, pos)
        });
    }

    /// Compute the interval hyperbolic cosine of `a`.
    pub fn cosh_assign(&mut self, a: &Ival) {
        with_ival(a.prec(), |abs_a| {
            abs_a.exact_fabs_assign(a);
            self.monotonic_with(&mpfr_cosh, abs_a, Share::Transcendental);
            with_threshold(Threshold::Acosh, self.prec(), |neg, pos| {
                self.overflows_at(abs_a, neg, pos)
            });
        });
    }

    /// Compute the interval hyperbolic tangent of `a`.
    pub fn tanh_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_tanh, a, Share::Transcendental);
    }

    /// Compute the interval inverse hyperbolic sine of `a`.
    pub fn asinh_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_asinh, a, Share::Transcendental);
    }

    /// Compute the interval inverse hyperbolic cosine of `a`.
    pub fn acosh_assign(&mut self, a: &Ival) {
        let c = clamp_view(a, const_one(), const_inf());
        self.clamped_monotonic(&mpfr_acosh, a, c, Share::Transcendental);
    }

    /// Compute the interval inverse hyperbolic tangent of `a`.
    pub fn atanh_assign(&mut self, a: &Ival) {
        let c = clamp_strict_view(a, const_neg_one(), const_one());
        self.clamped_monotonic(&mpfr_atanh, a, c, Share::Transcendental);
    }

    /// Compute the interval error function of `a`.
    pub fn erf_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_erf, a, Share::Transcendental);
    }

    /// Compute the interval complementary error function of `a`.
    pub fn erfc_assign(&mut self, a: &Ival) {
        self.comonotonic_with(&mpfr_erfc, a, Share::Transcendental);
    }

    /// Compute the interval round-to-integer of `a`.
    pub fn rint_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_rint, a, Share::Never);
    }

    /// Compute the interval round-to-integer of `a`.
    pub fn round_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_round, a, Share::Never);
    }

    /// Compute the interval ceiling of `a`.
    pub fn ceil_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_ceil, a, Share::Never);
    }

    /// Compute the interval floor of `a`.
    pub fn floor_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_floor, a, Share::Never);
    }

    /// Compute the interval truncation of `a`.
    pub fn trunc_assign(&mut self, a: &Ival) {
        self.monotonic_with(&mpfr_trunc, a, Share::Never);
    }

    /// Compute the interval minimum of `a` and `b`.
    pub fn fmin_assign(&mut self, a: &Ival, b: &Ival) {
        let lo_exact = mpfr_min(
            a.lo.as_float(),
            b.lo.as_float(),
            self.lo.as_float_mut(),
            Round::Down,
        ) == 0;
        let hi_exact = mpfr_min(
            a.hi.as_float(),
            b.hi.as_float(),
            self.hi.as_float_mut(),
            Round::Up,
        ) == 0;

        let lo_stable = if a.lo.immovable && b.lo.immovable {
            true
        } else if a.lo.immovable {
            a.lo.as_float() <= b.lo.as_float()
        } else if b.lo.immovable {
            b.lo.as_float() <= a.lo.as_float()
        } else {
            false
        };

        self.lo.immovable = lo_exact && lo_stable;
        self.hi.immovable = hi_exact && a.hi.immovable && b.hi.immovable;
        self.err = a.err.union(&b.err);
    }

    /// Compute the interval maximum of `a` and `b`.
    pub fn fmax_assign(&mut self, a: &Ival, b: &Ival) {
        let lo_exact = mpfr_max(
            a.lo.as_float(),
            b.lo.as_float(),
            self.lo.as_float_mut(),
            Round::Down,
        ) == 0;
        let hi_exact = mpfr_max(
            a.hi.as_float(),
            b.hi.as_float(),
            self.hi.as_float_mut(),
            Round::Up,
        ) == 0;

        let hi_stable = if a.hi.immovable && b.hi.immovable {
            true
        } else if a.hi.immovable {
            a.hi.as_float() >= b.hi.as_float()
        } else if b.hi.immovable {
            b.hi.as_float() >= a.hi.as_float()
        } else {
            false
        };

        self.lo.immovable = lo_exact && a.lo.immovable && b.lo.immovable;
        self.hi.immovable = hi_exact && hi_stable;
        self.err = a.err.union(&b.err);
    }

    /// Compute the interval `copysign(x, y)`.
    pub fn copysign_assign(&mut self, x: &Ival, y: &Ival) {
        with_ival(self.prec(), |abs_x| {
            abs_x.fabs_assign(x);

            let y_lo = y.lo.as_float();
            let y_hi = y.hi.as_float();

            let can_zero = y_lo.is_zero() || y_hi.is_zero();
            let can_neg = mpfr_sign(y_lo) == -1 || can_zero;
            let can_pos = mpfr_sign(y_hi) == 1 || can_zero;
            let sign_immovable = can_neg && can_pos && y.lo.immovable && y.hi.immovable;

            let err = y.err.union(&abs_x.err);

            match (can_neg, can_pos) {
                (true, true) => {
                    self.lo.immovable =
                        endpoint_unary(mpfr_neg, &abs_x.hi, self.lo.as_float_mut(), Round::Down);
                    self.hi.as_float_mut().assign(abs_x.hi.as_float());
                    self.hi.immovable = abs_x.hi.immovable;
                    self.err = err;
                    if !sign_immovable {
                        self.lo.immovable = false;
                        self.hi.immovable = false;
                    }
                }
                (true, false) => {
                    self.lo.immovable =
                        endpoint_unary(mpfr_neg, &abs_x.hi, self.lo.as_float_mut(), Round::Down);
                    self.hi.immovable =
                        endpoint_unary(mpfr_neg, &abs_x.lo, self.hi.as_float_mut(), Round::Up);
                    self.err = err;
                }
                (false, true) => {
                    self.assign_from(abs_x);
                    self.err = err;
                }
                (false, false) => {
                    self.lo.as_float_mut().assign(f64::NAN);
                    self.hi.as_float_mut().assign(f64::NAN);
                    self.lo.immovable = true;
                    self.hi.immovable = true;
                    self.err = ErrorFlags::error();
                }
            }
        });
    }

    fn overflows_at(&mut self, a: &Ival, lo: &Float, hi: &Float) {
        let x_lo = a.lo.as_float();
        let x_hi = a.hi.as_float();

        self.lo.immovable = self.lo.immovable || (x_hi <= lo && a.lo.immovable);
        self.hi.immovable = self.hi.immovable || (x_lo >= hi && a.hi.immovable);
    }
}

fn ternary_of<F>(f: &F) -> impl Fn(&Float, &mut Float, Round) -> i32
where
    F: Fn(&Float, &mut Float, Round) -> bool,
{
    move |x: &Float, out: &mut Float, rnd: Round| if f(x, out, rnd) { 0 } else { INEXACT_UNKNOWN }
}

#[must_use]
pub(crate) fn endpoint_unary(
    f: impl FnOnce(&Float, &mut Float, Round) -> i32,
    ep: &Endpoint,
    out: &mut Float,
    rnd: Round,
) -> bool {
    let v = ep.as_float();
    let exact = f(v, out, rnd) == 0;
    ep.immovable && exact
}

#[must_use]
pub(crate) fn endpoint_binary(
    f: impl FnOnce(&Float, &Float, &mut Float, Round) -> i32,
    ep1: &Endpoint,
    ep2: &Endpoint,
    out: &mut Float,
    rnd: Round,
) -> bool {
    let v1 = ep1.as_float();
    let v2 = ep2.as_float();
    let exact = f(v1, v2, out, rnd) == 0;
    binary_immovable(ep1, ep2, exact)
}

pub(crate) fn split_zero(x: &Ival, f: impl FnOnce(&Ival, &Ival)) {
    with_ival2(x.prec(), |neg, pos| {
        x.split_into(const_zero(), neg, pos);
        f(neg, pos)
    })
}
