//! Register machine evaluator.

use super::{
    builder::{Expression, ExpressionBuilder},
    execute,
    instructions::Instruction,
};
use crate::{
    eval::profile::{Execution, Profiler},
    interval::Ival,
};
use rug::Float;

/// A discretization represents some subset of the real numbers
/// (for example, `f64`).
pub trait Discretization: Clone {
    /// The precision in bits needed to exactly represent a value in this subset.
    fn target(&self) -> u32;
    /// Convert a bigfloat value to a value in this subset.
    fn convert(&self, idx: usize, v: &Float) -> Float;
    /// Determine how close two values in the subset are.
    ///
    /// A distance of `0` indicates that the two values are equal.
    /// A distance of `2` or greater indicates that the two values are far apart.
    /// A value of exactly `1` indicates that the two values are sequential,
    /// that is, that they share a rounding boundary.
    /// This last case triggers special behavior inside Rival
    /// to handle double-rounding issues.
    fn distance(&self, idx: usize, lo: &Float, hi: &Float) -> usize;
    /// Convert both endpoints into this subset and measure their distance.
    ///
    /// The default implementation combines [`convert`](Discretization::convert)
    /// and [`distance`](Discretization::distance); implementations may
    /// override it to avoid intermediate allocations.
    fn converted_distance(&self, idx: usize, lo: &Float, hi: &Float) -> usize {
        let lo = self.convert(idx, lo);
        let hi = self.convert(idx, hi);
        self.distance(idx, &lo, &hi)
    }
}

/// Interval evaluation machine with persistent state and discretization.
///
/// A machine is compiled from an [`ExpressionBuilder`] via
/// [`MachineBuilder::build`], and can then be evaluated at specific input
/// points using [`Machine::apply`].
///
/// Internally, a machine is a simple register machine. Compilation is fairly
/// slow, so the ideal use case is to compile a function once and then apply it
/// to multiple points.
///
/// Common subexpressions are identified and eliminated as expressions are
/// built, so a shared expression graph stays compact. This makes Rival ideal
/// for evaluating large families of related expressions, a feature that is
/// heavily used in [Herbie](https://herbie.uwplse.org). Note that each output
/// can use a different discretization.
pub struct Machine<D: Discretization> {
    pub(crate) disc: D,

    // Program structure.
    pub(crate) arguments: Vec<String>,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) outputs: Vec<usize>,

    // Initial state computed during compilation.
    pub(crate) initial_repeats: Vec<bool>,
    pub(crate) initial_precisions: Vec<u32>,
    pub(crate) best_known_precisions: Vec<u32>,
    pub(crate) default_hint: Vec<Hint>,

    // Runtime state.
    pub(crate) registers: Vec<Ival>,
    pub(crate) precisions: Vec<u32>,
    pub(crate) repeats: Vec<bool>, // true = skip execution (no change needed)
    pub(crate) output_distance: Vec<bool>, // true = output near discretization boundary

    pub(crate) iteration: usize,
    pub(crate) bumps: usize, // Number of times bumps mode has been activated

    pub(crate) scratch_precs_max: Vec<u32>,
    pub(crate) scratch_precs_min: Vec<u32>,
    pub(crate) scratch_repeats: Vec<bool>,

    // Profiling.
    pub(crate) profiler: Profiler,
    pub(crate) profiling_enabled: bool,

    // Configuration parameters.
    pub(crate) max_precision: u32,
    pub(crate) min_precision: u32,
    pub(crate) lower_bound_early_stopping: bool,
    pub(crate) slack_unit: i64,
    pub(crate) bumps_activated: bool,
}

/// Hints guide the execution of individual instructions in the machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hint {
    /// Instruction executes normally.
    Execute,
    /// Instruction is not needed for the outputs being computed.
    /// Examples include dead code or an untaken branch.
    Skip,
    /// Skip execution and copy the value from the chosen input position.
    /// For example, `if (true) x else y` becomes an alias to `x`.
    Alias(u8),
    /// Skip execution because the result is already known exactly.
    /// For example, `if (x > 0)` where `x = [5, 10]` is always true.
    KnownBool(bool),
}

#[derive(Clone, Debug)]
pub(crate) struct PathOutcome {
    pub hint: Hint,
    pub converged: bool,
}

/// Builder for constructing a [`Machine`] with custom parameters.
///
/// See the [crate documentation](crate) for an example.
pub struct MachineBuilder<D: Discretization> {
    disc: D,
    min_precision: u32,
    max_precision: u32,
    slack_unit: i64,
    base_tuning_precision: u32,
    ampl_tuning_bits: u32,
    profile_capacity: usize,
    profiling_enabled: bool,
}

