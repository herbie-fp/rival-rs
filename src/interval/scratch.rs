use super::value::Ival;
use crate::mpfr::set_prec_raw;
use rug::{Assign, Float};
use std::cell::RefCell;

thread_local! {
    static IVALS: RefCell<Vec<Ival>> = const { RefCell::new(Vec::new()) };
    static FLOATS: RefCell<Vec<Float>> = const { RefCell::new(Vec::new()) };
}

fn take_ival(prec: u32) -> Ival {
    match IVALS.with(|pool| pool.borrow_mut().pop()) {
        Some(mut ival) => {
            ival.reset_zero(prec);
            ival
        }
        None => Ival::zero(prec),
    }
}

fn give_ival(ival: Ival) {
    IVALS.with(|pool| pool.borrow_mut().push(ival));
}

fn take_float(prec: u32) -> Float {
    match FLOATS.with(|pool| pool.borrow_mut().pop()) {
        Some(mut f) => {
            set_prec_raw(&mut f, prec);
            f.assign(0);
            f
        }
        None => Float::with_val(prec, 0),
    }
}

fn give_float(f: Float) {
    FLOATS.with(|pool| pool.borrow_mut().push(f));
}

pub(crate) fn with_ival<R>(prec: u32, f: impl FnOnce(&mut Ival) -> R) -> R {
    let mut a = take_ival(prec);
    let r = f(&mut a);
    give_ival(a);
    r
}

pub(crate) fn with_ival2<R>(prec: u32, f: impl FnOnce(&mut Ival, &mut Ival) -> R) -> R {
    let mut a = take_ival(prec);
    let mut b = take_ival(prec);
    let r = f(&mut a, &mut b);
    give_ival(b);
    give_ival(a);
    r
}

pub(crate) fn with_float<R>(prec: u32, f: impl FnOnce(&mut Float) -> R) -> R {
    let mut a = take_float(prec);
    let r = f(&mut a);
    give_float(a);
    r
}

pub(crate) fn with_float2<R>(prec: u32, f: impl FnOnce(&mut Float, &mut Float) -> R) -> R {
    let mut a = take_float(prec);
    let mut b = take_float(prec);
    let r = f(&mut a, &mut b);
    give_float(b);
    give_float(a);
    r
}

pub(crate) fn with_float4<R>(
    prec: u32,
    f: impl FnOnce(&mut Float, &mut Float, &mut Float, &mut Float) -> R,
) -> R {
    let mut a = take_float(prec);
    let mut b = take_float(prec);
    let mut c = take_float(prec);
    let mut d = take_float(prec);
    let r = f(&mut a, &mut b, &mut c, &mut d);
    give_float(d);
    give_float(c);
    give_float(b);
    give_float(a);
    r
}
