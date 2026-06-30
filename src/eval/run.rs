//! Main evaluation loop with adaptive precision tuning.

use itertools::{enumerate, izip};

use crate::eval::{
    execute,
    machine::{Discretization, Hint, Machine},
    profile::Execution,
};
use crate::interval::Ival;

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
    /// # Errors
    ///
    /// Returns [`RivalError::InvalidInput`] if the point is an
    /// invalid input to at least one of the compiled expressions.
    /// Returns [`RivalError::Unsamplable`] if Rival is unable to
    /// evaluate at least one expression.
    ///
    /// Note that `apply` will only return `Ok` if it can prove
    /// that it has correctly-rounded the output. It will only
    /// return `InvalidInput` if it can prove that at least one
    /// output expression in the machine throws on the given input.
    pub fn apply(&mut self, args: &[Ival], hint: Option<&[Hint]>) -> Result<Vec<Ival>, RivalError> {
        self.load_arguments(args);
        let hint_storage;
        let hint_slice: &[Hint] = if let Some(h) = hint {
            h
        } else {
            hint_storage = self.default_hint.clone();
            &hint_storage
        };

        let mut iteration = 0;
        loop {
            if let Some(results) = self.run_iteration(iteration, hint_slice)? {
                return Ok(results);
            }
            iteration = iteration.saturating_add(1);
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
    ) -> Result<Vec<Ival>, RivalError> {
        self.load_arguments(args);

        let hint_storage;
        let hint_slice: &[Hint] = if let Some(h) = hint {
            h
        } else {
            hint_storage = self.default_hint.clone();
            &hint_storage
        };

        let start_prec = self.disc.target().saturating_add(10);
        let mut prec = start_prec;
        let mut iter: usize = 0;

        loop {
            self.iteration = iter;
            self.baseline_adjust(prec);
            self.run_with_hint(hint_slice);

            match self.collect_outputs()? {
                Some(outputs) => return Ok(outputs),
                None => {
                    let next = prec.saturating_mul(2);
                    if next > self.max_precision {
                        return Err(RivalError::Unsamplable);
                    }
                    prec = next;
                    iter = iter.saturating_add(1);
                }
            }
        }
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
    ) -> (Ival, Vec<Hint>, bool) {
        self.load_arguments(rect);

        let tmp;
        let hint_slice = if let Some(h) = hint {
            h
        } else {
            tmp = self.default_hint.clone();
            &tmp
        };

        self.iteration = 0;
        self.baseline_adjust(self.disc.target().saturating_add(10));
        self.run_with_hint(hint_slice);

        let (good, _done, bad, stuck) = self.return_flags();
        let (next_hint, converged) = self.make_hint(hint_slice);

        let status = Ival::bool_interval(bad || stuck, (!good) || stuck);
        (status, next_hint, converged)
    }

    /// Analyze a hyper-rectangle using the baseline strategy and
    /// return only the boolean interval status.
    ///
    /// See [`Machine::analyze`] for details on the return value.
    pub fn analyze_baseline(&mut self, rect: &[Ival]) -> Ival {
        let (status, _hint, _conv) = self.analyze_baseline_with_hints(rect, None);
        status
    }

    /// Run a single iteration with precision tuning and hint-guided evaluation.
    pub(crate) fn run_iteration(
        &mut self,
        iteration: usize,
        hints: &[Hint],
    ) -> Result<Option<Vec<Ival>>, RivalError> {
        assert_eq!(hints.len(), self.instructions.len(), "hint length mismatch");
        self.iteration = iteration;
        if self.adjust(hints) {
            return Err(RivalError::Unsamplable);
        }
        self.run_with_hint(hints);
        self.collect_outputs()
    }

    /// Analyze an input rectangle using adaptive precision tuning.
    ///
    /// Returns a `(status, hints, converged)` tuple:
    ///
    /// - `status` is a boolean interval indicating whether a call to
    ///   [`Machine::apply`] with inputs in the supplied `rect` is
    ///   guaranteed to raise an error. If false is returned, there is
    ///   no point calling `apply` with any point in the input range.
    ///   If uncertain, some points may raise errors while others may
    ///   not, though nothing is guaranteed. If true is returned,
    ///   `InvalidInput` will not be raised for any point in the range;
    ///   however, `Unsamplable` may still be raised.
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
    ) -> (Ival, Vec<Hint>, bool) {
        self.load_arguments(rect);

        // Use provided hint or default.
        let tmp;
        let hint_slice = if let Some(h) = hint {
            h
        } else {
            tmp = self.default_hint.clone();
            &tmp
        };

        // One analysis iteration at sampling iteration 0.
        self.iteration = 0;
        self.adjust(hint_slice);
        self.run_with_hint(hint_slice);

        let (good, _done, bad, stuck) = self.return_flags();
        let (next_hint, converged) = self.make_hint(hint_slice);

        let status = Ival::bool_interval(bad || stuck, (!good) || stuck);
        (status, next_hint, converged)
    }

    /// Analyze a hyper-rectangle and return only the boolean interval status.
    ///
    /// Returns a boolean interval which indicates whether a call to
    /// [`Machine::apply`], with inputs in the supplied `rect`, is
    /// guaranteed to raise an error.
    ///
    /// In other words, if false is returned, there is no point calling
    /// `apply` with any point in the input range. If uncertain, some
    /// points in the range may raise errors, while others may not,
    /// though nothing is guaranteed. If true is returned,
    /// [`RivalError::InvalidInput`] will not be raised for any point
    /// in the range. However, [`RivalError::Unsamplable`] may still
    /// be raised.
    ///
    /// The advantage of `analyze` over `apply` is that it applies to
    /// whole ranges of input points and is much faster.
    pub fn analyze(&mut self, rect: &[Ival]) -> Ival {
        let (status, _hint, _conv) = self.analyze_with_hints(rect, None);
        status
    }

    /// Load argument intervals into the front of the register file.
    pub(crate) fn load_arguments(&mut self, args: &[Ival]) {
        assert_eq!(args.len(), self.arguments.len(), "Argument count mismatch");
        for (i, arg) in args.iter().cloned().enumerate() {
            self.registers[i] = arg;
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
                    self.registers[out_reg] = Ival::bool_interval(*value, *value);
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

    /// Gather outputs and translate evaluation state into convergence results.
    fn collect_outputs(&mut self) -> Result<Option<Vec<Ival>>, RivalError> {
        let (good, done, bad, stuck) = self.return_flags();
        let mut outputs = Vec::with_capacity(self.outputs.len());

        for &root in &self.outputs {
            outputs.push(self.registers[root].clone());
        }

        if bad {
            return Err(RivalError::InvalidInput);
        }
        if done && good {
            return Ok(Some(outputs));
        }
        if stuck {
            return Err(RivalError::Unsamplable);
        }

        Ok(None)
    }

    /// Compute (good, done, bad, stuck) flags and update output_distance.
    fn return_flags(&mut self) -> (bool, bool, bool, bool) {
        let mut good = true;
        let mut done = true;
        let mut bad = false;
        let mut stuck = false;

        for (idx, &root) in self.outputs.iter().enumerate() {
            let value = &self.registers[root];
            let lo = self.disc.convert(idx, value.lo.as_float());
            let hi = self.disc.convert(idx, value.hi.as_float());
            let dist = self.disc.distance(idx, &lo, &hi);
            if dist != 0 {
                done = false;
                if value.lo.immovable && value.hi.immovable {
                    stuck = true;
                }
            }

            if value.err.total {
                bad = true;
            } else if value.err.partial {
                good = false;
            }
            self.output_distance[idx] = dist == 1;
        }

        (good, good && done, bad, stuck)
    }
}

/// Errors that can occur during [`Machine::apply`].
///
/// Note that [`Machine::apply`] will only return a result if it can prove
/// that it has correctly-rounded the output, and it will only return
/// [`RivalError::InvalidInput`] if it can prove that at least one of the
/// output expressions in the machine throws on the given input.
#[derive(thiserror::Error, Debug)]
pub enum RivalError {
    /// The input point is invalid for at least one compiled expression.
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

#[cfg(test)]
mod tests {
    use super::RivalError;
    use crate::{
        eval::{
            ast::Expr,
            machine::{Discretization, MachineBuilder},
        },
        interval::Ival,
    };
    use rug::Float;

    #[derive(Clone)]
    struct TestDisc;

    impl Discretization for TestDisc {
        fn target(&self) -> u32 {
            53
        }

        fn convert(&self, _idx: usize, value: &Float) -> Float {
            value.clone()
        }

        fn distance(&self, _idx: usize, _lo: &Float, _hi: &Float) -> usize {
            0
        }
    }

    #[test]
    fn apply_rejects_point_when_any_output_is_totally_invalid() {
        let sqrt_x = Expr::Sqrt(Box::new(Expr::Var("x".into())));
        let x_expr = Expr::Var("x".into());
        let mut machine =
            MachineBuilder::new(TestDisc).build(vec![sqrt_x, x_expr], vec!["x".into()]);

        let mut x = Ival::zero(machine.argument_precision());
        x.f64_assign(-1.0);

        assert!(matches!(
            machine.apply(&[x], None),
            Err(RivalError::InvalidInput)
        ));
    }
}