impl<D: Discretization> MachineBuilder<D> {
    /// Create a builder with default precision parameters.
    ///
    /// Defaults:
    /// - `min_precision`: 20 bits
    /// - `max_precision`: 10,000 bits
    /// - `slack_unit`: 512
    /// - `profiling`: enabled, buffer capacity 1000
    pub fn new(disc: D) -> Self {
        Self {
            disc,
            min_precision: 20,
            max_precision: 10_000,
            slack_unit: 512,
            base_tuning_precision: 5,
            ampl_tuning_bits: 2,
            profile_capacity: 1000,
            profiling_enabled: true,
        }
    }

    /// Set the minimum working precision in bits.
    pub fn min_precision(mut self, v: u32) -> Self {
        self.min_precision = v;
        self
    }

    /// Set the maximum working precision in bits.
    pub fn max_precision(mut self, v: u32) -> Self {
        self.max_precision = v;
        self
    }

    /// Set the slack unit used when computing slack bits.
    pub fn slack_unit(mut self, v: i64) -> Self {
        self.slack_unit = v;
        self
    }

    /// Set the base tuning precision added to discretization targets.
    pub fn base_tuning_precision(mut self, v: u32) -> Self {
        self.base_tuning_precision = v;
        self
    }

    /// Set the amplification tuning bits added during propagation.
    pub fn ampl_tuning_bits(mut self, v: u32) -> Self {
        self.ampl_tuning_bits = v;
        self
    }

    /// Enable or disable per-instruction profiling (enabled by default).
    pub fn enable_profiling(mut self, enabled: bool) -> Self {
        self.profiling_enabled = enabled;
        self
    }

    /// Set profiling buffer capacity (default 1000 records).
    pub fn profile_capacity(mut self, cap: usize) -> Self {
        self.profile_capacity = cap;
        self
    }

    /// Compile the expressions selected by `outputs` into a machine.
    ///
    /// Every output must have been created by `expressions`.
    ///
    /// Returns a [`Machine`], an opaque type that can be passed to
    /// [`Machine::apply`] to evaluate the compiled real expressions
    /// on a specific point.
    pub fn build(self, expressions: &ExpressionBuilder, outputs: &[Expression]) -> Machine<D> {
        let program = expressions.finish(outputs);
        let var_count = program.arguments.len();
        let instruction_count = program.instructions.len();
        let register_count = var_count + instruction_count;

        let mut registers = vec![Ival::zero(self.max_precision); register_count];
        let instructions = program.instructions;

        let mut best_known_precisions = vec![0u32; instruction_count];
        let initial_precisions = make_initial_precisions(
            &instructions,
            var_count,
            &program.outputs,
            &self.disc,
            self.base_tuning_precision,
            self.ampl_tuning_bits,
        );

        let initial_repeats = make_initial_repeats(
            &instructions,
            var_count,
            &mut registers,
            &initial_precisions,
            &mut best_known_precisions,
        );

        let default_hint = vec![Hint::Execute; instruction_count];
        let precisions = vec![0u32; instruction_count];
        let repeats = vec![false; instruction_count];
        let output_distance = vec![false; program.outputs.len()];

        Machine {
            disc: self.disc,
            arguments: program.arguments,
            instructions,
            outputs: program.outputs,
            initial_repeats,
            initial_precisions,
            best_known_precisions,
            default_hint,
            registers,
            precisions,
            repeats,
            output_distance,
            iteration: 0,
            bumps: 0,
            scratch_precs_max: vec![0u32; instruction_count],
            scratch_precs_min: vec![0u32; instruction_count],
            scratch_repeats: vec![true; instruction_count],
            max_precision: self.max_precision,
            min_precision: self.min_precision,
            lower_bound_early_stopping: false,
            slack_unit: self.slack_unit,
            bumps_activated: false,
            profiler: Profiler::with_capacity(self.profile_capacity),
            profiling_enabled: self.profiling_enabled,
        }
    }
}

impl<D: Discretization> Machine<D> {
    /// Return the instruction index that writes to the given register when applicable.
    #[inline]
    pub(crate) fn register_to_instruction(&self, register: usize) -> Option<usize> {
        let var_count = self.arguments.len();
        if register >= var_count {
            Some(register - var_count)
        } else {
            None
        }
    }

    /// Return the register index corresponding to an instruction index.
    #[inline]
    pub(crate) fn instruction_register(&self, index: usize) -> usize {
        self.arguments.len() + index
    }

    /// Return the total number of instructions in the compiled machine.
    #[inline]
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Return the number of input arguments expected by this machine.
    #[inline]
    pub fn argument_count(&self) -> usize {
        self.arguments.len()
    }

    /// Return the discretization's target precision in bits.
    #[inline]
    pub fn target_precision(&self) -> u32 {
        self.disc.target()
    }

    /// Return the minimum working precision in bits.
    #[inline]
    pub fn min_precision(&self) -> u32 {
        self.min_precision
    }

    /// Return the maximum working precision in bits.
    #[inline]
    pub fn max_precision(&self) -> u32 {
        self.max_precision
    }

    /// Return the precision used for input arguments.
    #[inline]
    pub fn argument_precision(&self) -> u32 {
        self.disc.target().max(self.min_precision)
    }

