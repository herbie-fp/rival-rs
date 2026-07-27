#ifndef RIVAL3_FFI_H
#define RIVAL3_FFI_H

/* Generated with cbindgen:0.29.2 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include "mpfr.h"

#define RIVAL_ABI_VERSION 2

#define RIVAL_EXPR_INVALID UINT32_MAX

enum RivalError
#ifdef __cplusplus
  : int32_t
#endif // __cplusplus
 {
    RIVAL_ERROR_OK = 0,
    RIVAL_ERROR_INVALID_INPUT = -1,
    RIVAL_ERROR_UNSAMPLABLE = -2,
};
#ifndef __cplusplus
typedef int32_t RivalError;
#endif // __cplusplus

enum RivalDiscType
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_DISC_TYPE_BOOL = 0,
    RIVAL_DISC_TYPE_F32 = 1,
    RIVAL_DISC_TYPE_F64 = 2,
};
#ifndef __cplusplus
typedef uint32_t RivalDiscType;
#endif // __cplusplus

enum RivalUnaryOp
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_UNARY_OP_NEG,
    RIVAL_UNARY_OP_FABS,
    RIVAL_UNARY_OP_SQRT,
    RIVAL_UNARY_OP_CBRT,
    RIVAL_UNARY_OP_POW2,
    RIVAL_UNARY_OP_EXP,
    RIVAL_UNARY_OP_EXP2,
    RIVAL_UNARY_OP_EXPM1,
    RIVAL_UNARY_OP_LOG,
    RIVAL_UNARY_OP_LOG2,
    RIVAL_UNARY_OP_LOG10,
    RIVAL_UNARY_OP_LOG1P,
    RIVAL_UNARY_OP_LOGB,
    RIVAL_UNARY_OP_SIN,
    RIVAL_UNARY_OP_COS,
    RIVAL_UNARY_OP_TAN,
    RIVAL_UNARY_OP_ASIN,
    RIVAL_UNARY_OP_ACOS,
    RIVAL_UNARY_OP_ATAN,
    RIVAL_UNARY_OP_SINH,
    RIVAL_UNARY_OP_COSH,
    RIVAL_UNARY_OP_TANH,
    RIVAL_UNARY_OP_ASINH,
    RIVAL_UNARY_OP_ACOSH,
    RIVAL_UNARY_OP_ATANH,
    RIVAL_UNARY_OP_ERF,
    RIVAL_UNARY_OP_ERFC,
    RIVAL_UNARY_OP_LGAMMA,
    RIVAL_UNARY_OP_TGAMMA,
    RIVAL_UNARY_OP_RINT,
    RIVAL_UNARY_OP_ROUND,
    RIVAL_UNARY_OP_CEIL,
    RIVAL_UNARY_OP_FLOOR,
    RIVAL_UNARY_OP_TRUNC,
    RIVAL_UNARY_OP_NOT,
    RIVAL_UNARY_OP_ASSERT,
    RIVAL_UNARY_OP_ERROR,
};
#ifndef __cplusplus
typedef uint32_t RivalUnaryOp;
#endif // __cplusplus

enum RivalUnaryParamOp
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_UNARY_PARAM_OP_COSU,
    RIVAL_UNARY_PARAM_OP_SINU,
    RIVAL_UNARY_PARAM_OP_TANU,
};
#ifndef __cplusplus
typedef uint32_t RivalUnaryParamOp;
#endif // __cplusplus

enum RivalBinaryOp
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_BINARY_OP_ADD,
    RIVAL_BINARY_OP_SUB,
    RIVAL_BINARY_OP_MUL,
    RIVAL_BINARY_OP_DIV,
    RIVAL_BINARY_OP_POW,
    RIVAL_BINARY_OP_HYPOT,
    RIVAL_BINARY_OP_FMIN,
    RIVAL_BINARY_OP_FMAX,
    RIVAL_BINARY_OP_FDIM,
    RIVAL_BINARY_OP_COPYSIGN,
    RIVAL_BINARY_OP_FMOD,
    RIVAL_BINARY_OP_REMAINDER,
    RIVAL_BINARY_OP_ATAN2,
    RIVAL_BINARY_OP_AND,
    RIVAL_BINARY_OP_OR,
    RIVAL_BINARY_OP_EQ,
    RIVAL_BINARY_OP_NE,
    RIVAL_BINARY_OP_LT,
    RIVAL_BINARY_OP_LE,
    RIVAL_BINARY_OP_GT,
    RIVAL_BINARY_OP_GE,
};
#ifndef __cplusplus
typedef uint32_t RivalBinaryOp;
#endif // __cplusplus

enum RivalTernaryOp
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_TERNARY_OP_FMA,
    RIVAL_TERNARY_OP_IF,
};
#ifndef __cplusplus
typedef uint32_t RivalTernaryOp;
#endif // __cplusplus

enum RivalProfilingMode
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_PROFILING_MODE_OFF = 0,
    RIVAL_PROFILING_MODE_ON = 1,
};
#ifndef __cplusplus
typedef uint32_t RivalProfilingMode;
#endif // __cplusplus

typedef struct RivalDiscretization RivalDiscretization;

typedef struct RivalExprBuilder RivalExprBuilder;

typedef struct RivalHints RivalHints;

typedef struct RivalMachine RivalMachine;

typedef struct RivalAnalyzeResult {
    RivalError error;
    bool is_error;
    bool maybe_error;
    bool converged;
    struct RivalHints *hints;
} RivalAnalyzeResult;

typedef struct RivalExecution {
    int32_t instruction_idx;
    uint32_t precision;
    double time_ms;
    uint32_t iteration;
} RivalExecution;

typedef struct RivalAggregatedProfile {
    int32_t instruction_idx;
    uint32_t precision_bucket;
    double total_time_ms;
    uintptr_t count;
} RivalAggregatedProfile;

typedef struct RivalProfileSummary {
    const struct RivalAggregatedProfile *entries;
    uintptr_t len;
    uint32_t bumps;
    uint32_t iterations;
} RivalProfileSummary;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

uint32_t rival_version(void);

const char *rival_error_message(RivalError error);

struct RivalDiscretization *rival_disc_f64(uint32_t precision);

struct RivalDiscretization *rival_disc_f32(uint32_t precision);

struct RivalDiscretization *rival_disc_bool(void);

struct RivalDiscretization *rival_disc_mixed(const RivalDiscType *types,
                                             uintptr_t n_types,
                                             uint32_t precision);

void rival_disc_free(struct RivalDiscretization *disc);

struct RivalExprBuilder *rival_expr_builder_new(const char *const *vars, uintptr_t n_vars);

void rival_expr_builder_free(struct RivalExprBuilder *builder);

uint32_t rival_expr_var(struct RivalExprBuilder *builder, const char *name);

uint32_t rival_expr_f64(struct RivalExprBuilder *builder, double value);

uint32_t rival_expr_rational(struct RivalExprBuilder *builder, int64_t num, int64_t den);

uint32_t rival_expr_bigint(struct RivalExprBuilder *builder, const char *value);

uint32_t rival_expr_bigrational(struct RivalExprBuilder *builder,
                                const char *numerator,
                                const char *denominator);

uint32_t rival_expr_pi(struct RivalExprBuilder *builder);

uint32_t rival_expr_e(struct RivalExprBuilder *builder);

uint32_t rival_expr_unary(struct RivalExprBuilder *builder, RivalUnaryOp op, uint32_t arg);

uint32_t rival_expr_unary_param(struct RivalExprBuilder *builder,
                                RivalUnaryParamOp op,
                                uint64_t param,
                                uint32_t arg);

uint32_t rival_expr_binary(struct RivalExprBuilder *builder,
                           RivalBinaryOp op,
                           uint32_t lhs,
                           uint32_t rhs);

uint32_t rival_expr_ternary(struct RivalExprBuilder *builder,
                            RivalTernaryOp op,
                            uint32_t arg1,
                            uint32_t arg2,
                            uint32_t arg3);

void rival_hints_free(struct RivalHints *hints);

uintptr_t rival_hints_len(const struct RivalHints *hints);

bool rival_machine_configure_baseline(struct RivalMachine *machine);

struct RivalMachine *rival_machine_new(const struct RivalExprBuilder *builder,
                                       const uint32_t *expr_handles,
                                       uintptr_t n_exprs,
                                       const struct RivalDiscretization *disc,
                                       uint32_t max_precision,
                                       uintptr_t profile_capacity);

void rival_machine_free(struct RivalMachine *machine);

uintptr_t rival_machine_instruction_count(const struct RivalMachine *machine);

uintptr_t rival_machine_var_count(const struct RivalMachine *machine);

uintptr_t rival_machine_expr_count(const struct RivalMachine *machine);

RivalError rival_apply(struct RivalMachine *machine,
                       const mpfr_t *const *args,
                       uintptr_t n_args,
                       mpfr_t *const *out,
                       uintptr_t n_out,
                       const struct RivalHints *hints,
                       uintptr_t max_iterations,
                       uint32_t max_precision,
                       bool require_all_outputs);

RivalError rival_apply_baseline(struct RivalMachine *machine,
                                const mpfr_t *const *args,
                                uintptr_t n_args,
                                mpfr_t *const *out,
                                uintptr_t n_out,
                                const struct RivalHints *hints,
                                uint32_t max_precision,
                                bool require_all_outputs);

struct RivalAnalyzeResult rival_analyze_with_hints(struct RivalMachine *machine,
                                                   const mpfr_t *const *rect,
                                                   uintptr_t n_args,
                                                   const struct RivalHints *hints,
                                                   bool require_all_outputs);

struct RivalAnalyzeResult rival_analyze_baseline_with_hints(struct RivalMachine *machine,
                                                            const mpfr_t *const *rect,
                                                            uintptr_t n_args,
                                                            const struct RivalHints *hints,
                                                            bool require_all_outputs);

uintptr_t rival_profiler_count(const struct RivalMachine *machine);

bool rival_profiler_get(const struct RivalMachine *machine,
                        uintptr_t idx,
                        struct RivalExecution *out);

void rival_profiler_reset(struct RivalMachine *machine);

struct RivalProfileSummary rival_profiler_aggregate(struct RivalMachine *machine,
                                                    uint32_t bucket_size);

const struct RivalExecution *rival_profiler_executions(struct RivalMachine *machine,
                                                       uintptr_t *out_len);

const uint8_t *rival_instruction_names(struct RivalMachine *machine, uintptr_t *out_len);

uint32_t rival_machine_iterations(const struct RivalMachine *machine);

uint32_t rival_machine_bumps(const struct RivalMachine *machine);

void rival_machine_set_profiling(struct RivalMachine *machine, RivalProfilingMode mode);

RivalProfilingMode rival_machine_get_profiling(const struct RivalMachine *machine);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* RIVAL3_FFI_H */
