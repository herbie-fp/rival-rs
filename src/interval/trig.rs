use super::scratch::{with_float, with_float4};
use super::value::{Endpoint, Ival, IvalClass, classify};
use crate::{
    interval::core::{Share, endpoint_unary},
    mpfr::{
        mpfr_cos, mpfr_cosu, mpfr_div, mpfr_floor_inplace, mpfr_get_exp, mpfr_pi,
        mpfr_round_inplace, mpfr_sin, mpfr_sinu, mpfr_tan, mpfr_tanu,
    },
};
use rug::{Assign, Float, float::Round};

const RANGE_REDUCE_PRECISION_CAP: u32 = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PeriodClass {
    TooWide,
    NearZero,
    RangeReduce,
}

#[derive(Clone, Copy)]
struct Reduced {
    equal: bool,
    even: bool,
    diff_one: bool,
}

fn classify_ival_periodic(x: &Ival, period_quarter_bitlen: i64) -> PeriodClass {
    let (xlo, xhi) = (x.lo.as_float(), x.hi.as_float());

    if xlo.is_infinite() || xhi.is_infinite() {
        return PeriodClass::TooWide;
    }

    let (lo_exp, hi_exp) = (mpfr_get_exp(xlo), mpfr_get_exp(xhi));
    if lo_exp < period_quarter_bitlen && hi_exp < period_quarter_bitlen {
        return PeriodClass::NearZero;
    }

    let lo_ulp = lo_exp.saturating_sub(xlo.prec() as i64);
    let hi_ulp = hi_exp.saturating_sub(xhi.prec() as i64);

    if lo_ulp > 0 || hi_ulp > 0 {
        if xlo == xhi {
            PeriodClass::RangeReduce
        } else {
            PeriodClass::TooWide
        }
    } else {
        PeriodClass::RangeReduce
    }
}

fn range_reduce_precision(xlo: &Float, xhi: &Float, curr_prec: u32) -> u32 {
    let lo = (mpfr_get_exp(xlo) + 2 * (xlo.prec() as i64)).max(curr_prec as i64) as u32;
    let hi = (mpfr_get_exp(xhi) + 2 * (xhi.prec() as i64)).max(curr_prec as i64) as u32;
    lo.max(hi).min(RANGE_REDUCE_PRECISION_CAP).max(curr_prec)
}

fn reduced_flags(a: &Float, b: &Float) -> Reduced {
    let equal = *a == *b;
    let even = bfeven(a);
    let diff_one = !equal && bfsub_is_one(a, b);
    Reduced {
        equal,
        even,
        diff_one,
    }
}

fn reduce_by_pi<R>(
    x: &Ival,
    round_fn: fn(&mut Float) -> i32,
    k: impl FnOnce(&Float, &Float) -> R,
) -> R {
    let prec = range_reduce_precision(x.lo.as_float(), x.hi.as_float(), x.prec());
    with_float4(prec, |pi_lo, pi_hi, q_lo, q_hi| {
        mpfr_pi(pi_lo, Round::Down);
        mpfr_pi(pi_hi, Round::Up);
        mpfr_div(x.lo.as_float(), pi_hi, q_lo, Round::Down);
        mpfr_div(x.hi.as_float(), pi_lo, q_hi, Round::Up);
        round_fn(q_lo);
        round_fn(q_hi);
        k(q_lo, q_hi)
    })
}

fn reduce_by_n_half<R>(
    x: &Ival,
    n: u64,
    round_fn: fn(&mut Float) -> i32,
    k: impl FnOnce(&Float, &Float) -> R,
) -> R {
    let prec = range_reduce_precision(x.lo.as_float(), x.hi.as_float(), x.prec());
    with_float4(prec, |n_half, q_lo, q_hi, _| {
        n_half.assign(n);
        *n_half /= 2u32;
        mpfr_div(x.lo.as_float(), n_half, q_lo, Round::Down);
        mpfr_div(x.hi.as_float(), n_half, q_hi, Round::Up);
        round_fn(q_lo);
        round_fn(q_hi);
        k(q_lo, q_hi)
    })
}

fn bfeven(x: &Float) -> bool {
    with_float(x.prec(), |t| {
        t.assign(x);
        *t /= 2;
        mpfr_floor_inplace(t);
        *t *= 2;
        *t == *x
    })
}