    /// Set the maximum working precision in bits.
    #[inline]
    pub fn set_max_precision(&mut self, bits: u32) {
        self.max_precision = bits;
    }

    /// Return the number of iterations needed for the most recent call to [`Machine::apply`].
    #[inline]
    pub fn iterations(&self) -> usize {
        self.iteration
    }

    /// Return the number of bumps detected during the most recent call to [`Machine::apply`].
    #[inline]
    pub fn bumps(&self) -> usize {
        self.bumps
    }

    /// Enable or disable per-instruction profiling.
    #[inline]
    pub fn set_profiling_enabled(&mut self, enabled: bool) {
        self.profiling_enabled = enabled;
    }

    /// Returns whether per-instruction profiling is enabled.
    #[inline]
    pub fn profiling_enabled(&self) -> bool {
        self.profiling_enabled
    }

    /// Return a slice of recorded [`Execution`] records from the profiling buffer.
    #[inline]
    pub fn execution_records(&self) -> &[Execution] {
        self.profiler.records()
    }

    /// Reset the profiling buffer, discarding all recorded executions.
    #[inline]
    pub fn clear_executions(&mut self) {
        self.profiler.reset();
    }

    /// Return the operation name for each instruction in the machine.
    pub fn instruction_names(&self) -> Vec<&'static str> {
        self.instructions
            .iter()
            .map(|instr| instr.data.name_static())
            .collect()
    }

    /// Reconfigure the machine to use the baseline strategy.
    ///
    /// The baseline strategy uses a single global precision for all
    /// instructions, doubling it each iteration. This is simpler but
    /// less efficient than the default adaptive precision tuning.
    pub fn configure_baseline(&mut self) {
        let var_count = self.arguments.len();
        let start_prec = self.disc.target().saturating_add(10);

        self.initial_precisions.fill(start_prec);
        self.best_known_precisions.fill(0);

        self.initial_repeats = make_initial_repeats(
            &self.instructions,
            var_count,
            &mut self.registers,
            &self.initial_precisions,
            &mut self.best_known_precisions,
        );
    }

    /// Return a snapshot of recorded [`Execution`] records and reset
    /// the internal buffer pointer.
    pub fn take_executions(&mut self) -> Vec<Execution> {
        let slice = self.execution_records().to_vec();
        self.clear_executions();
        slice
    }
}

impl PathOutcome {
    /// Create an execute outcome with the given convergence status.
    #[inline]
    pub(crate) fn execute(converged: bool) -> PathOutcome {
        PathOutcome {
            hint: Hint::Execute,
            converged,
        }
    }

    /// Create an alias outcome for the provided input position.
    #[inline]
    pub(crate) fn alias(idx: u8) -> PathOutcome {
        PathOutcome {
            hint: Hint::Alias(idx),
            converged: true,
        }
    }

    /// Create a known boolean outcome pinned to the provided value.
    #[inline]
    pub(crate) fn known_bool(value: bool) -> PathOutcome {
        PathOutcome {
            hint: Hint::KnownBool(value),
            converged: true,
        }
    }
}

/// Determine initial precision targets for each instruction.
fn make_initial_precisions<D: Discretization>(
    instructions: &[Instruction],
    var_count: usize,
    roots: &[usize],
    disc: &D,
    base_tuning_precision: u32,
    ampl_tuning_bits: u32,
) -> Vec<u32> {
    let mut precisions = vec![0u32; instructions.len()];

    // Initialize output nodes to target + base precision.
    for &root in roots.iter() {
        if root >= var_count {
            precisions[root - var_count] = disc.target() + base_tuning_precision;
        }
    }

    // Propagate precisions backward through the computation graph.
    for idx in (0..instructions.len()).rev() {
        let current_prec = precisions[idx];
        instructions[idx].for_each_input(|reg| {
            if reg >= var_count {
                let input_idx = reg - var_count;
                precisions[input_idx] = precisions[input_idx].max(current_prec + ampl_tuning_bits);
            }
        });
    }

    precisions
}

/// Evaluate and mark constant-only nodes that can skip future execution.
fn make_initial_repeats(
    instructions: &[Instruction],
    var_count: usize,
    registers: &mut [Ival],
    initial_precisions: &[u32],
    best_known_precisions: &mut [u32],
) -> Vec<bool> {
    let mut initial_repeats = vec![true; instructions.len()];

    for (idx, (instr, &prec)) in instructions.iter().zip(initial_precisions).enumerate() {
        let mut depends = false;
        instr.data.for_each_input(|reg| {
            let child = reg as isize - var_count as isize;
            if child < 0 || !initial_repeats[child as usize] {
                depends = true;
            }
        });

        if depends {
            initial_repeats[idx] = false;
        } else {
            execute::evaluate_instruction(instr, registers, prec);
            best_known_precisions[idx] = prec;
        }
    }

    initial_repeats
}
