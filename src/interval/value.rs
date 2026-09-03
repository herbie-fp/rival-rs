use rug::{
    Assign, Float,
    float::{OrdFloat, Round},
    ops::AssignRound,
};

use crate::mpfr::set_prec_raw;
use rug::ops::NegAssign;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Endpoint {
    pub(crate) val: OrdFloat,
    pub(crate) immovable: bool,
}

/// A standard Rival interval containing two arbitrary-precision endpoints.
///
/// A standard interval includes both endpoints. Neither endpoint
/// is allowed to be NaN. Intervals can be either *real* (with
/// [`rug::Float`] endpoints) or *boolean* (with endpoints 0 or 1).
///
/// # Boolean intervals
///
/// In a boolean interval, `false` is considered less than `true`,
/// yielding three boolean interval values:
/// - True: `[1, 1]` — constructed with [`Ival::bool_interval(true, true)`](Ival::bool_interval)
/// - False: `[0, 0]` — constructed with [`Ival::bool_interval(false, false)`](Ival::bool_interval)
/// - Uncertain: `[0, 1]` — constructed with [`Ival::bool_interval(false, true)`](Ival::bool_interval)
///
/// # Error intervals
///
/// Sometimes an interval will contain invalid inputs to some function.
/// For example, `sqrt` is undefined for negative inputs! In cases
/// like this, Rival's output interval will only consider valid inputs.
/// Error flags are "sticky": further computations on an interval
/// will maintain already-set error flags.
///
/// # Interval Operations
///
/// Rival aims to ensure three properties of all helper functions:
///
/// - **Soundness** means output intervals contain any output on inputs drawn
///   from the input intervals. IEEE-1788 refers to this as the output interval
///   being *valid*.
///
/// - **Refinement** means, moreover, that narrower input intervals lead to
///   narrower output intervals. Rival's movability flags make this a somewhat
///   more complicated property than typical.
///
/// - **Weak completeness** means, moreover, that Rival returns the narrowest
///   possible valid interval. IEEE-1788 refers to this as the output interval
///   being *tight*.
///
/// Weak completeness (tightness) is the strongest possible property,
/// while soundness (validity) is the weakest, with refinement somewhere
/// in between.
///
/// The typical use case for Rival is to recompute a certain expression at
/// ever higher precision, until the computed interval is narrow enough.
/// However, interval arithmetic is not complete. For example, due to the
/// limitations of the underlying MPFR library, it's impossible to compute
/// `(exp(x) / exp(x))` for large enough values of `x`.
///
/// While it's impossible to detect this in all cases, Rival provides
/// support for *movability flags* that can detect many such instances
/// automatically. Movability flags are correctly propagated by all of
/// Rival's supported operations, and are set by functions such as `exp`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ival {
    pub(crate) lo: Endpoint,
    pub(crate) hi: Endpoint,
    pub(crate) err: ErrorFlags,
}

/// Flags indicating whether invalid inputs were discarded during computation.
///
/// When an interval contains invalid inputs to some function (e.g.,
/// negative inputs to `sqrt`), these flags record what happened:
///
/// - [`partial`](ErrorFlags::partial): at least one invalid input was discarded, but some
///   valid inputs remain.
/// - [`total`](ErrorFlags::total): all inputs were invalid.
///
/// Error flags are "sticky": further computations on an interval
/// will maintain already-set error flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorFlags {
    pub(crate) partial: bool,
    pub(crate) total: bool,
}

impl Endpoint {
    pub(crate) fn new(val: OrdFloat, immovable: bool) -> Self {
        Endpoint { val, immovable }
    }

    #[inline]
    pub(crate) fn as_float(&self) -> &Float {
        self.val.as_float()
    }

    #[inline]
    pub(crate) fn as_float_mut(&mut self) -> &mut Float {
        self.val.as_float_mut()
    }

    pub(crate) fn assign_endpoint(&mut self, b: &Endpoint) {
        set_prec_raw(self.as_float_mut(), b.as_float().prec());
        self.as_float_mut().assign(b.as_float());
        self.immovable = b.immovable;
    }

    pub(crate) fn min_with(&mut self, b: &Endpoint) {
        use std::cmp::Ordering;
        match self.val.cmp(&b.val) {
            Ordering::Less => (),
            Ordering::Greater => self.assign_endpoint(b),
            Ordering::Equal => self.immovable |= b.immovable,
        }
    }

    pub(crate) fn max_with(&mut self, b: &Endpoint) {
        use std::cmp::Ordering;
        match self.val.cmp(&b.val) {
            Ordering::Greater => (),
            Ordering::Less => self.assign_endpoint(b),
            Ordering::Equal => self.immovable |= b.immovable,
        }
    }

    pub(crate) fn endpoint_min2_assign(&mut self, b: Endpoint) {
        use std::cmp::Ordering;
        match self.val.cmp(&b.val) {
            Ordering::Less => (),
            Ordering::Greater => *self = b,
            Ordering::Equal => self.immovable |= b.immovable,
        }
    }

