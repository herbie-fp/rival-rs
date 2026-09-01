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

enum RivalUnaryOp
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_UNARY_OP_NEG = 0,
    RIVAL_UNARY_OP_FABS = 1,
    RIVAL_UNARY_OP_SQRT = 2,
    RIVAL_UNARY_OP_CBRT = 3,
    RIVAL_UNARY_OP_POW2 = 4,
    RIVAL_UNARY_OP_EXP = 5,
    RIVAL_UNARY_OP_EXP2 = 6,
    RIVAL_UNARY_OP_EXPM1 = 7,
    RIVAL_UNARY_OP_LOG = 8,
    RIVAL_UNARY_OP_LOG2 = 9,
    RIVAL_UNARY_OP_LOG10 = 10,
    RIVAL_UNARY_OP_LOG1P = 11,
    RIVAL_UNARY_OP_LOGB = 12,
    RIVAL_UNARY_OP_SIN = 13,
    RIVAL_UNARY_OP_COS = 14,
    RIVAL_UNARY_OP_TAN = 15,
    RIVAL_UNARY_OP_ASIN = 16,
    RIVAL_UNARY_OP_ACOS = 17,
    RIVAL_UNARY_OP_ATAN = 18,
    RIVAL_UNARY_OP_SINH = 19,
    RIVAL_UNARY_OP_COSH = 20,
    RIVAL_UNARY_OP_TANH = 21,
    RIVAL_UNARY_OP_ASINH = 22,
    RIVAL_UNARY_OP_ACOSH = 23,
    RIVAL_UNARY_OP_ATANH = 24,
    RIVAL_UNARY_OP_ERF = 25,
    RIVAL_UNARY_OP_ERFC = 26,
    RIVAL_UNARY_OP_LGAMMA = 27,
    RIVAL_UNARY_OP_TGAMMA = 28,
    RIVAL_UNARY_OP_RINT = 29,
    RIVAL_UNARY_OP_ROUND = 30,
    RIVAL_UNARY_OP_CEIL = 31,
    RIVAL_UNARY_OP_FLOOR = 32,
    RIVAL_UNARY_OP_TRUNC = 33,
    RIVAL_UNARY_OP_NOT = 34,
    RIVAL_UNARY_OP_ASSERT = 35,
    RIVAL_UNARY_OP_ERROR = 36,
};
#ifndef __cplusplus
typedef uint32_t RivalUnaryOp;
#endif // __cplusplus

enum RivalUnaryParamOp
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_UNARY_PARAM_OP_COSU = 0,
    RIVAL_UNARY_PARAM_OP_SINU = 1,
    RIVAL_UNARY_PARAM_OP_TANU = 2,
};
#ifndef __cplusplus
typedef uint32_t RivalUnaryParamOp;
#endif // __cplusplus

enum RivalBinaryOp
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_BINARY_OP_ADD = 0,
    RIVAL_BINARY_OP_SUB = 1,
    RIVAL_BINARY_OP_MUL = 2,
    RIVAL_BINARY_OP_DIV = 3,
    RIVAL_BINARY_OP_POW = 4,
    RIVAL_BINARY_OP_HYPOT = 5,
    RIVAL_BINARY_OP_FMIN = 6,
    RIVAL_BINARY_OP_FMAX = 7,
    RIVAL_BINARY_OP_FDIM = 8,
    RIVAL_BINARY_OP_COPYSIGN = 9,
    RIVAL_BINARY_OP_FMOD = 10,
    RIVAL_BINARY_OP_REMAINDER = 11,
    RIVAL_BINARY_OP_ATAN2 = 12,
    RIVAL_BINARY_OP_AND = 13,
    RIVAL_BINARY_OP_OR = 14,
    RIVAL_BINARY_OP_EQ = 15,
    RIVAL_BINARY_OP_NE = 16,
    RIVAL_BINARY_OP_LT = 17,
    RIVAL_BINARY_OP_LE = 18,
    RIVAL_BINARY_OP_GT = 19,
    RIVAL_BINARY_OP_GE = 20,
};
#ifndef __cplusplus
typedef uint32_t RivalBinaryOp;
#endif // __cplusplus

enum RivalTernaryOp
#ifdef __cplusplus
  : uint32_t
#endif // __cplusplus
 {
    RIVAL_TERNARY_OP_FMA = 0,
    RIVAL_TERNARY_OP_IF = 1,
};
#ifndef __cplusplus
typedef uint32_t RivalTernaryOp;
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

const char *rival_error_message(int32_t error);

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

uint32_t rival_expr_unary(struct RivalExprBuilder *builder, uint32_t op, uint32_t arg);

uint32_t rival_expr_unary_param(struct RivalExprBuilder *builder,
                                uint32_t op,
                                uint64_t param,
                                uint32_t arg);

uint32_t rival_expr_binary(struct RivalExprBuilder *builder,
                           uint32_t op,
                           uint32_t lhs,
                           uint32_t rhs);

uint32_t rival_expr_ternary(struct RivalExprBuilder *builder,
                            uint32_t op,
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
                       bool require_all_outputs);

RivalError rival_apply_baseline(struct RivalMachine *machine,
                                const mpfr_t *const *args,
                                uintptr_t n_args,
                                mpfr_t *const *out,
                                uintptr_t n_out,
                                const struct RivalHints *hints,
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

void rival_machine_set_profiling(struct RivalMachine *machine, uint32_t mode);

RivalProfilingMode rival_machine_get_profiling(const struct RivalMachine *machine);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* RIVAL3_FFI_H */
