//! Main evaluation loop with adaptive precision tuning.

use itertools::{enumerate, izip};

use crate::eval::{
    execute,
    machine::{Discretization, Hint, Machine},
    profile::Execution,
};
use crate::interval::Ival;

/// Selects when an invalid output makes an evaluation invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputPolicy {
    /// Any totally invalid output makes the whole evaluation invalid.
    RequireAll,
    /// Only a totally invalid *set* of outputs makes the evaluation invalid.
    /// Individual invalid outputs are returned in place.
    AllowPartial,
}

/// The output intervals of a successful evaluation, borrowed from the machine.
pub struct Outputs<'a> {
    registers: &'a [Ival],
    outputs: &'a [usize],
}

impl<'a> Outputs<'a> {
    /// Number of outputs.
    #[inline]
    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    /// Whether there are no outputs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    /// The interval computed for output `index`.
    #[inline]
    pub fn get(&self, index: usize) -> &'a Ival {
        &self.registers[self.outputs[index]]
    }

    /// Iterate over the output intervals in order.
    pub fn iter(&self) -> impl Iterator<Item = &'a Ival> + '_ {
        self.outputs.iter().map(move |&root| &self.registers[root])
    }
}

/// The result of analyzing an input rectangle.
pub struct Analysis {
    /// The rectangle definitely produces an error.
    pub is_error: bool,
    /// The rectangle possibly produces an error.
    pub maybe_error: bool,
    /// Hints for subsequent evaluations on points in the rectangle.
    pub hints: Vec<Hint>,
    /// Whether the analysis has converged.
    pub converged: bool,
}

impl<D: Discretization> Machine<D> {
    /// Evaluate the compiled real expressions on an input point
    /// represented as a slice of intervals.
    ///
    /// `args` must be the same length as the `vars` passed to
    /// [`MachineBuilder::build`](super::machine::MachineBuilder::build). The output is a vector of output
    /// values of the same length as the `exprs` passed to
    /// [`MachineBuilder::build`](super::machine::MachineBuilder::build).
    ///
    /// `hint` can be provided from a previous call to
    /// [`Machine::analyze_with_hints`] to speed up evaluation.
    /// Pass `None` for default behavior.
    ///
    /// `max_iterations` sets the maximum number of re-evaluation
    /// iterations before giving up.
    ///
    /// # Errors
    ///
    /// Returns [`RivalError::InvalidInput`] according to the selected
    /// [`OutputPolicy`].
    /// Returns [`RivalError::Unsamplable`] if Rival is unable to
    /// evaluate at least one expression.
    ///
    /// Under [`OutputPolicy::AllowPartial`], totally invalid outputs are
    /// returned in place, while every non-error output must still be
    /// correctly rounded.
    pub fn apply(
        &mut self,
        args: &[Ival],
        hint: Option<&[Hint]>,
        max_iterations: usize,
        policy: OutputPolicy,
    ) -> Result<Vec<Ival>, RivalError> {
        self.apply_borrowed(args, hint, max_iterations, policy)
            .map(|outputs| outputs.iter().cloned().collect())
    }