    pub(crate) fn endpoint_max2_assign(&mut self, b: Endpoint) {
        use std::cmp::Ordering;
        match self.val.cmp(&b.val) {
            Ordering::Greater => (),
            Ordering::Less => *self = b,
            Ordering::Equal => self.immovable |= b.immovable,
        }
    }
}

impl Ival {
    pub(crate) fn new(lo: Endpoint, hi: Endpoint, err: ErrorFlags) -> Self {
        assert!(lo.as_float().prec() == hi.as_float().prec());
        Ival { lo, hi, err }
    }

    /// Returns the low endpoint of this interval.
    #[inline]
    pub fn lo(&self) -> &Float {
        self.lo.as_float()
    }

    /// Returns the high endpoint of this interval.
    #[inline]
    pub fn hi(&self) -> &Float {
        self.hi.as_float()
    }

    #[inline]
    pub fn lo_mut(&mut self) -> &mut Float {
        self.lo.as_float_mut()
    }

    #[inline]
    pub fn hi_mut(&mut self) -> &mut Float {
        self.hi.as_float_mut()
    }

    #[inline]
    pub fn lo_immovable(&self) -> bool {
        self.lo.immovable
    }

    #[inline]
    pub fn hi_immovable(&self) -> bool {
        self.hi.immovable
    }

    #[inline]
    pub fn set_immovable(&mut self, lo: bool, hi: bool) {
        self.lo.immovable = lo;
        self.hi.immovable = hi;
    }

    #[inline]
    pub fn error_flags(&self) -> ErrorFlags {
        self.err
    }

    #[inline]
    pub fn set_error_flags(&mut self, err: ErrorFlags) {
        self.err = err;
    }

    #[inline]
    pub fn prec(&self) -> u32 {
        self.lo.as_float().prec()
    }

    #[inline]
    pub fn set_prec(&mut self, prec: u32) {
        self.lo.as_float_mut().set_prec(prec);
        self.hi.as_float_mut().set_prec(prec);
    }

    #[inline]
    pub(crate) fn set_prec_raw(&mut self, prec: u32) {
        set_prec_raw(self.lo.as_float_mut(), prec);
        set_prec_raw(self.hi.as_float_mut(), prec);
    }

    pub(crate) fn reset_zero(&mut self, prec: u32) {
        self.set_prec_raw(prec);
        self.lo.as_float_mut().assign(0);
        self.hi.as_float_mut().assign(0);
        self.lo.immovable = true;
        self.hi.immovable = true;
        self.err = ErrorFlags::none();
    }

    pub(crate) fn set_bool(&mut self, lo_true: bool, hi_true: bool) {
        self.set_prec_raw(2);
        self.lo.as_float_mut().assign(if lo_true { 1 } else { 0 });
        self.hi.as_float_mut().assign(if hi_true { 1 } else { 0 });
        self.lo.immovable = true;
        self.hi.immovable = true;
        self.err = ErrorFlags::none();
    }

    pub(crate) fn neg_inplace(&mut self) {
        self.lo.as_float_mut().neg_assign();
        self.hi.as_float_mut().neg_assign();
        std::mem::swap(&mut self.lo, &mut self.hi);
    }

    /// Construct an interval from two endpoints.
    ///
    /// If either endpoint is NaN, or if `lo == hi` and both
    /// are infinite, an illegal interval is returned (with error
    /// flags set). The interval is considered movable.
    pub fn from_lo_hi(lo: Float, hi: Float) -> Self {
        let err = if lo.is_nan() || hi.is_nan() || (lo.eq(&hi) && lo.is_infinite()) {
            ErrorFlags::error()
        } else {
            ErrorFlags::none()
        };
        Ival {
            lo: Endpoint::new(OrdFloat::from(lo), false),
            hi: Endpoint::new(OrdFloat::from(hi), false),
            err,
        }
    }

    /// Construct a boolean interval.
    ///
    /// A boolean interval has 2-bit precision endpoints:
    /// `false` is represented as `0` and `true` as `1`.
    /// Boolean intervals are always immovable.
    #[inline]
    pub fn bool_interval(lo_true: bool, hi_true: bool) -> Self {
        // 2-bit precision is sufficient for 0/1 endpoints.
        let to_float = |b: bool| Float::with_val(2, if b { 1 } else { 0 });
        let (lo, hi) = (to_float(lo_true), to_float(hi_true));
        Ival {
            lo: Endpoint::new(OrdFloat::from(lo), true),
            hi: Endpoint::new(OrdFloat::from(hi), true),
            err: ErrorFlags::none(),
        }
    }

    pub fn f64_assign(&mut self, value: f64) {
        self.lo.as_float_mut().assign_round(value, Round::Down);
        self.hi.as_float_mut().assign_round(value, Round::Up);
        self.err = ErrorFlags::none();
    }

    pub fn zero(prec: u32) -> Self {
        let lo = Float::with_val(prec, 0);
        let hi = Float::with_val(prec, 0);
        Ival::new(
            Endpoint::new(OrdFloat::from(lo), true),
            Endpoint::new(OrdFloat::from(hi), true),
            ErrorFlags::none(),
        )
    }

