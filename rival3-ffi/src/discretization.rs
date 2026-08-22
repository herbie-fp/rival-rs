use rival::Discretization;
use rug::Float;
use std::ptr;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RivalDiscType {
    Bool = 0,
    F32 = 1,
    F64 = 2,
}

impl RivalDiscType {
    fn from_abi(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Bool,
            1 => Self::F32,
            2 => Self::F64,
            _ => return None,
        })
    }
}

#[derive(Clone)]
pub struct RivalDiscretization {
    precision: u32,
    types: Vec<RivalDiscType>,
}

impl Discretization for RivalDiscretization {
    #[inline]
    fn target(&self) -> u32 {
        self.precision
    }

    #[inline]
    fn convert(&self, idx: usize, v: &Float) -> Float {
        let disc_type = self.types.get(idx).copied().unwrap_or(RivalDiscType::F64);
        match disc_type {
            RivalDiscType::F64 => {
                let f64_val = v.to_f64();
                Float::with_val(53, f64_val)
            }
            RivalDiscType::F32 => {
                let f32_val = v.to_f32();
                Float::with_val(24, f32_val)
            }
            RivalDiscType::Bool => v.clone(),
        }
    }

    #[inline]
    fn distance(&self, idx: usize, lo: &Float, hi: &Float) -> usize {
        let disc_type = self.types.get(idx).copied().unwrap_or(RivalDiscType::F64);

        match disc_type {
            RivalDiscType::Bool => {
                if lo == hi {
                    0
                } else {
                    2
                }
            }
            RivalDiscType::F32 => ordinal_distance_f32(lo.to_f32(), hi.to_f32()),
            RivalDiscType::F64 => ordinal_distance_f64(lo.to_f64(), hi.to_f64()),
        }
    }
}

#[inline]
fn ordinal_distance_f32(x: f32, y: f32) -> usize {
    if x == y {
        return 0;
    }
    let to_ordinal = |v: f32| -> i32 {
        if v == 0.0 {
            return 0;
        }
        let bits = v.to_bits() as i32;
        if bits < 0 { !bits } else { bits }
    };
    to_ordinal(y).wrapping_sub(to_ordinal(x)).unsigned_abs() as usize
}

#[inline]
fn ordinal_distance_f64(x: f64, y: f64) -> usize {
    if x == y {
        return 0;
    }
    let to_ordinal = |v: f64| -> i64 {
        if v == 0.0 {
            return 0;
        }
        let bits = v.to_bits() as i64;
        if bits < 0 { !bits } else { bits }
    };
    to_ordinal(y).wrapping_sub(to_ordinal(x)).unsigned_abs() as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn rival_disc_f64(precision: u32) -> *mut RivalDiscretization {
    Box::into_raw(Box::new(RivalDiscretization {
        precision,
        types: vec![RivalDiscType::F64],
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn rival_disc_f32(precision: u32) -> *mut RivalDiscretization {
    Box::into_raw(Box::new(RivalDiscretization {
        precision,
        types: vec![RivalDiscType::F32],
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn rival_disc_bool() -> *mut RivalDiscretization {
    Box::into_raw(Box::new(RivalDiscretization {
        precision: 53,
        types: vec![RivalDiscType::Bool],
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_disc_mixed(
    types: *const RivalDiscType,
    n_types: usize,
    precision: u32,
) -> *mut RivalDiscretization {
    if types.is_null() || n_types == 0 {
        return ptr::null_mut();
    }
    // Read as the underlying integer: the caller may supply codes this ABI
    // version does not define, and those are not valid `RivalDiscType` values.
    let codes = unsafe { std::slice::from_raw_parts(types.cast::<u32>(), n_types) };
    let Some(types) = codes
        .iter()
        .map(|&code| RivalDiscType::from_abi(code))
        .collect::<Option<Vec<_>>>()
    else {
        return ptr::null_mut();
    };
    Box::into_raw(Box::new(RivalDiscretization { precision, types }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_disc_free(disc: *mut RivalDiscretization) {
    if !disc.is_null() {
        unsafe { drop(Box::from_raw(disc)) };
    }
}
