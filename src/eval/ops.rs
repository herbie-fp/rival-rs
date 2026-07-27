//! Operation registry generating evaluation and optimization helpers
//! Defines interval operators along with dispatch, bounds, and path reduction hooks
use crate::eval::adjust::path_reduction;
use crate::eval::macros::def_ops;
use crate::eval::tricks::{AmplBounds, TrickContext, crosses_zero, get_slack};
use crate::interval::Ival;

def_ops! {
    constant {
        Pi: {
            method: set_pi,
        },

        E: {
            method: set_e,
        },
    },

    unary {
        Pow2: {
            method: pow2_assign,
            bounds: |ctx, _out, inp| {
                AmplBounds::new(ctx.logspan(inp) + 1, 0)
            },
        },
        Fabs: {
            method: fabs_assign,
            bounds: |_, _, _| AmplBounds::zero(),
        },

        Neg: {
            method: neg_assign,
            bounds: |_, _, _| AmplBounds::zero(),
        },

        Sqrt: {
            method: sqrt_assign,
            bounds: |ctx, _, inp| AmplBounds::new((ctx.logspan(inp) / 2).saturating_sub(1), 0),
        },

        Cbrt: {
            method: cbrt_assign,
            bounds: |ctx, _, inp| AmplBounds::new(((2 * ctx.logspan(inp)) / 3).saturating_sub(1), 0),
        },

        Exp: {
            method: exp_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.maxlog(inp, false) + ctx.logspan(out);
                let lower = if ctx.lower_bound_early_stopping {
                    ctx.minlog(inp, true)
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Exp2: {
            method: exp2_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.maxlog(inp, false) + ctx.logspan(out);
                let lower = if ctx.lower_bound_early_stopping {
                    ctx.minlog(inp, true)
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Expm1: {
            method: expm1_assign,
            bounds: |ctx, out, inp| {
                let mx = ctx.maxlog(inp, false);
                let upper = (1 + mx).max(1 + mx - ctx.minlog(out, false));
                AmplBounds::new(upper, 0)
            },
        },

        Log: {
            method: log_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.logspan(inp) - ctx.minlog(out, false) + 1;
                let lower = if ctx.lower_bound_early_stopping {
                    -ctx.maxlog(out, true)
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Log2: {
            method: log2_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.logspan(inp) - ctx.minlog(out, false) + 1;
                let lower = if ctx.lower_bound_early_stopping {
                    -ctx.maxlog(out, true)
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Log10: {
            method: log10_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.logspan(inp) - ctx.minlog(out, false) + 1;
                let lower = if ctx.lower_bound_early_stopping {
                    -ctx.maxlog(out, true)
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Log1p: {
            method: log1p_assign,
            bounds: |ctx, out, inp| {
                let upper_base = ctx.maxlog(inp, false) - ctx.minlog(out, false);
                let lo_neg = inp.lo.as_float().is_sign_negative();
                let hi_neg = inp.hi.as_float().is_sign_negative();
                let slack = if lo_neg || hi_neg { get_slack(ctx.iteration, ctx.slack_unit) } else { 0 };
                AmplBounds::new(upper_base + slack, 0)
            },
        },

        Logb: {
            method: logb_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Sin: {
            method: sin_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.maxlog(inp, false) - ctx.minlog(out, false);
                let lower = if ctx.lower_bound_early_stopping {
                    if ctx.maxlog(inp, false) >= 1 { -1 - ctx.maxlog(out, true) } else { 0 }
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Cos: {
            method: cos_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.maxlog(inp, false) - ctx.minlog(out, false)
                    + ctx.maxlog(inp, false).min(0);
                let lower = if ctx.lower_bound_early_stopping {
                    -ctx.maxlog(out, true) - 2
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Tan: {
            method: tan_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.maxlog(inp, false)
                    + ctx.maxlog(out, false).abs().max(ctx.minlog(out, false).abs())
                    + ctx.logspan(out)
                    + 1;
                let lower = if ctx.lower_bound_early_stopping {
                    ctx.minlog(inp, true)
                        + ctx
                            .maxlog(out, true)
                            .abs()
                            .min(ctx.minlog(out, true).abs())
                        - 1
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Asin: {
            method: asin_assign,
            bounds: |ctx, out, _| {
                let upper = if ctx.maxlog(out, false) >= 1 { get_slack(ctx.iteration, ctx.slack_unit) } else { 1 };
                AmplBounds::new(upper, 0)
            },
        },

        Acos: {
            method: acos_assign,
            bounds: |ctx, _out, inp| {
                let upper = if ctx.maxlog(inp, false) >= 0 { get_slack(ctx.iteration, ctx.slack_unit) } else { 0 };
                AmplBounds::new(upper, 0)
            },
        },

        Atan: {
            method: atan_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.logspan(inp)
                    - ctx.minlog(inp, false).abs().min(ctx.maxlog(inp, false).abs())
                    - ctx.minlog(out, false);
                let lower = if ctx.lower_bound_early_stopping {
                    - (ctx.minlog(inp, true).abs().max(ctx.maxlog(inp, true).abs()))
                        - ctx.maxlog(out, true)
                        - 2
                } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Sinh: {
            method: sinh_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.maxlog(inp, false) + ctx.logspan(out) - ctx.minlog(inp, false).min(0);
                let lower = if ctx.lower_bound_early_stopping { ctx.minlog(inp, true).max(0) } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Cosh: {
            method: cosh_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.maxlog(inp, false) + ctx.logspan(out) + ctx.maxlog(inp, false).min(0);
                let lower = if ctx.lower_bound_early_stopping { (ctx.minlog(inp, true) - 1).max(0) } else { 0 };
                AmplBounds::new(upper, lower)
            },
        },

        Tanh: {
            method: tanh_assign,
            bounds: |ctx, out, inp| {
                let upper = ctx.logspan(out) + ctx.logspan(inp);
                AmplBounds::new(upper, 0)
            },
        },

        Asinh: {
            method: asinh_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Acosh: {
            method: acosh_assign,
            bounds: |ctx, out, _| {
                let z_exp = ctx.minlog(out, false);
                let upper = if z_exp < 2 { get_slack(ctx.iteration, ctx.slack_unit) - z_exp } else { 0 };
                AmplBounds::new(upper, 0)
            },
        },

        Atanh: {
            method: atanh_assign,
            bounds: |ctx, _out, inp| {
                let upper = if ctx.maxlog(inp, false) >= 1 { get_slack(ctx.iteration, ctx.slack_unit) } else { 1 };
                AmplBounds::new(upper, 0)
            },
        },

        Erf: {
            method: erf_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Erfc: {
            method: erfc_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Lgamma: {
            method: lgamma_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Tgamma: {
            method: tgamma_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Rint: {
            method: rint_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Round: {
            method: round_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Ceil: {
            method: ceil_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Floor: {
            method: floor_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Trunc: {
            method: trunc_assign,
            bounds: |ctx, _, _| AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0),
        },

        Not: {
            method: not_assign,
            bounds: |_, _, _| AmplBounds::zero(),
            path_reduce: path_reduction::not_op_path_reduce,
        },

        Error: {
            method: error_assign,
            bounds: |_, _, _| AmplBounds::zero(),
        },

        Assert: {
            method: assert_assign,
            bounds: |_, _, _| AmplBounds::zero(),
            path_reduce: path_reduction::assert_op_path_reduce,
        },
    },

    unary_param {
        Cosu: {
            method: cosu_assign,
            bounds: |ctx, param, out, inp| {
                let n_log = param as i64;
                let upper = ctx.maxlog(inp, false) - n_log - ctx.minlog(out, false) + 2;
                let lower = 0;
                AmplBounds::new(upper, lower)
            },
        },

        Sinu: {
            method: sinu_assign,
            bounds: |ctx, param, out, inp| {
                let n_log = param as i64;
                let upper = ctx.maxlog(inp, false) - n_log - ctx.minlog(out, false) + 2;
                let lower = 0;
                AmplBounds::new(upper, lower)
            },
        },

        Tanu: {
            method: tanu_assign,
            bounds: |ctx, param, out, inp| {
                let n_log = param as i64;
                let upper = ctx.maxlog(inp, false) - n_log
                    + ctx.maxlog(out, false).abs().max(ctx.minlog(out, false).abs()) + 3;
                let lower = 0;
                AmplBounds::new(upper, lower)
            },
        },
    },

    binary {
        Pow: {
            method: pow_assign,
            bounds: |ctx, out, x, y| {
                let maxlog_y = ctx.maxlog(y, false);
                let minlog_y_less = ctx.minlog(y, true);
                let logspan_x = ctx.logspan(x);
                let logspan_out = ctx.logspan(out);
                let maxlog_x = ctx.maxlog(x, false);
                let minlog_x = ctx.minlog(x, false);
                // Slack adjustments
                let y_slack = if crosses_zero(out) && x.lo.as_float().is_sign_negative() { get_slack(ctx.iteration, ctx.slack_unit) } else { 0 };
                let x_slack = if out.lo.as_float().is_zero() { get_slack(ctx.iteration, ctx.slack_unit) } else { 0 };

                // Upper bounds
                let upper_x_base = maxlog_y + logspan_x + logspan_out + x_slack;
                let upper_x = upper_x_base.max(x_slack);
                let abs_maxlog_x = maxlog_x.abs();
                let abs_minlog_x = minlog_x.abs();
                let span_x_mag = abs_maxlog_x.max(abs_minlog_x);
                let upper_y_base = maxlog_y + span_x_mag + logspan_out + y_slack;
                let upper_y = upper_y_base.max(y_slack);

                // Lower bounds
                let lower_x = if ctx.lower_bound_early_stopping { minlog_y_less } else { 0 };
                let min_abs_span = abs_maxlog_x.min(abs_minlog_x);
                let lower_y = if ctx.lower_bound_early_stopping {
                    if min_abs_span == 0 { 0 } else { minlog_y_less }
                } else { 0 };

                (AmplBounds::new(upper_x, lower_x), AmplBounds::new(upper_y, lower_y))
            },
        },

        Fdim: {
            method: fdim_assign,
            bounds: |ctx, out, x, y| {
                let output_min = ctx.minlog(out, false);
                let lhs_upper = ctx.maxlog(x, false) - output_min;
                let rhs_upper = ctx.maxlog(y, false) - output_min;
                let lhs_lower = if ctx.lower_bound_early_stopping { ctx.minlog(x, true) - ctx.maxlog(out, true) } else { 0 };
                let rhs_lower = if ctx.lower_bound_early_stopping { ctx.minlog(y, true) - ctx.maxlog(out, true) } else { 0 };
                (AmplBounds::new(lhs_upper, lhs_lower), AmplBounds::new(rhs_upper, rhs_lower))
            },
        },

        Hypot: {
            method: hypot_assign,
            bounds: |ctx, _, _, _| {
                let bounds = AmplBounds::new(get_slack(ctx.iteration, ctx.slack_unit), 0);
                (bounds, bounds)
            },
        },
        Add: {
            method: add_assign,
            bounds: |ctx, out, lhs, rhs| {
                let output_min = ctx.minlog(out, false);
                let lhs_upper = ctx.maxlog(lhs, false) - output_min;
                let rhs_upper = ctx.maxlog(rhs, false) - output_min;
                let lhs_lower = if ctx.lower_bound_early_stopping {
                    ctx.minlog(lhs, true) - ctx.maxlog(out, true)
                } else { 0 };
                let rhs_lower = if ctx.lower_bound_early_stopping {
                    ctx.minlog(rhs, true) - ctx.maxlog(out, true)
                } else { 0 };
                (AmplBounds::new(lhs_upper, lhs_lower), AmplBounds::new(rhs_upper, rhs_lower))
            },
        },

        Sub: {
            method: sub_assign,
            bounds: |ctx, out, lhs, rhs| {
                let output_min = ctx.minlog(out, false);
                let lhs_upper = ctx.maxlog(lhs, false) - output_min;
                let rhs_upper = ctx.maxlog(rhs, false) - output_min;
                let lhs_lower = if ctx.lower_bound_early_stopping {
                    ctx.minlog(lhs, true) - ctx.maxlog(out, true)
                } else { 0 };
                let rhs_lower = if ctx.lower_bound_early_stopping {
                    ctx.minlog(rhs, true) - ctx.maxlog(out, true)
                } else { 0 };
                (AmplBounds::new(lhs_upper, lhs_lower), AmplBounds::new(rhs_upper, rhs_lower))
            },
        },

        Mul: {
            method: mul_assign,
            bounds: |ctx, _, lhs, rhs| {
                (AmplBounds::new(ctx.logspan(rhs), 0),
                 AmplBounds::new(ctx.logspan(lhs), 0))
            },
        },

        Div: {
            method: div_assign,
            bounds: |ctx, _, lhs, rhs| {
                let lhs_bounds = AmplBounds::new(ctx.logspan(rhs), 0);
                let rhs_bounds = AmplBounds::new(ctx.logspan(lhs) + 2 * ctx.logspan(rhs), 0);
                (lhs_bounds, rhs_bounds)
            },
        },

        And: {
            method: and_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::bool_op_path_reduce,
        },

        Or: {
            method: or_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::bool_op_path_reduce,
        },

        Eq: {
            method: eq_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::bool_op_path_reduce,
        },

        Ne: {
            method: ne_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::bool_op_path_reduce,
        },

        Lt: {
            method: lt_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::bool_op_path_reduce,
        },

        Le: {
            method: le_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::bool_op_path_reduce,
        },

        Gt: {
            method: gt_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::bool_op_path_reduce,
        },

        Ge: {
            method: ge_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::bool_op_path_reduce,
        },

        Fmin: {
            method: fmin_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: |machine, idx, mark| {
                path_reduction::minmax_path_reduce(machine, idx, mark, false)
            },
        },

        Fmax: {
            method: fmax_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: |machine, idx, mark| {
                path_reduction::minmax_path_reduce(machine, idx, mark, true)
            },
        },

        Copysign: {
            method: copysign_assign,
            bounds: |_, _, _, _| (AmplBounds::zero(), AmplBounds::zero()),
        },

        Atan2: {
            method: atan2_assign,
            bounds: |ctx, out, y, x| {
                let upper = ctx.maxlog(x, false) + ctx.maxlog(y, false)
                    - 2 * ctx.minlog(x, false).min(ctx.minlog(y, false))
                    - ctx.minlog(out, false);
                let lower = if ctx.lower_bound_early_stopping {
                    ctx.minlog(x, true) + ctx.minlog(y, true)
                        - 2 * ctx.maxlog(x, true).max(ctx.maxlog(y, true))
                        - ctx.maxlog(out, true)
                } else { 0 };
                (AmplBounds::new(upper, lower), AmplBounds::new(upper, lower))
            },
        },

        Fmod: {
            method: fmod_assign,
            bounds: |ctx, out, x, y| {
                let slack = if crosses_zero(y) { get_slack(ctx.iteration, ctx.slack_unit) } else { 0 };
                let upper_x = ctx.maxlog(x, false) - ctx.minlog(out, false);
                let upper_y = upper_x + slack;
                (AmplBounds::new(upper_x, 0), AmplBounds::new(upper_y, 0))
            },
        },

        Remainder: {
            method: remainder_assign,
            bounds: |ctx, out, x, y| {
                let slack = if crosses_zero(y) { get_slack(ctx.iteration, ctx.slack_unit) } else { 0 };
                let upper_x = ctx.maxlog(x, false) - ctx.minlog(out, false);
                let upper_y = upper_x + slack;
                (AmplBounds::new(upper_x, 0), AmplBounds::new(upper_y, 0))
            },
        },
    },

    ternary {
        Fma: {
            method: fma_assign,
            bounds: |ctx, out, a, b, _c| {
                (AmplBounds::new(ctx.logspan(b) + ctx.logspan(out), 0),
                 AmplBounds::new(ctx.logspan(a) + ctx.logspan(out), 0),
                 AmplBounds::new(ctx.logspan(out), 0))
            },
        },

        If: {
            method: if_assign,
            bounds: |_, _, _, _, _| (AmplBounds::zero(), AmplBounds::zero(), AmplBounds::zero()),
            path_reduce: path_reduction::if_op_path_reduce,
        },
    },
}