    pub(crate) fn assign_from(&mut self, src: &Ival) {
        let src_prec = src.prec();
        set_prec_raw(self.lo.as_float_mut(), src_prec);
        set_prec_raw(self.hi.as_float_mut(), src_prec);
        self.lo.as_float_mut().assign(src.lo.as_float());
        self.lo.immovable = src.lo.immovable;
        self.hi.as_float_mut().assign(src.hi.as_float());
        self.hi.immovable = src.hi.immovable;
        self.err = src.err;
    }

    /// Compute the union of this interval with `other`.
    ///
    /// Maintains error flags, and movability flags when possible.
    /// If either interval is totally in error, the other is used
    /// with its partial error flag set.
    pub fn union_assign(&mut self, other: Ival) {
        if self.err.total {
            self.lo = other.lo;
            self.hi = other.hi;
            self.err = other.err;
            self.err.partial = true;
            return;
        }

        if other.err.total {
            self.err.partial = true;
            return;
        }

        self.lo.endpoint_min2_assign(other.lo);
        self.hi.endpoint_max2_assign(other.hi);
        self.err = self.err.union_disjoint(&other.err);
    }

    pub(crate) fn union_with(&mut self, other: &Ival) {
        if self.err.total {
            self.lo.assign_endpoint(&other.lo);
            self.hi.assign_endpoint(&other.hi);
            self.err = other.err;
            self.err.partial = true;
            return;
        }

        if other.err.total {
            self.err.partial = true;
            return;
        }

        self.lo.min_with(&other.lo);
        self.hi.max_with(&other.hi);
        self.err = self.err.union_disjoint(&other.err);
    }

    /// Return Some(false) if interval is exactly [0,0], Some(true) if [1,1], else None.
    /// Returns None whenever there are error flags present.
    pub(crate) fn known_bool(&self) -> Option<bool> {
        if self.err.partial || self.err.total {
            return None;
        }
        let lo = self.lo.as_float();
        let hi = self.hi.as_float();
        if lo.is_zero() && hi.is_zero() {
            Some(false)
        } else if *lo == 1 && *hi == 1 {
            Some(true)
        } else {
            None
        }
    }

    pub(crate) fn split_into(&self, val: &Float, lower: &mut Ival, upper: &mut Ival) {
        let prec = self.prec();
        lower.set_prec_raw(prec);
        lower.lo.as_float_mut().assign(self.lo.as_float());
        lower.lo.immovable = self.lo.immovable;
        lower.hi.as_float_mut().assign(val);
        lower.hi.immovable = self.hi.immovable;
        lower.err = self.err;
        upper.set_prec_raw(prec);
        upper.lo.as_float_mut().assign(val);
        upper.lo.immovable = self.lo.immovable;
        upper.hi.as_float_mut().assign(self.hi.as_float());
        upper.hi.immovable = self.hi.immovable;
        upper.err = self.err;
    }

    /// Split an interval at a point, returning the two halves of that
    /// interval on either side of the split point.
    pub fn split_at(&self, val: &Float) -> (Ival, Ival) {
        let lower = Ival::new(
            self.lo.clone(),
            Endpoint::new(OrdFloat::from(val.clone()), self.hi.immovable),
            self.err,
        );
        let upper = Ival::new(
            Endpoint::new(OrdFloat::from(val.clone()), self.lo.immovable),
            self.hi.clone(),
            self.err,
        );
        (lower, upper)
    }
}

impl ErrorFlags {
    pub fn new(partial: bool, total: bool) -> Self {
        ErrorFlags { partial, total }
    }

    pub fn none() -> Self {
        ErrorFlags::new(false, false)
    }

    pub fn error() -> Self {
        ErrorFlags::new(true, true)
    }

    #[inline]
    pub fn partial(&self) -> bool {
        self.partial
    }

    #[inline]
    pub fn total(&self) -> bool {
        self.total
    }

    pub(crate) fn union(&self, other: &ErrorFlags) -> ErrorFlags {
        ErrorFlags::new(self.partial || other.partial, self.total || other.total)
    }

    pub(crate) fn union_disjoint(&self, other: &ErrorFlags) -> ErrorFlags {
        ErrorFlags::new(self.partial || other.partial, self.total && other.total)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IvalClass {
    Pos = 1,
    Neg = -1,
    Mix = 0,
}

pub(crate) fn classify(ival: &Ival, strict: bool) -> IvalClass {
    let lo = ival.lo.as_float();
    let hi = ival.hi.as_float();
    if strict {
        if *lo > 0.0 {
            IvalClass::Pos
        } else if *hi < 0.0 {
            IvalClass::Neg
        } else {
            IvalClass::Mix
        }
    } else if *lo >= 0.0 {
        IvalClass::Pos
    } else if *hi <= 0.0 {
        IvalClass::Neg
    } else {
        IvalClass::Mix
    }
}
