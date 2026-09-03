use super::core::{Share, endpoint_unary};
use super::value::{ErrorFlags, Ival};
use crate::mpfr::{
    mpfr_add, mpfr_ceil, mpfr_div, mpfr_even, mpfr_floor, mpfr_integer, mpfr_lgamma, mpfr_log,
    mpfr_neg, mpfr_pi, mpfr_sub, zero,
};
use rug::{Assign, Float, float::Round};

impl Ival {
    /// Compute the interval logarithm of the absolute value of gamma of `x`.
    pub fn lgamma_assign(&mut self, x: &Ival) {
        let prec = self.prec();
        let (neg, pos) = split_optional(x, &zero(x.prec()));

        match (neg, pos) {
            (Some(neg), Some(pos)) => {
                self.lgamma_pos_assign(&pos);
                let mut neg_result = Ival::zero(prec);
                neg_result.lgamma_neg_assign(&neg);
                self.union_assign(neg_result);
            }
            (Some(neg), None) => self.lgamma_neg_assign(&neg),
            (None, Some(pos)) => self.lgamma_pos_assign(&pos),
            (None, None) => unreachable!("splitting an interval must retain at least one half"),
        }
    }

    /// Compute the interval gamma of `x`.
    pub fn tgamma_assign(&mut self, x: &Ival) {
        let prec = self.prec();
        let mut log_y = Ival::zero(prec);
        log_y.lgamma_assign(x);

        let mut abs_y = Ival::zero(prec);
        abs_y.exp_assign(&log_y);

        let lo = x.lo.as_float();
        let hi = x.hi.as_float();
        if lo >= &zero(prec) {
            self.assign_from(&abs_y);
            return;
        }

        let mut lo_floor = zero(lo.prec());
        let mut hi_floor = zero(hi.prec());
        mpfr_floor(lo, &mut lo_floor, Round::Nearest);
        mpfr_floor(hi, &mut hi_floor, Round::Nearest);

        if lo_floor != hi_floor {
            self.lo.as_float_mut().assign(f64::NEG_INFINITY);
            self.hi.as_float_mut().assign(f64::INFINITY);
            self.lo.immovable = x.lo.immovable;
            self.hi.immovable = x.hi.immovable;
            self.err = ErrorFlags::new(true, x.err.total);
        } else if lo == hi && mpfr_integer(lo) {
            self.lo.as_float_mut().assign(f64::NAN);
            self.hi.as_float_mut().assign(f64::NAN);
            self.lo.immovable = true;
            self.hi.immovable = true;
            self.err = ErrorFlags::error();
        } else if mpfr_even(&lo_floor) {
            self.assign_from(&abs_y);
        } else {
            self.neg_assign(&abs_y);
        }
    }

    fn lgamma_pos_assign(&mut self, x: &Ival) {
        let prec = self.prec();
        if x.lo.as_float() >= &Float::with_val(prec, 1.5) {
            self.monotonic_with(&mpfr_lgamma, x, Share::Transcendental);
        } else if x.lo.as_float() >= &zero(prec) && x.hi.as_float() <= &Float::with_val(prec, 1.4) {
            self.comonotonic_with(&mpfr_lgamma, x, Share::Transcendental);
        } else {
            let (min_x, min_y) = convex_find_min(
                Float::with_val(prec, 1.46163),
                Float::with_val(prec, 1.46164),
                prec,
            );
            self.convex_lgamma_assign(x, &min_x, &min_y);
        }
    }

    fn lgamma_neg_assign(&mut self, x: &Ival) {
        let prec = self.prec();
        let mut split = zero(prec);
        mpfr_ceil(x.lo.as_float(), &mut split, Round::Nearest);

        let (left, rest) = split_optional(x, &split);
        let (right, _) = if let Some(rest) = rest {
            mpfr_add(
                &Float::with_val(prec, 1),
                rest.lo.as_float(),
                &mut split,
                Round::Up,
            );
            split_optional(&rest, &split)
        } else {
            (None, None)
        };

        match (left, right) {
            (Some(left), Some(right)) => {
                self.lgamma_basin_assign(&left);
                let mut right_result = Ival::zero(prec);
                right_result.lgamma_basin_assign(&right);
                self.union_assign(right_result);
            }
            (Some(left), None) => self.lgamma_basin_assign(&left),
            (None, Some(right)) => self.lgamma_basin_assign(&right),
            (None, None) => self.set_full_range(),
        }
    }

    fn lgamma_basin_assign(&mut self, x: &Ival) {
        let prec = self.prec();
        let mut min = zero(prec);
        let mut max = zero(prec);
        mpfr_floor(x.lo.as_float(), &mut min, Round::Nearest);
        mpfr_ceil(x.hi.as_float(), &mut max, Round::Nearest);

        if within_steps(&min, &max, 24) {
            self.lgamma_basin_bound_assign(&min);
            self.err = self.err.union(&x.err);
        } else {
            let (min_x, min_y) = convex_find_min(min, max, prec);
            self.convex_lgamma_assign(x, &min_x, &min_y);
        }
    }

    fn lgamma_basin_bound_assign(&mut self, min: &Float) {
        let prec = self.prec();
        let mut log_pi = zero(prec);
        let mut pi = zero(prec);

        mpfr_pi(&mut pi, Round::Down);
        mpfr_log(&pi, &mut log_pi, Round::Down);

        let mut neg_min = zero(prec);
        let mut one_minus_min = zero(prec);
        let mut log_gamma = zero(prec);
        mpfr_neg(min, &mut neg_min, Round::Nearest);
        mpfr_add(
            &Float::with_val(prec, 1),
            &neg_min,
            &mut one_minus_min,
            Round::Up,
        );
        mpfr_lgamma(&one_minus_min, &mut log_gamma, Round::Up);
        mpfr_sub(&log_pi, &log_gamma, self.lo.as_float_mut(), Round::Down);

        self.hi.as_float_mut().assign(f64::INFINITY);
        self.lo.immovable = false;
        self.hi.immovable = false;
        self.err = ErrorFlags::none();
    }