    /// Evaluate like [`Machine::apply`], but borrow the outputs from the
    /// machine's registers instead of copying them.
    pub fn apply_borrowed(
        &mut self,
        args: &[Ival],
        hint: Option<&[Hint]>,
        max_iterations: usize,
        policy: OutputPolicy,
    ) -> Result<Outputs<'_>, RivalError> {
        self.load_arguments(args);
        self.with_hints(hint, |machine, hints| {
            for iteration in 0..max_iterations {
                if machine.run_iteration(iteration, hints, policy)? {
                    return Ok(());
                }
            }
            Err(RivalError::Unsamplable)
        })?;
        Ok(self.outputs())
    }

    fn outputs(&self) -> Outputs<'_> {
        Outputs {
            registers: &self.registers,
            outputs: &self.outputs,
        }
    }

    fn with_hints<R>(
        &mut self,
        hint: Option<&[Hint]>,
        f: impl FnOnce(&mut Self, &[Hint]) -> R,
    ) -> R {
        match hint {
            Some(hints) => f(self, hints),
            None => {
                let default = std::mem::take(&mut self.default_hint);
                let result = f(self, &default);
                self.default_hint = default;
                result
            }
        }
    }

    /// Evaluate the machine using the baseline strategy.
    ///
    /// The baseline strategy uses a single global precision for all
    /// instructions, doubling it each iteration. This is simpler but
    /// less efficient than [`Machine::apply`], which uses adaptive
    /// per-instruction precision tuning.
    ///
    /// Call [`Machine::configure_baseline`] before using this method
    /// to set up the machine for baseline evaluation.
    pub fn apply_baseline(
        &mut self,
        args: &[Ival],
        hint: Option<&[Hint]>,
        policy: OutputPolicy,
    ) -> Result<Vec<Ival>, RivalError> {
        self.apply_baseline_borrowed(args, hint, policy)
            .map(|outputs| outputs.iter().cloned().collect())
    }

    /// Evaluate like [`Machine::apply_baseline`], but borrow the outputs
    /// from the machine's registers instead of copying them.
    pub fn apply_baseline_borrowed(
        &mut self,
        args: &[Ival],
        hint: Option<&[Hint]>,
        policy: OutputPolicy,
    ) -> Result<Outputs<'_>, RivalError> {
        self.load_arguments(args);
        self.with_hints(hint, |machine, hints| {
            let start_prec = machine.disc.target().saturating_add(10);
            let mut prec = start_prec;
            let mut iter: usize = 0;

            loop {
                machine.iteration = iter;
                machine.baseline_adjust(prec);
                machine.run_with_hint(hints);

                if machine.collect_outputs(policy)? {
                    return Ok(());
                }
                let next = prec.saturating_mul(2);
                if next > machine.max_precision {
                    return Err(RivalError::Unsamplable);
                }
                prec = next;
                iter = iter.saturating_add(1);
            }
        })?;
        Ok(self.outputs())
    }

    /// Analyze an input rectangle using the baseline strategy,
    /// returning status, next hints, and a convergence flag.
    ///
    /// See [`Machine::analyze_with_hints`] for details on the
    /// return values.
    pub fn analyze_baseline_with_hints(
        &mut self,
        rect: &[Ival],
        hint: Option<&[Hint]>,
        policy: OutputPolicy,
    ) -> (Ival, Vec<Hint>, bool) {
        analysis_triple(self.analyze_baseline_hints(rect, hint, policy))
    }

    /// Analyze like [`Machine::analyze_baseline_with_hints`], returning
    /// the status as plain flags.
    pub fn analyze_baseline_hints(
        &mut self,
        rect: &[Ival],
        hint: Option<&[Hint]>,
        policy: OutputPolicy,
    ) -> Analysis {
        self.load_arguments(rect);
        self.with_hints(hint, |machine, hints| {
            machine.iteration = 0;
            machine.baseline_adjust(machine.disc.target().saturating_add(10));
            machine.run_with_hint(hints);
            machine.analysis(hints, policy)
        })
    }

    fn analysis(&mut self, hints: &[Hint], policy: OutputPolicy) -> Analysis {
        let (good, _done, bad, stuck) = self.return_flags(policy);
        let (next_hint, converged) = self.make_hint(hints);
        Analysis {
            is_error: bad || stuck,
            maybe_error: (!good) || stuck,
            hints: next_hint,
            converged,
        }
    }

    /// Analyze a hyper-rectangle using the baseline strategy and
    /// return only the boolean interval status.
    ///
    /// See [`Machine::analyze`] for details on the return value.
    pub fn analyze_baseline(&mut self, rect: &[Ival], policy: OutputPolicy) -> Ival {
        let (status, _hint, _conv) = self.analyze_baseline_with_hints(rect, None, policy);
        status
    }

    /// Run a single iteration with precision tuning and hint-guided evaluation.
    pub(crate) fn run_iteration(
        &mut self,
        iteration: usize,
        hints: &[Hint],
        policy: OutputPolicy,
    ) -> Result<bool, RivalError> {
        assert_eq!(hints.len(), self.instructions.len(), "hint length mismatch");
        self.iteration = iteration;
        if self.adjust(hints) {
            return Err(RivalError::Unsamplable);
        }
        self.run_with_hint(hints);
        self.collect_outputs(policy)
    }

    /// Analyze an input rectangle using adaptive precision tuning.
    ///
    /// Returns a `(status, hints, converged)` tuple:
    ///
    /// - `status` describes failure under the selected output policy.
    ///   A true lower endpoint means definite failure; otherwise, a true
    ///   upper endpoint means possible failure.
    ///
    /// - `hints` is a vector of [`Hint`]s that can be passed to
    ///   subsequent calls to [`Machine::apply`] to skip unnecessary
    ///   computation.
    ///
    /// - `converged` indicates whether the analysis has converged.
    pub fn analyze_with_hints(
        &mut self,
        rect: &[Ival],
        hint: Option<&[Hint]>,
        policy: OutputPolicy,
    ) -> (Ival, Vec<Hint>, bool) {
        analysis_triple(self.analyze_hints(rect, hint, policy))
    }

    /// Analyze like [`Machine::analyze_with_hints`], returning the status
    /// as plain flags.
    pub fn analyze_hints(
        &mut self,
        rect: &[Ival],
        hint: Option<&[Hint]>,
        policy: OutputPolicy,
    ) -> Analysis {
        self.load_arguments(rect);
        self.with_hints(hint, |machine, hints| {
            machine.iteration = 0;
            machine.adjust(hints);
            machine.run_with_hint(hints);
            machine.analysis(hints, policy)
        })
    }

    /// Analyze a hyper-rectangle and return only the boolean interval status.
    ///
    /// Returns the status described by [`Machine::analyze_with_hints`].
    ///
    /// The advantage of `analyze` over `apply` is that it applies to
    /// whole ranges of input points and is much faster.
    pub fn analyze(&mut self, rect: &[Ival], policy: OutputPolicy) -> Ival {
        let (status, _hint, _conv) = self.analyze_with_hints(rect, None, policy);
        status
    }

    /// Load argument intervals into the front of the register file.
    pub(crate) fn load_arguments(&mut self, args: &[Ival]) {
        assert_eq!(args.len(), self.arguments.len(), "Argument count mismatch");
        for (register, arg) in self.registers.iter_mut().zip(args) {
            register.assign_from(arg);
        }
        self.bumps = 0;
        self.bumps_activated = false;
        self.iteration = 0;
        self.precisions.fill(0);
        self.repeats.fill(false);
        self.output_distance.fill(false);
        if self.profiling_enabled {
            self.profiler.reset();
        }
    }

    /// Execute instructions once using the supplied precision and hint plan.
    fn run_with_hint(&mut self, hints: &[Hint]) {
        // On the first iteration use the initial plan; subsequent iterations use tuned state.
        let (precisions, repeats) = if self.iteration == 0 {
            (&self.initial_precisions[..], &self.initial_repeats[..])
        } else {
            (&self.precisions[..], &self.repeats[..])
        };

        for (idx, (instruction, &repeat, &precision, hint)) in
            enumerate(izip!(&self.instructions, repeats, precisions, hints))
        {
            if repeat {
                continue;
            }
            let out_reg = self.instruction_register(idx);

            // Hints can override execution.
            match hint {
                Hint::Skip => {}
                Hint::Execute => {
                    if self.profiling_enabled {
                        let start = std::time::Instant::now();
                        execute::evaluate_instruction(instruction, &mut self.registers, precision);
                        let dt = start.elapsed().as_secs_f64() * 1000.0;
                        let exec = Execution {
                            name: instruction.data.name_static(),
                            number: idx as i32,
                            precision,
                            time_ms: dt,
                            iteration: self.iteration,
                        };
                        self.profiler.record(exec);
                    } else {
                        execute::evaluate_instruction(instruction, &mut self.registers, precision)
                    }
                }
                // Path reduction aliasing the output of an instruction to one of its inputs.
                Hint::Alias(pos) => {
                    if let Some(src_reg) = instruction.data.input_at(*pos as usize)
                        && src_reg != out_reg
                    {
                        let (src, dst) = if src_reg < out_reg {
                            let (left, right) = self.registers.split_at_mut(out_reg);
                            (&left[src_reg], &mut right[0])
                        } else {
                            let (left, right) = self.registers.split_at_mut(src_reg);
                            (&right[0], &mut left[out_reg])
                        };
                        dst.assign_from(src);
                    }
                }
                // Use pre-computed boolean value.
                Hint::KnownBool(value) => {
                    self.registers[out_reg].set_bool(*value, *value);
                }
            }
        }
    }

    fn baseline_adjust(&mut self, new_prec: u32) {
        let instruction_count = self.instructions.len();
        let profiling = self.profiling_enabled;
        let start_time = if profiling {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Baseline uses a single global precision for all instructions.
        self.precisions.fill(new_prec);

        if self.iteration != 0 {
            let var_count = self.arguments.len();

            // Determine which instructions can affect outputs (must be executed).
            let mut useful = vec![false; instruction_count];
            for &root in &self.outputs {
                if let Some(idx) = self.register_to_instruction(root) {
                    useful[idx] = true;
                }
            }

            for idx in (0..instruction_count).rev() {
                if !useful[idx] {
                    continue;
                }
                let out_reg = self.instruction_register(idx);
                let reg = &self.registers[out_reg];
                if reg.lo.immovable && reg.hi.immovable {
                    useful[idx] = false;
                    continue;
                }
                self.instructions[idx].for_each_input(|reg| {
                    if reg >= var_count {
                        useful[reg - var_count] = true;
                    }
                });
            }

            // Set repeats and update constant precisions.
            for idx in 0..instruction_count {
                let is_constant = self.initial_repeats[idx];
                let best_known = self.best_known_precisions[idx];

                let mut inputs_stable = true;
                if is_constant {
                    self.instructions[idx].for_each_input(|reg| {
                        if reg >= var_count && !self.repeats[reg - var_count] {
                            inputs_stable = false;
                        }
                    });
                }

                let no_need_to_reevaluate = is_constant && new_prec <= best_known && inputs_stable;
                let result_is_exact_already = !useful[idx];
                let repeat = result_is_exact_already || no_need_to_reevaluate;

                if is_constant && !repeat {
                    self.best_known_precisions[idx] = new_prec;
                }
                self.repeats[idx] = repeat;
            }
        }

        if profiling && let Some(t0) = start_time {
            let dt_ms = t0.elapsed().as_secs_f64() * 1000.0;
            self.profiler.record(Execution {
                name: "adjust",
                number: -1,
                precision: (self.iteration as u32) * 1000,
                time_ms: dt_ms,
                iteration: self.iteration,
            });
        }
    }

    /// Translate evaluation state into convergence results.
    fn collect_outputs(&mut self, policy: OutputPolicy) -> Result<bool, RivalError> {
        let (good, done, bad, stuck) = self.return_flags(policy);

        if bad {
            return Err(RivalError::InvalidInput);
        }
        if done && good {
            return Ok(true);
        }
        if stuck {
            return Err(RivalError::Unsamplable);
        }

        Ok(false)
    }

    /// Compute (good, done, bad, stuck) flags and update output_distance.
    fn return_flags(&mut self, policy: OutputPolicy) -> (bool, bool, bool, bool) {
        let require_all = policy == OutputPolicy::RequireAll;
        let mut good = require_all || self.outputs.is_empty();
        let mut done = true;
        let mut bad = !require_all && !self.outputs.is_empty();
        let mut stuck = false;

        for (idx, &root) in self.outputs.iter().enumerate() {
            let value = &self.registers[root];
            if require_all {
                if value.err.total {
                    bad = true;
                } else if value.err.partial {
                    good = false;
                }
            } else {
                good |= !value.err.partial && !value.err.total;
                bad &= value.err.total;
            }
            self.output_distance[idx] = false;

            if !require_all && value.err.total {
                continue;
            }
            if !require_all && value.err.partial {
                done = false;
            }

            let dist = self
                .disc
                .converted_distance(idx, value.lo.as_float(), value.hi.as_float());
            self.output_distance[idx] = dist == 1;
            if dist != 0 {
                done = false;
                if value.lo.immovable && value.hi.immovable {
                    stuck = true;
                }
            }
        }

        (good, done, bad, stuck)
    }
}

fn analysis_triple(analysis: Analysis) -> (Ival, Vec<Hint>, bool) {
    let status = Ival::bool_interval(analysis.is_error, analysis.maybe_error);
    (status, analysis.hints, analysis.converged)
}

/// Errors that can occur during [`Machine::apply`].
///
/// Note that [`Machine::apply`] will only return a result if it can prove
/// that it has correctly rounded every non-error output. It only returns
/// [`RivalError::InvalidInput`] when the selected evaluation policy proves
/// the input invalid.
#[derive(thiserror::Error, Debug)]
pub enum RivalError {
    /// The input point is invalid under the evaluation policy being used.
    ///
    /// For example, taking the square root of a negative number, or
    /// dividing by zero.
    #[error("Invalid input for rival machine")]
    InvalidInput,
    /// Rival was unable to correctly round the output within the
    /// configured precision and iteration limits.
    #[error("Unsamplable input for rival machine")]
    Unsamplable,
}