fn bfsub_is_one(a: &Float, b: &Float) -> bool {
    with_float(a.prec().max(b.prec()), |d| {
        d.assign(b);
        *d -= a;
        *d == 1
    })
}

fn period_quarter_bitlen(n: u64, divisor: u64) -> i64 {
    let quarter = n / divisor;
    if quarter > 0 {
        (quarter.ilog2() + 1) as i64
    } else {
        0
    }
}

fn endpoint_min<F>(f: &F, lo: &Endpoint, hi: &Endpoint, out: &mut Float) -> bool
where
    F: Fn(&Float, &mut Float, Round) -> i32,
{
    with_float(out.prec(), |tmp| {
        let imm_lo = endpoint_unary(f, lo, out, Round::Down);
        let imm_hi = endpoint_unary(f, hi, tmp, Round::Down);

        if *tmp < *out {
            out.assign(&*tmp);
            imm_hi
        } else if *tmp == *out {
            imm_lo || imm_hi
        } else {
            imm_lo
        }
    })
}

fn endpoint_max<F>(f: &F, lo: &Endpoint, hi: &Endpoint, out: &mut Float) -> bool
where
    F: Fn(&Float, &mut Float, Round) -> i32,
{
    with_float(out.prec(), |tmp| {
        let imm_lo = endpoint_unary(f, lo, out, Round::Up);
        let imm_hi = endpoint_unary(f, hi, tmp, Round::Up);

        if *tmp > *out {
            out.assign(&*tmp);
            imm_hi
        } else if *tmp == *out {
            imm_lo || imm_hi
        } else {
            imm_lo
        }
    })
}

impl Ival {
    fn set_unit_range(&mut self) {
        self.lo.as_float_mut().assign(-1);
        self.hi.as_float_mut().assign(1);
        self.lo.immovable = false;
        self.hi.immovable = false;
    }

    fn set_full_range_with(&mut self, immovable: bool, total: bool) {
        self.lo.as_float_mut().assign(f64::NEG_INFINITY);
        self.hi.as_float_mut().assign(f64::INFINITY);
        self.lo.immovable = immovable;
        self.hi.immovable = immovable;
        self.err.partial = true;
        self.err.total = total;
    }

    fn cos_like<F>(&mut self, f: &F, x: &Ival, class: PeriodClass, reduce: impl FnOnce() -> Reduced)
    where
        F: Fn(&Float, &mut Float, Round) -> i32,
    {
        self.err = x.err;
        match class {
            PeriodClass::TooWide => self.set_unit_range(),
            PeriodClass::NearZero => match classify(x, false) {
                IvalClass::Neg => self.monotonic_with(f, x, Share::Transcendental),
                IvalClass::Pos => self.comonotonic_with(f, x, Share::Transcendental),
                IvalClass::Mix => {
                    self.set_prec_raw(x.prec());
                    self.lo.immovable = endpoint_min(f, &x.lo, &x.hi, self.lo.as_float_mut());
                    self.hi.as_float_mut().assign(1);
                    self.hi.immovable = false;
                }
            },
            PeriodClass::RangeReduce => {
                let r = reduce();
                if r.equal && r.even {
                    self.comonotonic_with(f, x, Share::Transcendental);
                } else if r.equal {
                    self.monotonic_with(f, x, Share::Transcendental);
                } else if r.diff_one && r.even {
                    self.set_prec_raw(x.prec());
                    self.lo.as_float_mut().assign(-1);
                    self.lo.immovable = false;
                    self.hi.immovable = endpoint_max(f, &x.lo, &x.hi, self.hi.as_float_mut());
                } else if r.diff_one {
                    self.set_prec_raw(x.prec());
                    self.lo.immovable = endpoint_min(f, &x.lo, &x.hi, self.lo.as_float_mut());
                    self.hi.as_float_mut().assign(1);
                    self.hi.immovable = false;
                } else {
                    self.set_unit_range();
                }
            }
        }
    }

