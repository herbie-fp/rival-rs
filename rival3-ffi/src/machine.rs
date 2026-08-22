use crate::RivalError;
use crate::discretization::RivalDiscretization;
use crate::expr::RivalExprBuilder;
use crate::hints::RivalHints;
use crate::profile::{ProfileCache, RivalExecution, RivalProfileSummary};
use gmp_mpfr_sys::mpfr::{self, mpfr_t};
use rival::{
    ErrorFlags, Hint, Ival, Machine, MachineBuilder, OutputPolicy, RivalError as CoreError,
};
use rug::Float;
use std::ptr;
use std::slice;

pub struct RivalMachine {
    pub(crate) machine: Machine<RivalDiscretization>,
    pub(crate) arg_buf: Vec<Ival>,
    pub(crate) rect_buf: Vec<Ival>,
    pub(crate) profile_cache: ProfileCache,
    pub(crate) instruction_names_cache: Vec<u8>,
    pub(crate) n_vars: usize,
    pub(crate) n_exprs: usize,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RivalProfilingMode {
    Off = 0,
    On = 1,
}

impl RivalProfilingMode {
    fn from_abi(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Off,
            1 => Self::On,
            _ => return None,
        })
    }
}

#[repr(C)]
pub struct RivalAnalyzeResult {
    pub error: RivalError,
    pub is_error: bool,
    pub maybe_error: bool,
    pub converged: bool,
    pub hints: *mut RivalHints,
}

#[inline]
fn invalid_analyze_result() -> RivalAnalyzeResult {
    RivalAnalyzeResult {
        error: RivalError::InvalidInput,
        is_error: true,
        maybe_error: true,
        converged: false,
        hints: ptr::null_mut(),
    }
}

#[inline]
fn output_policy(require_all_outputs: bool) -> OutputPolicy {
    if require_all_outputs {
        OutputPolicy::RequireAll
    } else {
        OutputPolicy::AllowPartial
    }
}

#[inline]
unsafe fn extract_hints<'a>(hints: *const RivalHints) -> Option<&'a [Hint]> {
    if hints.is_null() {
        None
    } else {
        Some(unsafe { (*hints).hints.as_slice() })
    }
}

#[inline]
unsafe fn marshal_point_args(
    args: *const *const mpfr_t,
    n_args: usize,
    buf: &mut [Ival],
) -> Result<(), RivalError> {
    let arg_ptrs = unsafe { slice::from_raw_parts(args, n_args) };

    for &ptr in arg_ptrs.iter() {
        if ptr.is_null() {
            return Err(RivalError::InvalidInput);
        }
    }

    for (i, &ptr) in arg_ptrs.iter().enumerate() {
        let ival = &mut buf[i];
        let src_prec = unsafe { mpfr::get_prec(ptr) };
        ival.lo_mut().set_prec(src_prec as u32);
        ival.hi_mut().set_prec(src_prec as u32);
        unsafe {
            mpfr::set(ival.lo_mut().as_raw_mut(), ptr, mpfr::rnd_t::RNDN);
            mpfr::set(ival.hi_mut().as_raw_mut(), ptr, mpfr::rnd_t::RNDN);
        }
        ival.set_immovable(true, true);
        let err = if ival.lo().is_finite() {
            ErrorFlags::none()
        } else {
            ErrorFlags::error()
        };
        ival.set_error_flags(err);
    }

    Ok(())
}

#[inline]
unsafe fn marshal_rect_args(
    rect: *const *const mpfr_t,
    n_args: usize,
    buf: &mut [Ival],
) -> Result<(), RivalError> {
    let rect_ptrs = unsafe { slice::from_raw_parts(rect, n_args * 2) };

    for &ptr in rect_ptrs.iter() {
        if ptr.is_null() {
            return Err(RivalError::InvalidInput);
        }
    }

    for i in 0..n_args {
        let lo_ptr = rect_ptrs[2 * i];
        let hi_ptr = rect_ptrs[2 * i + 1];

        let lo_prec = unsafe { mpfr::get_prec(lo_ptr) } as u32;
        let hi_prec = unsafe { mpfr::get_prec(hi_ptr) } as u32;
        let prec = lo_prec.max(hi_prec);

        let ival = &mut buf[i];
        ival.lo_mut().set_prec(prec);
        ival.hi_mut().set_prec(prec);

        unsafe {
            mpfr::set(ival.lo_mut().as_raw_mut(), lo_ptr, mpfr::rnd_t::RNDN);
            mpfr::set(ival.hi_mut().as_raw_mut(), hi_ptr, mpfr::rnd_t::RNDN);
        }

        let fixed = { ival.lo() == ival.hi() };
        let err = {
            let lo = ival.lo();
            let hi = ival.hi();
            lo.is_nan() || hi.is_nan() || (fixed && lo.is_infinite())
        };
        ival.set_immovable(fixed, fixed);
        let flags = if err {
            ErrorFlags::error()
        } else {
            ErrorFlags::none()
        };
        ival.set_error_flags(flags);
    }

    Ok(())
}

