//! Power interval operations.

use super::core::{Share, binary_pair, same_value, split_zero};
use super::scratch::{with_float, with_ival, with_ival2};
use super::value::{Endpoint, ErrorFlags, Ival, IvalClass, classify};
use crate::mpfr::{
    Threshold, const_zero, mpfr_get_exp, mpfr_integer, mpfr_log2, mpfr_odd, mpfr_pow, mpfr_pow2,
    mpfr_sign, with_threshold,
};
use rug::{Assign, Float, float::Round};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PosIvalClass1 {
    GreaterOrEqual = 1,
    Less = -1,
    Straddles = 0,
}

impl Ival {
    /// Compute the interval power `x^y`.
    pub fn pow_assign(&mut self, x: &Ival, y: &Ival) {
        let x_lo = x.lo.as_float();
        let x_hi = x.hi.as_float();

        let hi_is_neg = mpfr_sign(x_hi) == -1 && !x_hi.is_zero();
        let lo_is_pos = mpfr_sign(x_lo) == 1 || x_lo.is_zero();

        if hi_is_neg {
            self.pow_neg_assign(x, y);
        } else if lo_is_pos {
            self.pow_pos_assign(x, y);
        } else {
            split_zero(x, |neg, pos| {
                with_ival2(self.prec(), |neg_result, pos_result| {
                    neg_result.pow_neg_assign(neg, y);
                    pos_result.pow_pos_assign(pos, y);
                    self.assign_from(neg_result);
                    self.union_with(pos_result);
                });
            });
            if !(x.lo.immovable && x.hi.immovable) {
                self.lo.immovable = false;
                self.hi.immovable = false;
            }
        }
    }

    pub fn pow2_assign(&mut self, x: &Ival) {
        with_ival(x.prec(), |abs_x| {
            abs_x.pre_fabs_assign(x);
            self.monotonic_with(&mpfr_pow2, abs_x, Share::Algebraic);
        });
    }

    fn pow_pos_assign(&mut self, x: &Ival, y: &Ival) {
        let x_class = classify_pos_ival_1(x);
        let y_class = classify(y, false);

        let mk_pow =
            |out: &mut Ival, lo_a: &Endpoint, lo_b: &Endpoint, hi_a: &Endpoint, hi_b: &Endpoint| {
                let prec = out.prec();
                let (lo_imm, hi_imm) = eppow_pair(lo_a, lo_b, hi_a, hi_b, x_class, y_class, out);
                let lo_zero = out.lo.as_float().is_zero();
                let hi_inf = out.hi.as_float().is_infinite();

                let (real_lo_imm, real_hi_imm) = if lo_zero || hi_inf {
                    with_threshold(Threshold::Exp2, prec, |_, threshold| {
                        let log2_sum_exceeds_threshold = |exp: &Float, base: &Float| -> bool {
                            with_float(prec, |log2_base| {
                                mpfr_log2(base, log2_base, Round::Zero);
                                mpfr_get_exp(exp).saturating_add(mpfr_get_exp(log2_base))
                                    > mpfr_get_exp(threshold)
                            })
                        };

                        let x_class_i = x_class as i32;
                        let y_class_i = y_class as i32;

                        let must_overflow = hi_inf
                            && x_class_i * y_class_i == 1
                            && log2_sum_exceeds_threshold(lo_b.as_float(), lo_a.as_float());

                        let must_underflow = lo_zero
                            && x_class_i * y_class_i == -1
                            && log2_sum_exceeds_threshold(hi_b.as_float(), hi_a.as_float());

                        let new_lo_imm = lo_imm
                            || must_underflow
                            || (lo_zero && lo_a.immovable && lo_b.immovable);

                        let new_hi_imm = hi_imm
                            || must_underflow
                            || must_overflow
                            || (hi_inf && hi_a.immovable && hi_b.immovable);

                        (new_lo_imm, new_hi_imm)
                    })
                } else {
                    (lo_imm, hi_imm)
                };

                out.lo.immovable = real_lo_imm;
                out.hi.immovable = real_hi_imm;

                let x_lo_zero = x.lo.as_float().is_zero();
                out.err = x.err.union(&y.err);
                if x_lo_zero && !matches!(y_class, IvalClass::Pos) {
                    out.err.partial = true;
                }
                if x.hi.as_float().is_zero()
                    && matches!(y_class, IvalClass::Neg)
                    && !y.hi.as_float().is_zero()
                {
                    out.err.total = true;
                }
            };

        let xlo = &x.lo;
        let xhi = &x.hi;
        let ylo = &y.lo;
        let yhi = &y.hi;

        match (x_class, y_class) {
            (PosIvalClass1::GreaterOrEqual, IvalClass::Pos) => mk_pow(self, xlo, ylo, xhi, yhi),
            (PosIvalClass1::GreaterOrEqual, IvalClass::Mix) => mk_pow(self, xhi, ylo, xhi, yhi),
            (PosIvalClass1::GreaterOrEqual, IvalClass::Neg) => mk_pow(self, xhi, ylo, xlo, yhi),
            (PosIvalClass1::Straddles, IvalClass::Pos) => mk_pow(self, xlo, yhi, xhi, yhi),
            (PosIvalClass1::Straddles, IvalClass::Neg) => mk_pow(self, xhi, ylo, xlo, ylo),
            (PosIvalClass1::Less, IvalClass::Pos) => mk_pow(self, xlo, yhi, xhi, ylo),
            (PosIvalClass1::Less, IvalClass::Mix) => mk_pow(self, xlo, yhi, xlo, ylo),
            (PosIvalClass1::Less, IvalClass::Neg) => mk_pow(self, xhi, yhi, xlo, ylo),
            (PosIvalClass1::Straddles, IvalClass::Mix) => {
                mk_pow(self, xlo, yhi, xhi, yhi);
                with_ival(self.prec(), |tmp| {
                    mk_pow(tmp, xhi, ylo, xlo, ylo);
                    self.union_with(tmp);
                });
            }
        }
    }

