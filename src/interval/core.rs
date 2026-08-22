//! Core interval operations.

use super::value::{Endpoint, ErrorFlags, Ival, IvalClass, classify};
use crate::mpfr::{
    acosh_overflow_threshold, exp_overflow_threshold, exp2_overflow_threshold, inf, mpfr_abs,
    mpfr_acos, mpfr_acosh, mpfr_asin, mpfr_asinh, mpfr_atan, mpfr_atan2, mpfr_atanh, mpfr_cbrt,
    mpfr_ceil, mpfr_cmpabs, mpfr_cosh, mpfr_erf, mpfr_erfc, mpfr_exp, mpfr_exp2, mpfr_expm1,
    mpfr_floor, mpfr_log, mpfr_log1p, mpfr_log2, mpfr_log10, mpfr_max, mpfr_min, mpfr_neg, mpfr_pi,
    mpfr_rint, mpfr_round, mpfr_sign, mpfr_sinh, mpfr_sqrt, mpfr_tanh, mpfr_trunc,
    sinh_overflow_threshold, zero,
};
use rug::{Assign, Float, float::Round, ops::NegAssign};

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
        self.lo.immovable = endpoint_unary(mpfr_func, &a.lo, self.lo.as_float_mut(), Round::Down);
        self.hi.immovable = endpoint_unary(mpfr_func, &a.hi, self.hi.as_float_mut(), Round::Up);
        self.err = a.err;
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
        self.lo.immovable = endpoint_unary(mpfr_func, &a.hi, self.lo.as_float_mut(), Round::Down);
        self.hi.immovable = endpoint_unary(mpfr_func, &a.lo, self.hi.as_float_mut(), Round::Up);
        self.err = a.err;
    }

    pub(crate) fn overflows_loose_at(&mut self, a: &Ival, lo: Float, hi: Float) {
        let x_lo = a.lo.as_float();
        let x_hi = a.hi.as_float();

        self.lo.immovable = self.lo.immovable || x_hi <= &lo || (x_lo <= &lo && a.lo.immovable);
        self.hi.immovable = self.hi.immovable || x_lo >= &hi || (x_hi >= &hi && a.hi.immovable);
        self.err = a.err;
    }

    /// Compute the interval negation of `a`.
    pub fn neg_assign(&mut self, a: &Ival) {
        self.comonotonic_assign(&mpfr_neg, a);
    }

    pub(crate) fn exact_neg_assign(&mut self, a: &Ival) {
        let prec = a.prec();
        self.set_prec(prec);
        self.neg_assign(a);
    }

    /// Compute the interval absolute value of `x`.
    pub fn fabs_assign(&mut self, x: &Ival) {
        match classify(x, false) {
            IvalClass::Neg => self.comonotonic_assign(&mpfr_abs, x),
            IvalClass::Pos => self.monotonic_assign(&mpfr_abs, x),
            IvalClass::Mix => {
                let prec = x.prec();
                let mut tmp1 = zero(prec);
                let mut tmp2 = zero(prec);

                let abs_lo_imm = endpoint_unary(mpfr_abs, &x.lo, &mut tmp1, Round::Up);
                let abs_hi_imm = endpoint_unary(mpfr_abs, &x.hi, &mut tmp2, Round::Up);

                self.lo.as_float_mut().assign(0);
                self.lo.immovable = x.lo.immovable && x.hi.immovable;

                if tmp1 > tmp2 {
                    self.hi.as_float_mut().assign(&tmp1);
                    self.hi.immovable = abs_lo_imm;
                } else {
                    self.hi.as_float_mut().assign(&tmp2);
                    self.hi.immovable = if tmp1 == tmp2 {
                        abs_lo_imm || abs_hi_imm
                    } else {
                        abs_hi_imm
                    };
                }

                self.err = x.err;
            }
        }
    }

    pub(crate) fn exact_fabs_assign(&mut self, a: &Ival) {
        let prec = a.prec();
        self.set_prec(prec);
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
        // TODO: To get rid of all these clones when clamping, we can add inplace operators to mpfr.rs
        let mut clamped = a.clone();
        clamped.clamp(zero(self.prec()), inf(self.prec()));
        self.monotonic_assign(&mpfr_sqrt, &clamped);
    }

    /// Compute the interval cube root of `a`.
    pub fn cbrt_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_cbrt, a);
    }

    /// Compute the interval exponential of `a`.
    pub fn exp_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_exp, a);
        let thresh = exp_overflow_threshold(self.prec());
        let mut neg = thresh.clone();
        neg.neg_assign();
        self.overflows_loose_at(a, neg, thresh);
    }

    /// Compute the interval base-2 exponential of `a`.
    pub fn exp2_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_exp2, a);
        let thresh = exp2_overflow_threshold(self.prec());
        let mut neg = thresh.clone();
        neg.neg_assign();
        self.overflows_loose_at(a, neg, thresh);
    }

    /// Compute the interval `exp(a) - 1`.
    pub fn expm1_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_expm1, a);
        let thresh = exp_overflow_threshold(self.prec());
        let mut neg = thresh.clone();
        neg.neg_assign();
        self.overflows_at(a, neg, thresh);
    }

    /// Compute the interval natural logarithm of `a`.
    pub fn log_assign(&mut self, a: &Ival) {
        let mut clamped = a.clone();
        clamped.clamp_strict(zero(self.prec()), inf(self.prec()));
        self.monotonic_assign(&mpfr_log, &clamped);
    }

    /// Compute the interval base-2 logarithm of `a`.
    pub fn log2_assign(&mut self, a: &Ival) {
        let mut clamped = a.clone();
        clamped.clamp_strict(zero(self.prec()), inf(self.prec()));
        self.monotonic_assign(&mpfr_log2, &clamped);
    }

    /// Compute the interval base-10 logarithm of `a`.
    pub fn log10_assign(&mut self, a: &Ival) {
        let mut clamped = a.clone();
        clamped.clamp_strict(zero(self.prec()), inf(self.prec()));
        self.monotonic_assign(&mpfr_log10, &clamped);
    }

    /// Compute the interval `log(1 + a)`.
    pub fn log1p_assign(&mut self, a: &Ival) {
        let mut clamped = a.clone();
        let neg_one = Float::with_val(self.prec(), -1);
        clamped.clamp_strict(neg_one, inf(self.prec()));
        self.monotonic_assign(&mpfr_log1p, &clamped);
    }

    /// Compute the interval `logb` (exponent extraction) of `a`.
    pub fn logb_assign(&mut self, a: &Ival) {
        let mut abs_a = Ival::zero(a.prec());
        abs_a.exact_fabs_assign(a);

        let mut tmp = Ival::zero(self.prec());
        tmp.log2_assign(&abs_a);
        self.floor_assign(&tmp);
    }

    /// Compute the interval arcsine of `a`.
    pub fn asin_assign(&mut self, a: &Ival) {
        let mut clamped = a.clone();
        let one = Float::with_val(self.prec(), 1);
        let neg_one = Float::with_val(self.prec(), -1);
        clamped.clamp(neg_one, one);
        self.monotonic_assign(&mpfr_asin, &clamped);
    }

    /// Compute the interval arccosine of `a`.
    pub fn acos_assign(&mut self, a: &Ival) {
        let mut clamped = a.clone();
        let one = Float::with_val(self.prec(), 1);
        let neg_one = Float::with_val(self.prec(), -1);
        clamped.clamp(neg_one, one);
        self.comonotonic_assign(&mpfr_acos, &clamped);
    }

    /// Compute the interval arctangent of `a`.
    pub fn atan_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_atan, a);
    }

    /// Compute the interval two-argument arctangent `atan2(y, x)`.
    pub fn atan2_assign(&mut self, y: &Ival, x: &Ival) {
        let class_x = classify(x, true);
        let class_y = classify(y, true);

        let err = y.err.union(&x.err);
        self.err = err;

        let mut mkatan = |a: &Endpoint, b: &Endpoint, c: &Endpoint, d: &Endpoint| {
            let prec = self.prec();
            let mut lo_out = zero(prec);
            let mut hi_out = zero(prec);

            self.lo.immovable = endpoint_binary(mpfr_atan2, a, b, &mut lo_out, Round::Down);
            self.hi.immovable = endpoint_binary(mpfr_atan2, c, d, &mut hi_out, Round::Up);
            self.lo.as_float_mut().assign(&lo_out);
            self.hi.as_float_mut().assign(&hi_out);
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
                let prec = self.prec();
                let mut pi_lo = zero(prec);
                let mut pi_hi = zero(prec);
                mpfr_pi(&mut pi_lo, Round::Down);
                mpfr_pi(&mut pi_hi, Round::Up);

                let mut neg_pi = zero(prec);
                neg_pi.assign(&pi_hi);
                neg_pi.neg_assign();
                self.lo.as_float_mut().assign(&neg_pi);
                self.hi.as_float_mut().assign(&pi_hi);
                self.lo.immovable = false;
                self.hi.immovable = false;

                let x_lo = x.lo.as_float();
                let x_hi = x.hi.as_float();
                let y_lo = y.lo.as_float();
                let y_hi = y.hi.as_float();

                self.err.partial = err.partial || x_hi >= &zero(prec);
                self.err.total = err.total
                    || (x_lo.is_zero() && x_hi.is_zero() && y_lo.is_zero() && y_hi.is_zero());
            }
        }
    }

    /// Compute the interval hyperbolic sine of `a`.
    pub fn sinh_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_sinh, a);
        let thresh = sinh_overflow_threshold(self.prec());
        let mut neg = thresh.clone();
        neg.neg_assign();
        self.overflows_at(a, neg, thresh);
    }

    /// Compute the interval hyperbolic cosine of `a`.
    pub fn cosh_assign(&mut self, a: &Ival) {
        let mut abs_a = Ival::zero(a.prec());
        abs_a.exact_fabs_assign(a);
        self.monotonic_assign(&mpfr_cosh, &abs_a);
        let thresh = acosh_overflow_threshold(self.prec());
        let mut neg = thresh.clone();
        neg.neg_assign();
        self.overflows_at(&abs_a, neg, thresh);
    }

    /// Compute the interval hyperbolic tangent of `a`.
    pub fn tanh_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_tanh, a);
    }

    /// Compute the interval inverse hyperbolic sine of `a`.
    pub fn asinh_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_asinh, a);
    }

    /// Compute the interval inverse hyperbolic cosine of `a`.
    pub fn acosh_assign(&mut self, a: &Ival) {
        let mut clamped = a.clone();
        let one = Float::with_val(self.prec(), 1);
        clamped.clamp(one, inf(self.prec()));
        self.monotonic_assign(&mpfr_acosh, &clamped);
    }

    /// Compute the interval inverse hyperbolic tangent of `a`.
    pub fn atanh_assign(&mut self, a: &Ival) {
        let mut clamped = a.clone();
        let one = Float::with_val(self.prec(), 1);
        let neg_one = Float::with_val(self.prec(), -1);
        clamped.clamp_strict(neg_one, one);
        self.monotonic_assign(&mpfr_atanh, &clamped);
    }

    /// Compute the interval error function of `a`.
    pub fn erf_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_erf, a);
    }

    /// Compute the interval complementary error function of `a`.
    pub fn erfc_assign(&mut self, a: &Ival) {
        self.comonotonic_assign(&mpfr_erfc, a);
    }

    /// Compute the interval round-to-integer of `a`.
    pub fn rint_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_rint, a);
    }

    /// Compute the interval round-to-integer of `a`.
    pub fn round_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_round, a);
    }

    /// Compute the interval ceiling of `a`.
    pub fn ceil_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_ceil, a);
    }

    /// Compute the interval floor of `a`.
    pub fn floor_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_floor, a);
    }

    /// Compute the interval truncation of `a`.
    pub fn trunc_assign(&mut self, a: &Ival) {
        self.monotonic_assign(&mpfr_trunc, a);
    }

    /// Compute the interval minimum of `a` and `b`.
    pub fn fmin_assign(&mut self, a: &Ival, b: &Ival) {
        let lo_exact = mpfr_min(
            a.lo.as_float(),
            b.lo.as_float(),
            self.lo.as_float_mut(),
            Round::Down,
        );
        let hi_exact = mpfr_min(
            a.hi.as_float(),
            b.hi.as_float(),
            self.hi.as_float_mut(),
            Round::Up,
        );

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
        );
        let hi_exact = mpfr_max(
            a.hi.as_float(),
            b.hi.as_float(),
            self.hi.as_float_mut(),
            Round::Up,
        );

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
        let mut abs_x = Ival::zero(self.prec());
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
                let prec = self.prec();
                let mut neg_hi = zero(prec);
                let imm_lo = endpoint_unary(mpfr_neg, &abs_x.hi, &mut neg_hi, Round::Down);

                self.lo.as_float_mut().assign(&neg_hi);
                self.lo.immovable = imm_lo;
                self.hi.as_float_mut().assign(abs_x.hi.as_float());
                self.hi.immovable = abs_x.hi.immovable;
                self.err = err;
                if !sign_immovable {
                    self.lo.immovable = false;
                    self.hi.immovable = false;
                }
            }
            (true, false) => {
                let prec = self.prec();
                let mut neg_hi = zero(prec);
                let mut neg_lo = zero(prec);
                let imm_lo = endpoint_unary(mpfr_neg, &abs_x.hi, &mut neg_hi, Round::Down);
                let imm_hi = endpoint_unary(mpfr_neg, &abs_x.lo, &mut neg_lo, Round::Up);

                self.lo.as_float_mut().assign(&neg_hi);
                self.lo.immovable = imm_lo;
                self.hi.as_float_mut().assign(&neg_lo);
                self.hi.immovable = imm_hi;
                self.err = err;
            }
            (false, true) => {
                self.assign_from(&abs_x);
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
    }

    fn overflows_at(&mut self, a: &Ival, lo: Float, hi: Float) {
        let x_lo = a.lo.as_float();
        let x_hi = a.hi.as_float();

        self.lo.immovable = self.lo.immovable || (x_hi <= &lo && a.lo.immovable);
        self.hi.immovable = self.hi.immovable || (x_lo >= &hi && a.hi.immovable);
    }
}

#[must_use]
pub(crate) fn endpoint_unary(
    f: impl FnOnce(&Float, &mut Float, Round) -> bool,
    ep: &Endpoint,
    out: &mut Float,
    rnd: Round,
) -> bool {
    let v = ep.as_float();
    let exact = f(v, out, rnd);
    ep.immovable && exact
}

#[must_use]
pub(crate) fn endpoint_binary(
    f: impl FnOnce(&Float, &Float, &mut Float, Round) -> bool,
    ep1: &Endpoint,
    ep2: &Endpoint,
    out: &mut Float,
    rnd: Round,
) -> bool {
    let v1 = ep1.as_float();
    let v2 = ep2.as_float();
    let exact = f(v1, v2, out, rnd);
    (ep1.immovable && v1.is_infinite())
        || (ep2.immovable && v2.is_infinite())
        || (ep1.immovable && ep2.immovable && exact)
}