#[inline]
unsafe fn write_outputs(
    outputs: &[Ival],
    out: *const *mut mpfr_t,
    n_out: usize,
) -> Result<(), RivalError> {
    if outputs.len() != n_out {
        return Err(RivalError::InvalidInput);
    }
    let out_ptrs = unsafe { slice::from_raw_parts(out, n_out) };
    for &out_ptr in out_ptrs.iter() {
        if out_ptr.is_null() {
            return Err(RivalError::InvalidInput);
        }
    }
    for (i, val) in outputs.iter().enumerate() {
        if val.error_flags().total() {
            unsafe { mpfr::set_nan(out_ptrs[i]) };
        } else {
            unsafe { mpfr::set(out_ptrs[i], val.lo().as_raw(), mpfr::rnd_t::RNDN) };
        }
    }
    Ok(())
}

#[inline]
unsafe fn apply_inner(
    wrapper: &mut RivalMachine,
    args: *const *const mpfr_t,
    n_args: usize,
    out: *const *mut mpfr_t,
    n_out: usize,
    hints: *const RivalHints,
    max_precision: u32,
    // Selects adaptive (`Some`) or baseline (`None`) evaluation.
    max_iterations: Option<usize>,
    require_all_outputs: bool,
) -> RivalError {
    if n_args != wrapper.n_vars || n_out != wrapper.n_exprs {
        return RivalError::InvalidInput;
    }

    if !hints.is_null() && unsafe { (*hints).hints.len() } != wrapper.machine.instruction_count() {
        return RivalError::InvalidInput;
    }

    if n_args > 0 {
        if let Err(e) = unsafe { marshal_point_args(args, n_args, &mut wrapper.arg_buf) } {
            return e;
        }
    }

    wrapper.machine.set_max_precision(max_precision);
    let hints_opt = unsafe { extract_hints(hints) };

    let policy = output_policy(require_all_outputs);
    let result = match max_iterations {
        Some(iters) => wrapper
            .machine
            .apply(&wrapper.arg_buf, hints_opt, iters, policy),
        None => wrapper
            .machine
            .apply_baseline(&wrapper.arg_buf, hints_opt, policy),
    };

    match result {
        Ok(outputs) => match unsafe { write_outputs(&outputs, out, n_out) } {
            Ok(()) => RivalError::Ok,
            Err(e) => e,
        },
        Err(CoreError::InvalidInput) => RivalError::InvalidInput,
        Err(CoreError::Unsamplable) => RivalError::Unsamplable,
    }
}

