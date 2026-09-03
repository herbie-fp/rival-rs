use std::os::raw::c_char;

pub mod discretization;
pub mod expr;
pub mod hints;
pub mod machine;
pub mod profile;

pub use discretization::{RivalDiscType, RivalDiscretization};
pub use expr::{
    RIVAL_EXPR_INVALID, RivalBinaryOp, RivalExprBuilder, RivalTernaryOp, RivalUnaryOp,
    RivalUnaryParamOp,
};
pub use hints::RivalHints;
pub use machine::{RivalAnalyzeResult, RivalMachine, RivalProfilingMode};
pub use profile::{RivalAggregatedProfile, RivalExecution, RivalProfileSummary};

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RivalError {
    Ok = 0,
    InvalidInput = -1,
    Unsamplable = -2,
}

impl RivalError {
    fn from_abi(code: i32) -> Option<Self> {
        Some(match code {
            0 => Self::Ok,
            -1 => Self::InvalidInput,
            -2 => Self::Unsamplable,
            _ => return None,
        })
    }
}

pub const RIVAL_ABI_VERSION: u32 = 3;

#[unsafe(no_mangle)]
pub extern "C" fn rival_version() -> u32 {
    RIVAL_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn rival_error_message(error: i32) -> *const c_char {
    match RivalError::from_abi(error) {
        Some(RivalError::Ok) => c"Success".as_ptr(),
        Some(RivalError::InvalidInput) => c"Invalid input".as_ptr(),
        Some(RivalError::Unsamplable) => c"Unsamplable input".as_ptr(),
        None => c"Unknown error".as_ptr(),
    }
}