    fn pow_neg_assign(&mut self, x: &Ival, y: &Ival) {
        let y_lo = y.lo.as_float();
        let y_hi = y.hi.as_float();

        if y_lo == y_hi {
            if mpfr_integer(y_lo) {
                with_ival(x.prec(), |abs_x| {
                    abs_x.exact_fabs_assign(x);
                    self.pow_pos_assign(abs_x, y);
                    if mpfr_odd(y_lo) {
                        self.neg_inplace();
                    }
                });
            } else {
                self.lo.as_float_mut().assign(f64::NAN);
                self.hi.as_float_mut().assign(f64::NAN);
                self.lo.immovable = true;
                self.hi.immovable = true;
                self.err = ErrorFlags::error();
            }
        } else {
            with_ival(x.prec(), |abs_x| {
                abs_x.exact_fabs_assign(x);
                with_ival2(self.prec(), |pos_pow, neg_pow| {
                    pos_pow.pow_pos_assign(abs_x, y);
                    neg_pow.neg_assign(pos_pow);
                    self.assign_from(pos_pow);
                    self.union_with(neg_pow);
                    self.err.partial = true;
                });
            });
        }
    }
}

fn classify_pos_ival_1(x: &Ival) -> PosIvalClass1 {
    let x_lo = x.lo.as_float();
    if mpfr_get_exp(x_lo) >= 1 {
        return PosIvalClass1::GreaterOrEqual;
    }

    let x_hi = x.hi.as_float();
    if mpfr_get_exp(x_hi) < 1 && !x_hi.is_infinite() {
        return PosIvalClass1::Less;
    }

    PosIvalClass1::Straddles
}

#[inline]
fn effective_base(v: &Float) -> &Float {
    if v.is_zero() { const_zero() } else { v }
}

fn eppow_pair(
    lo_a: &Endpoint,
    lo_b: &Endpoint,
    hi_a: &Endpoint,
    hi_b: &Endpoint,
    a_class: PosIvalClass1,
    b_class: IvalClass,
    out: &mut Ival,
) -> (bool, bool) {
    let lo_base = effective_base(lo_a.as_float());
    let hi_base = effective_base(hi_a.as_float());
    let lo_exp = lo_b.as_float();
    let hi_exp = hi_b.as_float();

    let (lo_exact, hi_exact) = if same_value(lo_base, hi_base) && same_value(lo_exp, hi_exp) {
        let Ival { lo, hi, .. } = out;
        binary_pair(
            &mpfr_pow,
            lo_base,
            lo_exp,
            lo.as_float_mut(),
            hi.as_float_mut(),
            Share::Transcendental,
        )
    } else {
        (
            mpfr_pow(lo_base, lo_exp, out.lo.as_float_mut(), Round::Down) == 0,
            mpfr_pow(hi_base, hi_exp, out.hi.as_float_mut(), Round::Up) == 0,
        )
    };

    (
        eppow_immovable(lo_a, lo_b, lo_base, a_class, b_class, lo_exact),
        eppow_immovable(hi_a, hi_b, hi_base, a_class, b_class, hi_exact),
    )
}

fn eppow_immovable(
    a: &Endpoint,
    b: &Endpoint,
    base: &Float,
    a_class: PosIvalClass1,
    b_class: IvalClass,
    exact: bool,
) -> bool {
    let exp_val = b.as_float();
    let a_imm = a.immovable;
    let b_imm = b.immovable;

    (a_imm && b_imm && exact)
        || (a_imm && *base == 1)
        || (a_imm && base.is_zero() && !matches!(b_class, IvalClass::Mix))
        || (a_imm && base.is_infinite() && !matches!(b_class, IvalClass::Mix))
        || (b_imm && exp_val.is_zero())
        || (b_imm && exp_val.is_infinite() && !matches!(a_class, PosIvalClass1::Straddles))
}