    fn convex_lgamma_assign(&mut self, x: &Ival, min_x: &Float, min_y: &Float) {
        if x.lo.as_float() > min_x {
            self.monotonic_with(&mpfr_lgamma, x, Share::Transcendental);
            return;
        }
        if x.hi.as_float() < min_x {
            self.comonotonic_with(&mpfr_lgamma, x, Share::Transcendental);
            return;
        }

        self.lo.as_float_mut().assign(min_y);
        self.lo.immovable = false;

        let mut tmp = zero(self.prec());
        let lo_immovable = endpoint_unary(mpfr_lgamma, &x.lo, &mut tmp, Round::Up);
        let hi_immovable = endpoint_unary(mpfr_lgamma, &x.hi, self.hi.as_float_mut(), Round::Up);
        if tmp > *self.hi.as_float() {
            self.hi.as_float_mut().assign(&tmp);
            self.hi.immovable = lo_immovable;
        } else {
            self.hi.immovable = hi_immovable || (tmp == *self.hi.as_float() && lo_immovable);
        }
        self.err = x.err;
    }

    fn set_full_range(&mut self) {
        self.lo.as_float_mut().assign(f64::NEG_INFINITY);
        self.hi.as_float_mut().assign(f64::INFINITY);
        self.lo.immovable = false;
        self.hi.immovable = false;
        self.err = ErrorFlags::none();
    }
}

fn convex_find_min(mut lo: Float, mut hi: Float, prec: u32) -> (Float, Float) {
    let mut mid_lo = weighted_third(&lo, &hi, false, prec);
    let mut mid_hi = weighted_third(&lo, &hi, true, prec);

    loop {
        let y_lo = lgamma(&lo, prec, Round::Up);
        let y_mid_lo = lgamma(&mid_lo, prec, Round::Down);
        let y_mid_hi = lgamma(&mid_hi, prec, Round::Down);
        let y_hi = lgamma(&hi, prec, Round::Up);

        if within_steps(&lo, &hi, 3) {
            let mut delta_lo = zero(prec);
            let mut delta_hi = zero(prec);
            mpfr_sub(&y_mid_lo, &y_lo, &mut delta_lo, Round::Up);
            mpfr_sub(&y_mid_hi, &y_hi, &mut delta_hi, Round::Up);

            let mut delta = zero(prec);
            mpfr_div(
                if delta_lo > delta_hi {
                    &delta_lo
                } else {
                    &delta_hi
                },
                &Float::with_val(prec, 2),
                &mut delta,
                Round::Up,
            );

            let mut min_y = zero(prec);
            mpfr_add(&y_mid_lo, &delta, &mut min_y, Round::Down);
            return (mid_lo, min_y);
        }

        if within_steps(&mid_lo, &mid_hi, 1) {
            lo = mid_lo.clone();
            lo.next_down();
            hi = mid_hi.clone();
            hi.next_up();
        } else if y_mid_lo > y_mid_hi {
            lo = mid_lo;
            mid_lo = mid_hi;
            mid_hi = midpoint(&mid_lo, &hi, prec);
        } else {
            hi = mid_hi;
            mid_hi = mid_lo;
            mid_lo = midpoint(&lo, &mid_hi, prec);
        }
    }
}

fn weighted_third(lo: &Float, hi: &Float, favor_hi: bool, prec: u32) -> Float {
    let (double, single) = if favor_hi { (hi, lo) } else { (lo, hi) };
    let mut double_value = zero(prec);
    let mut sum = zero(prec);
    let mut third = zero(prec);
    mpfr_add(double, double, &mut double_value, Round::Nearest);
    mpfr_add(&double_value, single, &mut sum, Round::Nearest);
    mpfr_div(&sum, &Float::with_val(prec, 3), &mut third, Round::Nearest);
    keep_inside(&mut third, lo, hi);
    third
}

fn midpoint(lo: &Float, hi: &Float, prec: u32) -> Float {
    let mut sum = zero(prec);
    let mut mid = zero(prec);
    mpfr_add(lo, hi, &mut sum, Round::Nearest);
    mpfr_div(&sum, &Float::with_val(prec, 2), &mut mid, Round::Nearest);
    keep_inside(&mut mid, lo, hi);
    mid
}

fn keep_inside(value: &mut Float, lo: &Float, hi: &Float) {
    if &*value <= lo {
        value.assign(lo);
        value.next_up();
    } else if &*value >= hi {
        value.assign(hi);
        value.next_down();
    }
}

fn split_optional(x: &Ival, val: &Float) -> (Option<Ival>, Option<Ival>) {
    let val = Float::with_val(x.prec(), val);
    if x.hi.as_float() <= &val {
        (Some(x.clone()), None)
    } else if x.lo.as_float() >= &val {
        (None, Some(x.clone()))
    } else {
        let (lo, hi) = x.split_at(&val);
        (Some(lo), Some(hi))
    }
}

fn within_steps(lo: &Float, hi: &Float, limit: usize) -> bool {
    let mut current = lo.clone();
    for _ in 0..=limit {
        if &current >= hi {
            return true;
        }
        current.next_up();
    }
    false
}

fn lgamma(x: &Float, prec: u32, round: Round) -> Float {
    let mut out = zero(prec);
    mpfr_lgamma(x, &mut out, round);
    out
}
