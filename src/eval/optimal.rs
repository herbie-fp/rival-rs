//! Per-point precision minimization for compiled machines.

use std::time::Instant;

use crate::{
    RivalError,
    eval::machine::{Discretization, Machine},
    interval::Ival,
};

/// Precision-minimization result for a machine at one input point.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimalPrecisionResult {
    /// Minimal precision assignment found by searching each instruction.
    pub optimal_precisions: Vec<u32>,
    /// Best observed average runtime for `optimal_precisions`, in milliseconds.
    pub optimal_time_ms: f64,
    /// Precision assignment produced by the normal tuning pass before minimization.
    pub tuned_precisions: Vec<u32>,
    /// Best observed average runtime for `tuned_precisions`, in milliseconds.
    pub tuned_time_ms: f64,
}

impl<D: Discretization> Machine<D> {
    /// Find a lower per-instruction precision assignment that still succeeds at `args`.
    ///
    /// Returns `Ok(None)` when the tuned precision assignment produced by normal
    /// evaluation does not succeed when replayed directly.
    pub fn find_optimal_precisions(
        &mut self,
        args: &[Ival],
    ) -> Result<Option<OptimalPrecisionResult>, RivalError> {
        let _ = self.apply(args, None)?;

        let mut tuned_precisions = self.precisions.clone();
        for idx in 0..tuned_precisions.len() {
            if !self.initial_repeats[idx] {
                tuned_precisions[idx] = tuned_precisions[idx].max(self.initial_precisions[idx]);
            }
        }

        if !self.test_precision_vector(args, &tuned_precisions)? {
            return Ok(None);
        }

        let mut tuned_time_ms = f64::INFINITY;
        for _ in 0..5 {
            let start = Instant::now();
            for _ in 0..10 {
                if !self.test_precision_vector(args, &tuned_precisions)? {
                    return Err(RivalError::Unsamplable);
                }
            }
            tuned_time_ms = tuned_time_ms.min(start.elapsed().as_secs_f64() * 100.0);
        }

        let mut optimal_precisions = tuned_precisions.clone();

        for idx in (0..self.instructions.len()).rev() {
            let mut test_vec = optimal_precisions.clone();

            let mut lo = 2;
            let mut hi = test_vec[idx];
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                test_vec[idx] = mid;
                if self.test_precision_vector(args, &test_vec)? {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            optimal_precisions[idx] = hi;
        }

        if !self.test_precision_vector(args, &optimal_precisions)? {
            return Err(RivalError::Unsamplable);
        }

        let mut optimal_time_ms = f64::INFINITY;
        for _ in 0..5 {
            let start = Instant::now();
            for _ in 0..10 {
                if !self.test_precision_vector(args, &optimal_precisions)? {
                    return Err(RivalError::Unsamplable);
                }
            }
            optimal_time_ms = optimal_time_ms.min(start.elapsed().as_secs_f64() * 100.0);
        }

        Ok(Some(OptimalPrecisionResult {
            optimal_precisions,
            optimal_time_ms,
            tuned_precisions,
            tuned_time_ms,
        }))
    }

    fn test_precision_vector(
        &mut self,
        args: &[Ival],
        precision_vec: &[u32],
    ) -> Result<bool, RivalError> {
        if precision_vec.len() != self.instructions.len() {
            return Err(RivalError::InvalidInput);
        }

        self.load_arguments(args);
        self.iteration = 1;
        self.precisions.copy_from_slice(precision_vec);
        self.repeats.copy_from_slice(&self.initial_repeats);
        self.run_with_hint(&self.default_hint.clone());

        let (good, done, bad, stuck) = self.return_flags();
        Ok(good && done && !bad && !stuck)
    }
}
