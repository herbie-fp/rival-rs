//! Per-point precision minimization for compiled machines.

use crate::{
    RivalError,
    eval::machine::{Discretization, Machine},
    interval::Ival,
};

impl<D: Discretization> Machine<D> {
    /// Find a lower per-instruction precision assignment that still succeeds at `args`.
    ///
    /// Returns `Ok(None)` when neither the precision assignment left by normal
    /// evaluation nor an all-max-precision assignment succeeds when replayed
    /// directly.
    pub fn find_optimal_precisions(
        &mut self,
        args: &[Ival],
    ) -> Result<Option<Vec<u32>>, RivalError> {
        let _ = self.apply(args, None);

        let mut optimal_precisions = self.precisions.clone();
        for idx in 0..optimal_precisions.len() {
            if !self.initial_repeats[idx] {
                optimal_precisions[idx] = optimal_precisions[idx].max(self.initial_precisions[idx]);
            }
        }

        if !self.test_precision_vector(args, &optimal_precisions)? {
            optimal_precisions.fill(self.max_precision);
            if !self.test_precision_vector(args, &optimal_precisions)? {
                return Ok(None);
            }
        }

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

        Ok(Some(optimal_precisions))
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
