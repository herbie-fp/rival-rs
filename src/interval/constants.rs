use super::value::{ErrorFlags, Ival};
use crate::mpfr::{mpfr_e, mpfr_pi};
use rug::float::Round;

impl Ival {
    /// Set this interval to contain pi.
    pub fn set_pi(&mut self) {
        mpfr_pi(self.lo.as_float_mut(), Round::Down);
        mpfr_pi(self.hi.as_float_mut(), Round::Up);
        self.lo.immovable = false;
        self.hi.immovable = false;
        self.err = ErrorFlags::none();
    }

    /// Set this interval to contain e.
    pub fn set_e(&mut self) {
        mpfr_e(self.lo.as_float_mut(), Round::Down);
        mpfr_e(self.hi.as_float_mut(), Round::Up);
        self.lo.immovable = false;
        self.hi.immovable = false;
        self.err = ErrorFlags::none();
    }
}