#[inline]
unsafe fn analyze_inner(
    wrapper: &mut RivalMachine,
    rect: *const *const mpfr_t,
    n_args: usize,
    hints: *const RivalHints,
    baseline: bool,
    require_all_outputs: bool,
) -> RivalAnalyzeResult {
    if n_args != wrapper.n_vars {
        return invalid_analyze_result();
    }

    if !hints.is_null() && unsafe { (*hints).hints.len() } != wrapper.machine.instruction_count() {
        return invalid_analyze_result();
    }

    if unsafe { marshal_rect_args(rect, n_args, &mut wrapper.rect_buf) }.is_err() {
        return invalid_analyze_result();
    }

    let hints_opt = unsafe { extract_hints(hints) };

    let policy = output_policy(require_all_outputs);
    let (status, next_hints, converged) = if baseline {
        wrapper
            .machine
            .analyze_baseline_with_hints(&wrapper.rect_buf, hints_opt, policy)
    } else {
        wrapper
            .machine
            .analyze_with_hints(&wrapper.rect_buf, hints_opt, policy)
    };

    RivalAnalyzeResult {
        error: RivalError::Ok,
        is_error: !status.lo().is_zero(),
        maybe_error: !status.hi().is_zero(),
        converged,
        hints: Box::into_raw(Box::new(RivalHints { hints: next_hints })),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_configure_baseline(machine: *mut RivalMachine) -> bool {
    if machine.is_null() {
        return false;
    }

    let wrapper = unsafe { &mut *machine };
    wrapper.machine.configure_baseline();
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_new(
    builder: *const RivalExprBuilder,
    expr_handles: *const u32,
    n_exprs: usize,
    disc: *const RivalDiscretization,
    max_precision: u32,
    profile_capacity: usize,
) -> *mut RivalMachine {
    if builder.is_null() || disc.is_null() || (expr_handles.is_null() && n_exprs > 0) {
        return ptr::null_mut();
    }

    unsafe {
        let builder = &*builder;
        let handles = if n_exprs > 0 {
            slice::from_raw_parts(expr_handles, n_exprs)
        } else {
            &[]
        };
        let outputs = match builder.outputs(handles) {
            Some(outputs) => outputs,
            None => return ptr::null_mut(),
        };
        let n_vars = builder.expressions.argument_count();
        let disc_cloned = (*disc).clone();
        let machine = MachineBuilder::new(disc_cloned)
            .max_precision(max_precision)
            .profile_capacity(profile_capacity)
            .build(&builder.expressions, &outputs);

        let arg_prec = machine.argument_precision();
        let arg_buf: Vec<Ival> = (0..n_vars).map(|_| Ival::zero(arg_prec)).collect();
        let rect_buf: Vec<Ival> = (0..n_vars)
            .map(|_| Ival::from_lo_hi(Float::with_val(arg_prec, 0), Float::with_val(arg_prec, 0)))
            .collect();

        Box::into_raw(Box::new(RivalMachine {
            machine,
            arg_buf,
            rect_buf,
            profile_cache: ProfileCache::new(),
            instruction_names_cache: Vec::new(),
            n_vars,
            n_exprs,
        }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_free(machine: *mut RivalMachine) {
    if !machine.is_null() {
        unsafe { drop(Box::from_raw(machine)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_instruction_count(machine: *const RivalMachine) -> usize {
    if machine.is_null() {
        0
    } else {
        unsafe { (*machine).machine.instruction_count() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_var_count(machine: *const RivalMachine) -> usize {
    if machine.is_null() {
        0
    } else {
        unsafe { (*machine).n_vars }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_expr_count(machine: *const RivalMachine) -> usize {
    if machine.is_null() {
        0
    } else {
        unsafe { (*machine).n_exprs }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_apply(
    machine: *mut RivalMachine,
    args: *const *const mpfr_t,
    n_args: usize,
    out: *const *mut mpfr_t,
    n_out: usize,
    hints: *const RivalHints,
    max_iterations: usize,
    max_precision: u32,
    require_all_outputs: bool,
) -> RivalError {
    if machine.is_null() || out.is_null() || (args.is_null() && n_args > 0) {
        return RivalError::InvalidInput;
    }

    let wrapper = unsafe { &mut *machine };
    unsafe {
        apply_inner(
            wrapper,
            args,
            n_args,
            out,
            n_out,
            hints,
            max_precision,
            Some(max_iterations),
            require_all_outputs,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_apply_baseline(
    machine: *mut RivalMachine,
    args: *const *const mpfr_t,
    n_args: usize,
    out: *const *mut mpfr_t,
    n_out: usize,
    hints: *const RivalHints,
    max_precision: u32,
    require_all_outputs: bool,
) -> RivalError {
    if machine.is_null() || out.is_null() || (args.is_null() && n_args > 0) {
        return RivalError::InvalidInput;
    }

    let wrapper = unsafe { &mut *machine };
    unsafe {
        apply_inner(
            wrapper,
            args,
            n_args,
            out,
            n_out,
            hints,
            max_precision,
            None,
            require_all_outputs,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_analyze_with_hints(
    machine: *mut RivalMachine,
    rect: *const *const mpfr_t,
    n_args: usize,
    hints: *const RivalHints,
    require_all_outputs: bool,
) -> RivalAnalyzeResult {
    if machine.is_null() || rect.is_null() {
        return invalid_analyze_result();
    }

    let wrapper = unsafe { &mut *machine };
    unsafe { analyze_inner(wrapper, rect, n_args, hints, false, require_all_outputs) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_analyze_baseline_with_hints(
    machine: *mut RivalMachine,
    rect: *const *const mpfr_t,
    n_args: usize,
    hints: *const RivalHints,
    require_all_outputs: bool,
) -> RivalAnalyzeResult {
    if machine.is_null() || rect.is_null() {
        return invalid_analyze_result();
    }

    let wrapper = unsafe { &mut *machine };
    unsafe { analyze_inner(wrapper, rect, n_args, hints, true, require_all_outputs) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_profiler_count(machine: *const RivalMachine) -> usize {
    if machine.is_null() {
        0
    } else {
        unsafe { (*machine).machine.execution_records().len() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_profiler_get(
    machine: *const RivalMachine,
    idx: usize,
    out: *mut RivalExecution,
) -> bool {
    if machine.is_null() || out.is_null() {
        return false;
    }
    let records = unsafe { (*machine).machine.execution_records() };
    if idx >= records.len() {
        return false;
    }
    let exec = &records[idx];
    unsafe {
        *out = RivalExecution {
            instruction_idx: exec.number,
            precision: exec.precision,
            time_ms: exec.time_ms,
            iteration: exec.iteration as u32,
        };
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_profiler_reset(machine: *mut RivalMachine) {
    if !machine.is_null() {
        unsafe { (*machine).machine.clear_executions() };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_profiler_aggregate(
    machine: *mut RivalMachine,
    bucket_size: u32,
) -> RivalProfileSummary {
    if machine.is_null() {
        return RivalProfileSummary {
            entries: ptr::null(),
            len: 0,
            bumps: 0,
            iterations: 0,
        };
    }

    let wrapper = unsafe { &mut *machine };
    let bumps = wrapper.machine.bumps();
    let iterations = wrapper.machine.iterations();
    let records = wrapper.machine.execution_records();

    if records.is_empty() {
        wrapper.profile_cache.summary_from_cache()
    } else {
        let summary =
            wrapper
                .profile_cache
                .aggregate_from(records.iter(), bucket_size, bumps, iterations);
        wrapper.machine.clear_executions();
        summary
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_profiler_executions(
    machine: *mut RivalMachine,
    out_len: *mut usize,
) -> *const RivalExecution {
    if machine.is_null() || out_len.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0 };
        }
        return ptr::null();
    }

    let wrapper = unsafe { &mut *machine };
    let records = wrapper.machine.execution_records();

    if !records.is_empty() {
        wrapper.profile_cache.cache_executions(records.iter());
        wrapper.machine.clear_executions();
    } else {
        wrapper.profile_cache.executions.clear();
    }

    unsafe { *out_len = wrapper.profile_cache.executions.len() };
    if wrapper.profile_cache.executions.is_empty() {
        ptr::null()
    } else {
        wrapper.profile_cache.executions.as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_instruction_names(
    machine: *mut RivalMachine,
    out_len: *mut usize,
) -> *const u8 {
    if machine.is_null() || out_len.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0 };
        }
        return ptr::null();
    }

    let wrapper = unsafe { &mut *machine };

    if wrapper.instruction_names_cache.is_empty() {
        let names: Vec<&str> = wrapper.machine.instruction_names();
        wrapper.instruction_names_cache = names.join("\0").into_bytes();
    }

    unsafe { *out_len = wrapper.instruction_names_cache.len() };
    wrapper.instruction_names_cache.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_iterations(machine: *const RivalMachine) -> u32 {
    if machine.is_null() {
        0
    } else {
        unsafe { (*machine).machine.iterations() as u32 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_bumps(machine: *const RivalMachine) -> u32 {
    if machine.is_null() {
        0
    } else {
        unsafe { (*machine).machine.bumps() as u32 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_set_profiling(machine: *mut RivalMachine, mode: u32) {
    let Some(mode) = RivalProfilingMode::from_abi(mode) else {
        return;
    };
    if !machine.is_null() {
        let wrapper = unsafe { &mut *machine };
        wrapper
            .machine
            .set_profiling_enabled(mode == RivalProfilingMode::On);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rival_machine_get_profiling(
    machine: *const RivalMachine,
) -> RivalProfilingMode {
    if machine.is_null() {
        RivalProfilingMode::Off
    } else {
        let wrapper = unsafe { &*machine };
        if wrapper.machine.profiling_enabled() {
            RivalProfilingMode::On
        } else {
            RivalProfilingMode::Off
        }
    }
}