    fn sin_like<F>(&mut self, f: &F, x: &Ival, class: PeriodClass, reduce: impl FnOnce() -> Reduced)
    where
        F: Fn(&Float, &mut Float, Round) -> i32,
    {
        self.err = x.err;
        match class {
            PeriodClass::TooWide => self.set_unit_range(),
            PeriodClass::NearZero => self.monotonic_with(f, x, Share::Transcendental),
            PeriodClass::RangeReduce => {
                let r = reduce();
                if r.equal && r.even {
                    self.monotonic_with(f, x, Share::Transcendental);
                } else if r.equal {
                    self.comonotonic_with(f, x, Share::Transcendental);
                } else if r.diff_one && !r.even {
                    self.set_prec_raw(x.prec());
                    self.lo.as_float_mut().assign(-1);
                    self.lo.immovable = false;
                    self.hi.immovable = endpoint_max(f, &x.lo, &x.hi, self.hi.as_float_mut());
                } else if r.diff_one {
                    self.set_prec_raw(x.prec());
                    self.lo.immovable = endpoint_min(f, &x.lo, &x.hi, self.lo.as_float_mut());
                    self.hi.as_float_mut().assign(1);
                    self.hi.immovable = false;
                } else {
                    self.set_unit_range();
                }
            }
        }
    }

    fn tan_like<F>(&mut self, f: &F, x: &Ival, class: PeriodClass, reduce: impl FnOnce() -> bool)
    where
        F: Fn(&Float, &mut Float, Round) -> i32,
    {
        let immovable = x.lo.immovable && x.hi.immovable;
        match class {
            PeriodClass::TooWide => self.set_full_range_with(immovable, x.err.total),
            PeriodClass::NearZero => {
                self.monotonic_with(f, x, Share::Transcendental);
                self.err = x.err;
            }
            PeriodClass::RangeReduce => {
                if reduce() {
                    self.monotonic_with(f, x, Share::Transcendental);
                    self.err = x.err;
                } else {
                    self.set_full_range_with(immovable, x.err.total);
                }
            }
        }
    }

    /// Compute the interval cosine of `x`.
    pub fn cos_assign(&mut self, x: &Ival) {
        let class = classify_ival_periodic(x, 1);
        self.cos_like(&mpfr_cos, x, class, || {
            reduce_by_pi(x, mpfr_floor_inplace, reduced_flags)
        });
    }

    /// Compute the interval sine of `x`.
    pub fn sin_assign(&mut self, x: &Ival) {
        let class = classify_ival_periodic(x, 1);
        self.sin_like(&mpfr_sin, x, class, || {
            reduce_by_pi(x, mpfr_round_inplace, reduced_flags)
        });
    }

    /// Compute the interval tangent of `x`.
    pub fn tan_assign(&mut self, x: &Ival) {
        let class = classify_ival_periodic(x, 0);
        self.tan_like(&mpfr_tan, x, class, || {
            reduce_by_pi(x, mpfr_round_inplace, |a, b| *a == *b)
        });
    }

    pub(crate) fn cosu_assign(&mut self, x: &Ival, n: u64) {
        let class = classify_ival_periodic(x, period_quarter_bitlen(n, 4));
        let cosu = |x: &Float, out: &mut Float, rnd: Round| mpfr_cosu(x, n, out, rnd);
        self.cos_like(&cosu, x, class, || {
            reduce_by_n_half(x, n, mpfr_floor_inplace, reduced_flags)
        });
    }

    pub(crate) fn sinu_assign(&mut self, x: &Ival, n: u64) {
        let class = classify_ival_periodic(x, period_quarter_bitlen(n, 4));
        let sinu = |x: &Float, out: &mut Float, rnd: Round| mpfr_sinu(x, n, out, rnd);
        self.sin_like(&sinu, x, class, || {
            reduce_by_n_half(x, n, mpfr_round_inplace, reduced_flags)
        });
    }

    pub(crate) fn tanu_assign(&mut self, x: &Ival, n: u64) {
        let class = classify_ival_periodic(x, period_quarter_bitlen(n, 8));
        let tanu = |x: &Float, out: &mut Float, rnd: Round| mpfr_tanu(x, n, out, rnd);
        self.tan_like(&tanu, x, class, || {
            reduce_by_n_half(x, n, mpfr_round_inplace, |a, b| *a == *b)
        });
    }
}
