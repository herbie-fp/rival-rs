//! Macro for defining interval operations and generated helpers.
//! Provides enums, dispatch, amplification bounds, and path reduction wiring.
//! TODO: Split up this macro to make it easier to use/extend.
macro_rules! def_ops {
    (
        constant {
            $( $const_name:ident: { method: $const_method:ident $(,)? } ),* $(,)?
        },
        unary {
            $(
                $unary_name:ident: {
                    method: $unary_method:ident,
                    bounds: $unary_bounds:expr
                    $(, path_reduce: $unary_path_reduce:expr )?
                    $(,)?
                }
            ),* $(,)?
        },
        unary_param {
            $(
                $unary_param_name:ident: {
                    method: $unary_param_method:ident,
                    bounds: $unary_param_bounds:expr
                    $(, path_reduce: $unary_param_path_reduce:expr )?
                    $(,)?
                }
            ),* $(,)?
        },
        binary {
            $(
                $binary_name:ident: {
                    method: $binary_method:ident,
                    bounds: $binary_bounds:expr
                    $(, path_reduce: $binary_path_reduce:expr )?
                    $(,)?
                }
            ),* $(,)?
        },
        ternary {
            $(
                $ternary_name:ident: {
                    method: $ternary_method:ident,
                    bounds: $ternary_bounds:expr
                    $(, path_reduce: $ternary_path_reduce:expr )?
                    $(,)?
                }
            ),* $(,)?
        } $(,)?
    ) => {
        /// Unary instruction operations.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum UnaryOp {
            $( $unary_name, )*
        }

        /// Unary parameterized instruction operations.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum UnaryParamOp {
            $( $unary_param_name, )*
        }

        /// Binary instruction operations.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum BinaryOp {
            $( $binary_name, )*
        }

        /// Ternary instruction operations.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum TernaryOp {
            $( $ternary_name, )*
        }

        /// Constant operations.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum ConstantOp {
            $( $const_name, )*
        }

        pub fn name_of_constant(op: ConstantOp) -> &'static str {
            match op {
                $( ConstantOp::$const_name => stringify!($const_name), )*
            }
        }

        pub fn name_of_unary(op: UnaryOp) -> &'static str {
            match op {
                $( UnaryOp::$unary_name => stringify!($unary_name), )*
            }
        }

        pub fn name_of_unary_param(op: UnaryParamOp) -> &'static str {
            match op {
                $( UnaryParamOp::$unary_param_name => stringify!($unary_param_name), )*
            }
        }

        pub fn name_of_binary(op: BinaryOp) -> &'static str {
            match op {
                $( BinaryOp::$binary_name => stringify!($binary_name), )*
            }
        }

        pub fn name_of_ternary(op: TernaryOp) -> &'static str {
            match op {
                $( TernaryOp::$ternary_name => stringify!($ternary_name), )*
            }
        }

        /// Compute error amplification bounds for unary operations.
        pub fn bounds_for_unary(
            ctx: &TrickContext,
            op: UnaryOp,
            output: &Ival,
            input: &Ival,
        ) -> AmplBounds {
            match op {
                $(
                    UnaryOp::$unary_name => {
                        let bounds_fn: fn(&TrickContext, &Ival, &Ival) -> AmplBounds = $unary_bounds;
                        bounds_fn(ctx, output, input)
                    }
                )*
            }
        }

        /// Compute error amplification bounds for binary operations.
        pub fn bounds_for_binary(
            ctx: &TrickContext,
            op: BinaryOp,
            output: &Ival,
            lhs: &Ival,
            rhs: &Ival,
        ) -> (AmplBounds, AmplBounds) {
            match op {
                $(
                    BinaryOp::$binary_name => {
                        let bounds_fn: fn(&TrickContext, &Ival, &Ival, &Ival) -> (AmplBounds, AmplBounds) = $binary_bounds;
                        bounds_fn(ctx, output, lhs, rhs)
                    }
                )*
            }
        }

        /// Compute error amplification bounds for ternary operations.
        pub fn bounds_for_ternary(
            ctx: &TrickContext,
            op: TernaryOp,
            output: &Ival,
            arg1: &Ival,
            arg2: &Ival,
            arg3: &Ival,
        ) -> (AmplBounds, AmplBounds, AmplBounds) {
            match op {
                $(
                    TernaryOp::$ternary_name => {
                        let bounds_fn: fn(&TrickContext, &Ival, &Ival, &Ival, &Ival) -> (AmplBounds, AmplBounds, AmplBounds) = $ternary_bounds;
                        bounds_fn(ctx, output, arg1, arg2, arg3)
                    }
                )*
            }
        }

        /// Compute error amplification bounds for unary parameterized operations.
        pub fn bounds_for_unary_param(
            ctx: &TrickContext,
            op: UnaryParamOp,
            param: u64,
            output: &Ival,
            input: &Ival,
        ) -> AmplBounds {
            match op {
                $(
                    UnaryParamOp::$unary_param_name => {
                        let bounds_fn: fn(&TrickContext, u64, &Ival, &Ival) -> AmplBounds = $unary_param_bounds;
                        bounds_fn(ctx, param, output, input)
                    }
                )*
            }
        }

        /// Path reduction for unary operations.
        pub fn path_reduce_unary<D, F>(
            op: UnaryOp,
            machine: &$crate::eval::machine::Machine<D>,
            idx: usize,
            mut mark: F,
        ) -> $crate::eval::machine::PathOutcome
        where
            D: $crate::eval::machine::Discretization,
            F: FnMut(usize),
        {
            match op {
                $(
                    UnaryOp::$unary_name => def_ops!(@path_reduce_impl machine, idx, mark ; $( $unary_path_reduce )?),
                )*
            }
        }

        /// Path reduction for binary operations.
        pub fn path_reduce_binary<D, F>(
            op: BinaryOp,
            machine: &$crate::eval::machine::Machine<D>,
            idx: usize,
            mut mark: F,
        ) -> $crate::eval::machine::PathOutcome
        where
            D: $crate::eval::machine::Discretization,
            F: FnMut(usize),
        {
            match op {
                $(
                    BinaryOp::$binary_name => def_ops!(@path_reduce_impl machine, idx, mark ; $( $binary_path_reduce )?),
                )*
            }
        }

        /// Path reduction for ternary operations.
        pub fn path_reduce_ternary<D, F>(
            op: TernaryOp,
            machine: &$crate::eval::machine::Machine<D>,
            idx: usize,
            mut mark: F,
        ) -> $crate::eval::machine::PathOutcome
        where
            D: $crate::eval::machine::Discretization,
            F: FnMut(usize),
        {
            match op {
                $(
                    TernaryOp::$ternary_name => def_ops!(@path_reduce_impl machine, idx, mark ; $( $ternary_path_reduce )?),
                )*
            }
        }

        /// Path reduction for unary parameterized operations.
        pub fn path_reduce_unary_param<D, F>(
            op: UnaryParamOp,
            machine: &$crate::eval::machine::Machine<D>,
            idx: usize,
            mut mark: F,
        ) -> $crate::eval::machine::PathOutcome
        where
            D: $crate::eval::machine::Discretization,
            F: FnMut(usize),
        {
            match op {
                $(
                    UnaryParamOp::$unary_param_name => def_ops!(@path_reduce_impl machine, idx, mark ; $( $unary_param_path_reduce )?),
                )*
            }
        }

        /// Execute unary operation on interval.
        pub fn execute_unary(op: UnaryOp, output: &mut Ival, input: &Ival) {
            match op {
                $(
                    UnaryOp::$unary_name => output.$unary_method(input),
                )*
            }
        }

        /// Execute binary operation on interval.
        pub fn execute_binary(op: BinaryOp, output: &mut Ival, lhs: &Ival, rhs: &Ival) {
            match op {
                $(
                    BinaryOp::$binary_name => output.$binary_method(lhs, rhs),
                )*
            }
        }

        /// Execute ternary operation on interval.
        pub fn execute_ternary(op: TernaryOp, output: &mut Ival, arg1: &Ival, arg2: &Ival, arg3: &Ival) {
            match op {
                $(
                    TernaryOp::$ternary_name => output.$ternary_method(arg1, arg2, arg3),
                )*
            }
        }

        /// Execute unary parameterized operation on interval.
        pub fn execute_unary_param(op: UnaryParamOp, param: u64, output: &mut Ival, input: &Ival) {
            match op {
                $(
                    UnaryParamOp::$unary_param_name => output.$unary_param_method(input, param),
                )*
            }
        }

        /// Execute constant operation on interval.
        pub fn execute_constant(op: ConstantOp, output: &mut Ival) {
            match op {
                $(
                    ConstantOp::$const_name => output.$const_method(),
                )*
            }
        }

    };

    // Helper: Path reduction - standard behavior if not specified.
    (@path_reduce_impl $machine:expr, $idx:expr, $mark:expr ; ) => {
        def_ops!(@standard_path_reduce $machine, $idx, $mark)
    };
    // Helper: Path reduction - custom closure if specified.
    (@path_reduce_impl $machine:expr, $idx:expr, $mark:expr ; $closure:expr) => {
        $closure($machine, $idx, $mark)
    };

    // Helper: Standard path reduction (mark all children and execute).
    (@standard_path_reduce $machine:expr, $idx:expr, $mark:expr) => {{
        let instruction = &$machine.instructions[$idx];

        instruction.for_each_input(&mut $mark);

        $crate::eval::machine::PathOutcome {
            hint: $crate::eval::machine::Hint::Execute,
            converged: true,
        }
    }};
}

pub(crate) use def_ops;
