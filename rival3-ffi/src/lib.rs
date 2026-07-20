use std::os::raw::c_char;

pub mod discretization;
pub mod expr;
pub mod hints;
pub mod machine;
pub mod profile;

pub use discretization::{RivalDiscType, RivalDiscretization};
pub use expr::{RIVAL_EXPR_INVALID, RivalExprArena};
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

pub const RIVAL_ABI_VERSION: u32 = 2;

#[unsafe(no_mangle)]
pub extern "C" fn rival_version() -> u32 {
    RIVAL_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn rival_error_message(error: RivalError) -> *const c_char {
    match error {
        RivalError::Ok => c"Success".as_ptr(),
        RivalError::InvalidInput => c"Invalid input".as_ptr(),
        RivalError::Unsamplable => c"Unsamplable input".as_ptr(),
    }
}
