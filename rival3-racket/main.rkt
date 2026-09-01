#lang racket

(require ffi/unsafe
         ffi/unsafe/define
         (only-in racket/contract [-> c:->] [->* c:->*] [->i c:->i])
         racket/runtime-path
         math/bigfloat
         math/flonum
         (only-in math/private/bigfloat/mpfr _mpfr-pointer)
         "ops.rkt")

;; No contract for functions that tend to be extremely hot.
(provide rival-apply
         rival-apply/partial
         baseline-apply
         baseline-apply/partial
         rival-analyze-with-hints
         rival-analyze-with-hints/partial
         rival-analyze
         rival-analyze/partial
         baseline-analyze-with-hints
         baseline-analyze-with-hints/partial
         baseline-analyze
         baseline-analyze/partial)

(provide (contract-out
          [rival-compile compile/c]
          [baseline-compile compile/c]
          [rival-machine? (c:-> any/c boolean?)]
          [rival-hints? (c:-> any/c boolean?)]
          [rival-profile (c:-> rival-machine? profile-key/c any)]
          [rival-set-profiling! (c:-> rival-machine? any/c void?)]
          [rival-profiling-enabled? (c:-> rival-machine? boolean?)]
          [boolean-discretization discretization/c]
          [flonum-discretization discretization/c])
         (struct-out exn:rival)
         (struct-out exn:rival:invalid)
         (struct-out exn:rival:unsamplable)
         (struct-out execution)
         (struct-out discretization)
         (struct-out ival)
         *rival-max-precision*
         *rival-max-iterations*
         *rival-profile-executions*
         (all-from-out "ops.rkt"))

(struct exn:rival exn:fail ())
(struct exn:rival:invalid exn:rival (pt))
(struct exn:rival:unsamplable exn:rival (pt))
(struct execution (name number precision time memory iteration) #:prefab)
(struct discretization (type target convert))
(struct ival (lo hi) #:transparent)

(define (bf->bool x)
  (and (not (bfzero? x)) #t))

(define (input->bf x)
  (if (boolean? x)
      (bf (if x 1 0))
      x))

(define (exactly-representable-at-current-bf-precision? x)
  (define lo
    (parameterize ([bf-rounding-mode 'down])
      (bf x)))
  (define hi
    (parameterize ([bf-rounding-mode 'up])
      (bf x)))
  (equal? lo hi))

(define boolean-discretization (discretization 'bool 53 bf->bool))
(define flonum-discretization (discretization 'f64 53 bigfloat->flonum))

(define *rival-max-precision* (make-parameter 10000))
(define *rival-max-iterations* (make-parameter 5))
(define *rival-profile-executions* (make-parameter 1000))

(define-runtime-path native-root (build-path "private" "native"))
;; Dev fallback for local builds of Rival 3
(define dev-native-root
  (build-path native-root ".." ".." ".." "rival3-ffi" "target" "release"))

(define _lib-name
  (string-append (case (system-type)
                   [(windows) "rival3_ffi"]
                   [else      "librival3_ffi"])
                 (bytes->string/utf-8 (system-type 'so-suffix))))

(define _lib-path
  (let ([pkg-path (build-path native-root
                              (system-library-subpath #f)
                              _lib-name)])
    (if (file-exists? pkg-path)
        pkg-path
        (build-path dev-native-root _lib-name))))

(define-ffi-definer define-rival (ffi-lib _lib-path))

(define _rival-error (_enum '(ok = 0 invalid_input = -1 unsamplable = -2) _int32))
(define _analyze-result (_list-struct _rival-error _stdbool _stdbool _stdbool _pointer))
(define _profile-summary (_list-struct _pointer _size _uint32 _uint32))
(define _execution-record (_list-struct _int32 _uint32 _double _uint32))
(define execution-record-size (ctype-sizeof _execution-record))
(define _aggregated-entry (_list-struct _int32 _uint32 _double _size))
(define aggregated-entry-size (ctype-sizeof _aggregated-entry))

(define _rival-profiling-mode (_enum '(off = 0 on = 1) _uint32))
(define _rival-disc-type (_enum '(bool = 0 f32 = 1 f64 = 2) _uint32))

(define RIVAL_EXPR_INVALID #xFFFFFFFF)
(define unary-op-codes
  '(neg = 0 fabs = 1 sqrt = 2 cbrt = 3 pow2 = 4
        exp = 5 exp2 = 6 expm1 = 7 log = 8 log2 = 9 log10 = 10 log1p = 11 logb = 12
        sin = 13 cos = 14 tan = 15 asin = 16 acos = 17 atan = 18
        sinh = 19 cosh = 20 tanh = 21 asinh = 22 acosh = 23 atanh = 24
        erf = 25 erfc = 26 lgamma = 27 tgamma = 28
        rint = 29 round = 30 ceil = 31 floor = 32 trunc = 33
        not = 34 assert = 35 error = 36))

(define _rival-unary-op (_enum unary-op-codes _uint32))
(define _rival-unary-param-op (_enum '(cosu = 0 sinu = 1 tanu = 2) _uint32))
(define _rival-binary-op
  (_enum '(add = 0 sub = 1 mul = 2 div = 3 pow = 4 hypot = 5
               fmin = 6 fmax = 7 fdim = 8 copysign = 9 fmod = 10 remainder = 11 atan2 = 12
               and = 13 or = 14 eq = 15 ne = 16 lt = 17 le = 18 gt = 19 ge = 20)
         _uint32))
(define _rival-ternary-op (_enum '(fma = 0 if = 1) _uint32))

(define-rival rival_version (_fun -> _uint32))
(define-rival rival_disc_f64 (_fun _uint32 -> _pointer))
(define-rival rival_disc_f32 (_fun _uint32 -> _pointer))
(define-rival rival_disc_bool (_fun -> _pointer))
(define-rival rival_disc_mixed (_fun _pointer _size _uint32 -> _pointer))
(define-rival rival_disc_free (_fun _pointer -> _void))

(define-rival rival_expr_builder_new (_fun _pointer _size -> _pointer))
(define-rival rival_expr_builder_free (_fun _pointer -> _void))

(define-rival rival_expr_var (_fun _pointer _string -> _uint32))
(define-rival rival_expr_f64 (_fun _pointer _double -> _uint32))
(define-rival rival_expr_rational (_fun _pointer _int64 _int64 -> _uint32))
(define-rival rival_expr_bigint (_fun _pointer _string -> _uint32))
(define-rival rival_expr_bigrational (_fun _pointer _string _string -> _uint32))
(define-rival rival_expr_pi (_fun _pointer -> _uint32))
(define-rival rival_expr_e (_fun _pointer -> _uint32))

(define-rival rival_expr_unary (_fun _pointer _rival-unary-op _uint32 -> _uint32))
(define-rival rival_expr_unary_param (_fun _pointer _rival-unary-param-op _uint64 _uint32 -> _uint32))
(define-rival rival_expr_binary (_fun _pointer _rival-binary-op _uint32 _uint32 -> _uint32))
(define-rival rival_expr_ternary (_fun _pointer _rival-ternary-op _uint32 _uint32 _uint32 -> _uint32))

(define-rival rival_machine_new (_fun _pointer _pointer _size _pointer _uint32 _size -> _pointer))
(define-rival rival_machine_free (_fun _pointer -> _void))
(define-rival rival_machine_configure_baseline (_fun _pointer -> _stdbool))
(define-rival rival_machine_instruction_count (_fun _pointer -> _size))
(define-rival rival_machine_iterations (_fun _pointer -> _uint32))
(define-rival rival_machine_bumps (_fun _pointer -> _uint32))
(define-rival rival_machine_set_profiling (_fun _pointer _rival-profiling-mode -> _void))
(define-rival rival_machine_get_profiling (_fun _pointer -> _rival-profiling-mode))

(define-rival rival_apply
              (_fun _pointer _pointer _size _pointer _size _pointer _size _stdbool
                    -> _rival-error))

(define-rival rival_apply_baseline
              (_fun _pointer _pointer _size _pointer _size _pointer _stdbool -> _rival-error))

(define-rival rival_analyze_with_hints
              (_fun _pointer _pointer _size _pointer _stdbool -> _analyze-result))
(define-rival rival_analyze_baseline_with_hints
              (_fun _pointer _pointer _size _pointer _stdbool -> _analyze-result))

(define-rival rival_hints_free (_fun _pointer -> _void))
(define-rival rival_hints_len (_fun _pointer -> _size))

(define-rival rival_profiler_reset (_fun _pointer -> _void))
(define-rival rival_profiler_aggregate (_fun _pointer _uint32 -> _profile-summary))
(define-rival rival_profiler_executions
              (_fun _pointer (out : (_ptr o _size)) -> (ptr : _pointer) -> (values ptr out)))

(define-rival rival_instruction_names
              (_fun _pointer (out : (_ptr o _size)) -> (ptr : _pointer) -> (values ptr out)))

(let ([v (rival_version)])
  (unless (= v 2)
    (error 'rival3 "ABI version mismatch: expected 2, got ~a" v)))

(struct machine-wrapper
        ([ptr #:mutable] n-vars n-exprs n-instrs discs arg-buf arg-bfs out-buf out-bfs rect-buf
                         rect-bfs name-table)
  #:property prop:cpointer
  (lambda (wrapper) (machine-wrapper-ptr wrapper)))

(struct hints-wrapper ([ptr #:mutable] len)
  #:property prop:cpointer
  (lambda (wrapper) (hints-wrapper-ptr wrapper)))

(define rival-machine? machine-wrapper?)
(define rival-hints? hints-wrapper?)

(define discretization/c
  (struct/c discretization (or/c 'bool 'f32 'f64) exact-positive-integer? procedure?))

(define compile/c
  (c:->i ([exprs list?]
          [vars (listof symbol?)]
          [discs (exprs)
                 (and/c (listof discretization/c)
                        (lambda (discs) (= (length discs) (length exprs))))])
         [machine rival-machine?]))

(define profile-key/c (or/c 'instructions 'iterations 'bumps 'executions 'summary))

(define (machine-destroy wrapper)
  (define ptr (machine-wrapper-ptr wrapper))
  (when ptr
    (set-machine-wrapper-ptr! wrapper #f)
    (rival_machine_free ptr)
    (free-ptr (machine-wrapper-arg-buf wrapper))
    (free-ptr (machine-wrapper-out-buf wrapper))
    (free-ptr (machine-wrapper-rect-buf wrapper))))

(define (hints-destroy wrapper)
  (define ptr (hints-wrapper-ptr wrapper))
  (when ptr
    (set-hints-wrapper-ptr! wrapper #f)
    (rival_hints_free ptr)))

(define (bytes-from-ptr ptr len)
  (define b (make-bytes len))
  (memcpy b ptr len)
  b)

(define (malloc-c-string str)
  (define bs (string->bytes/utf-8 str))
  (define n (bytes-length bs))
  (define ptr (malloc _byte (+ n 1) 'raw))
  (for ([i (in-range n)])
    (ptr-set! ptr _byte i (bytes-ref bs i)))
  (ptr-set! ptr _byte n 0)
  ptr)

(define (free-c-string-array arr n)
  (for ([i (in-range n)])
    (define ptr (ptr-ref arr _pointer i))
    (when ptr (free-ptr ptr)))
  (free-ptr arr))

;; Unary operators are named the same way in Rival expressions and in the ABI.
(define unary-ops
  (for/seteq ([entry (in-list unary-op-codes)]
              #:when (and (symbol? entry) (not (eq? entry '=))))
    entry))

(define binary-ops
  (hasheq '+ 'add
          '- 'sub
          '* 'mul
          '/ 'div
          'pow 'pow
          'hypot 'hypot
          'fmin 'fmin
          'fmax 'fmax
          'fdim 'fdim
          'copysign 'copysign
          'fmod 'fmod
          'remainder 'remainder
          'atan2 'atan2
          'and 'and
          'or 'or
          '== 'eq
          '!= 'ne
          '< 'lt
          '<= 'le
          '> 'gt
          '>= 'ge))

(define variadic-ops (seteq '+ '* 'and 'or))
(define chainable-cmp-ops (seteq '< '<= '> '>=))

(define (make-expr-compiler builder)
  ;; Compile a shared subexpression once, so that a shared input expression
  ;; does not expand into a tree of FFI calls.
  (define cache (make-hasheq))

  (define (fold-binary op args)
    (foldl (lambda (arg acc) (rival_expr_binary builder op acc (compile arg)))
           (compile (car args))
           (cdr args)))

  ;; Build chained comparisons (< a b c) => (and (< a b) (< b c))
  (define (chain-compare op args)
    (define ffi-args (map compile args))
    (define comparisons
      (for/list ([lhs (in-list ffi-args)]
                 [rhs (in-list (cdr ffi-args))])
        (rival_expr_binary builder op lhs rhs)))
    (foldl (lambda (comparison acc) (rival_expr_binary builder 'and acc comparison))
           (car comparisons)
           (cdr comparisons)))

  (define (compile expr)
    (define handle (hash-ref! cache expr (lambda () (compile-node expr))))
    (when (= handle RIVAL_EXPR_INVALID)
      (error 'rival-compile "Could not compile subexpression: ~a" expr))
    handle)

  (define (compile-node expr)
    (match expr
      [(or 'PI '(PI)) (rival_expr_pi builder)]
      [(or 'E '(E)) (rival_expr_e builder)]
      [(or 'TRUE '(TRUE)) (rival_expr_f64 builder 1.0)]
      [(or 'FALSE '(FALSE)) (rival_expr_f64 builder 0.0)]
      [(or 'INFINITY '(INFINITY)) (rival_expr_f64 builder +inf.0)]
      [(or 'NAN '(NAN)) (rival_expr_f64 builder +nan.0)]
      [(? symbol?) (rival_expr_var builder (symbol->string expr))]
      [(? exact-integer?)
       (if (exactly-representable-at-current-bf-precision? expr)
           (rival_expr_bigint builder (number->string expr))
           (rival_expr_bigrational builder (number->string expr) "1"))]
      [(? rational?)
       (define exact-val (inexact->exact expr))
       (if (integer? expr)
           (rival_expr_bigint builder (number->string exact-val))
           (rival_expr_bigrational builder
                                   (number->string (numerator exact-val))
                                   (number->string (denominator exact-val))))]
      [(? real?) (rival_expr_f64 builder (exact->inexact expr))]
      [`(- ,x) (rival_expr_unary builder 'neg (compile x))]
      [`((sinu ,n) ,x) (rival_expr_unary_param builder 'sinu n (compile x))]
      [`((cosu ,n) ,x) (rival_expr_unary_param builder 'cosu n (compile x))]
      [`((tanu ,n) ,x) (rival_expr_unary_param builder 'tanu n (compile x))]
      [`(fma ,a ,b ,c) (rival_expr_ternary builder 'fma (compile a) (compile b) (compile c))]
      [`(if ,c ,t ,f) (rival_expr_ternary builder 'if (compile c) (compile t) (compile f))]
      [`(,op ,x)
       #:when (set-member? unary-ops op)
       (rival_expr_unary builder op (compile x))]
      [`(,op ,x ,y)
       #:when (hash-ref binary-ops op #f)
       (rival_expr_binary builder (hash-ref binary-ops op) (compile x) (compile y))]
      [`(,op ,x ,y ,rest ...)
       #:when (set-member? variadic-ops op)
       (fold-binary (hash-ref binary-ops op) (list* x y rest))]
      [`(,op ,x ,y ,rest ...)
       #:when (set-member? chainable-cmp-ops op)
       (chain-compare (hash-ref binary-ops op) (list* x y rest))]
      [_ (error 'rival-compile "Unknown expression: ~a" expr)]))

  compile)

(define (disc->ffi disc)
  (case (discretization-type disc)
    [(bool) (rival_disc_bool)]
    [(f32) (rival_disc_f32 (discretization-target disc))]
    [(f64) (rival_disc_f64 (discretization-target disc))]))

(define (discs->ffi discs)
  (cond
    [(null? discs) (rival_disc_f64 53)]
    [(= (length discs) 1) (disc->ffi (car discs))]
    [else
     (define target (discretization-target (car discs)))
     (unless (andmap (lambda (d) (= (discretization-target d) target)) (cdr discs))
       (error 'rival-compile "All discretizations must have the same target"))
     (define n (length discs))
     (define types-arr (malloc _rival-disc-type n 'raw))
     (for ([i (in-range n)]
           [d (in-list discs)])
       (ptr-set! types-arr _rival-disc-type i (discretization-type d)))
     (define disc-ptr (rival_disc_mixed types-arr n target))
     (free-ptr types-arr)
     disc-ptr]))

(define (rival-compile exprs vars discs)
  (define n-vars (length vars))
  (define n-exprs (length exprs))
  (define max-precision (*rival-max-precision*))

  (define vars-arr (malloc _pointer n-vars 'raw))
  (for ([i (in-naturals)]
        [var (in-list vars)])
    (ptr-set! vars-arr _pointer i (malloc-c-string (symbol->string var))))
  (define builder (rival_expr_builder_new vars-arr n-vars))
  (free-c-string-array vars-arr n-vars)
  (unless builder
    (error 'rival-compile "Failed to create expression builder"))

  (define machine-ptr
    (dynamic-wind void
                  (lambda ()
                    (define compile-expr (make-expr-compiler builder))
                    (define expr-handles (map compile-expr exprs))
                    (define disc-ptr (discs->ffi discs))
                    (define exprs-arr (malloc _uint32 n-exprs 'raw))
                    (for ([i (in-naturals)]
                          [handle (in-list expr-handles)])
                      (ptr-set! exprs-arr _uint32 i handle))
                    (begin0 (rival_machine_new builder
                                               exprs-arr
                                               n-exprs
                                               disc-ptr
                                               max-precision
                                               (*rival-profile-executions*))
                            (free-ptr exprs-arr)
                            (rival_disc_free disc-ptr)))
                  (lambda () (rival_expr_builder_free builder))))

  (unless machine-ptr
    (error 'rival-compile "Failed to create machine"))

  (define arg-buf (malloc _pointer n-vars 'raw))
  (define out-buf (malloc _pointer n-exprs 'raw))
  (define rect-buf (malloc _pointer (* 2 n-vars) 'raw))

  (define arg-bfs (make-vector n-vars #f))
  (define rect-bfs (make-vector (* 2 n-vars) #f))

  (define out-bfs
    (parameterize ([bf-precision max-precision])
      (build-vector n-exprs (lambda (_) (bf 0.0)))))
  (for ([i (in-range n-exprs)]
        [x (in-vector out-bfs)])
    (ptr-set! out-buf _mpfr-pointer i x))

  (define-values (names-ptr names-len) (rival_instruction_names machine-ptr))
  (define name-table
    (if (and names-ptr (> names-len 0))
        (list->vector (string-split (bytes->string/utf-8 (bytes-from-ptr names-ptr names-len)) "\0"))
        (vector)))

  (define wrapper
    (machine-wrapper machine-ptr
                     n-vars
                     n-exprs
                     (rival_machine_instruction_count machine-ptr)
                     discs
                     arg-buf
                     arg-bfs
                     out-buf
                     out-bfs
                     rect-buf
                     rect-bfs
                     name-table))
  (register-finalizer wrapper machine-destroy)
  wrapper)

(define (baseline-compile exprs vars discs)
  (define machine (rival-compile exprs vars discs))
  (unless (rival_machine_configure_baseline (machine-wrapper-ptr machine))
    (error 'baseline-compile "Failed to configure baseline machine"))
  machine)

(define (native-apply machine args n-args outs n-outs hints require-all?)
  (rival_apply machine args n-args outs n-outs hints (*rival-max-iterations*) require-all?))

(define (apply-inner machine pt hints ffi-fn require-all? error-name)
  (define n-args (vector-length pt))
  (unless (= n-args (machine-wrapper-n-vars machine))
    (raise-arguments-error error-name
                           "point has the wrong number of variables"
                           "expected"
                           (machine-wrapper-n-vars machine)
                           "given"
                           n-args))
  (when (and hints (not (= (hints-wrapper-len hints) (machine-wrapper-n-instrs machine))))
    (raise-arguments-error error-name
                           "hints do not belong to this machine"
                           "expected"
                           (machine-wrapper-n-instrs machine)
                           "given"
                           (hints-wrapper-len hints)))
  (define arg-ptrs (machine-wrapper-arg-buf machine))
  (define arg-bfs (machine-wrapper-arg-bfs machine))
  (for ([i (in-range n-args)]
        [arg (in-vector pt)])
    (define x (input->bf arg))
    (vector-set! arg-bfs i x)
    (ptr-set! arg-ptrs _mpfr-pointer i x))
  (define n-outs (machine-wrapper-n-exprs machine))
  (define out-bfs (machine-wrapper-out-bfs machine))
  (define out-ptrs (machine-wrapper-out-buf machine))
  (define hints-ptr (and hints (hints-wrapper-ptr hints)))
  (define status-code
    (ffi-fn (machine-wrapper-ptr machine) arg-ptrs n-args out-ptrs n-outs hints-ptr require-all?))
  (match status-code
    ['ok
     (define discs (machine-wrapper-discs machine))
     (for/vector #:length n-outs
                 ([bf (in-vector out-bfs)]
                  [disc (in-list discs)])
       (if (bfnan? bf)
           'invalid
           ((discretization-convert disc) bf)))]
    ['invalid_input (raise (exn:rival:invalid "Invalid input" (current-continuation-marks) pt))]
    ['unsamplable (raise (exn:rival:unsamplable "Unsamplable input" (current-continuation-marks) pt))]
    [else (error error-name "Unknown result code: ~a" status-code)]))

(define (rival-apply machine pt [hints #f])
  (apply-inner machine pt hints native-apply #t 'rival-apply))

(define (rival-apply/partial machine pt [hints #f])
  (apply-inner machine pt hints native-apply #f 'rival-apply/partial))

(define (baseline-apply machine pt [hints #f])
  (apply-inner machine pt hints rival_apply_baseline #t 'baseline-apply))

(define (baseline-apply/partial machine pt [hints #f])
  (apply-inner machine pt hints rival_apply_baseline #f 'baseline-apply/partial))

(define (analyze-inner machine rect hint ffi-fn require-all? keep-hints? error-name)
  (define n-args (vector-length rect))
  (unless (= n-args (machine-wrapper-n-vars machine))
    (raise-arguments-error error-name
                           "rectangle has the wrong number of variables"
                           "expected"
                           (machine-wrapper-n-vars machine)
                           "given"
                           n-args))
  (when (and hint (not (= (hints-wrapper-len hint) (machine-wrapper-n-instrs machine))))
    (raise-arguments-error error-name
                           "hints do not belong to this machine"
                           "expected"
                           (machine-wrapper-n-instrs machine)
                           "given"
                           (hints-wrapper-len hint)))
  (define rect-ptrs (machine-wrapper-rect-buf machine))
  (define rect-bfs (machine-wrapper-rect-bfs machine))
  (for ([i (in-range n-args)]
        [iv (in-vector rect)])
    (define lo (input->bf (ival-lo iv)))
    (define hi (input->bf (ival-hi iv)))
    (vector-set! rect-bfs (* 2 i) lo)
    (vector-set! rect-bfs (+ (* 2 i) 1) hi)
    (ptr-set! rect-ptrs _mpfr-pointer (* 2 i) lo)
    (ptr-set! rect-ptrs _mpfr-pointer (+ (* 2 i) 1) hi))
  (define hint-ptr (and hint (hints-wrapper-ptr hint)))
  (match-define (list status-code is-error maybe-error converged hints-ptr)
    (ffi-fn (machine-wrapper-ptr machine) rect-ptrs n-args hint-ptr require-all?))
  (match status-code
    ['ok (void)]
    [else (error error-name "Unknown result code: ~a" status-code)])
  (define new-hints
    (cond
      [(not hints-ptr) #f]
      [keep-hints?
       (define wrapper (hints-wrapper hints-ptr (rival_hints_len hints-ptr)))
       (register-finalizer wrapper hints-destroy)
       wrapper]
      [else
       (rival_hints_free hints-ptr)
       #f]))
  (list (ival is-error maybe-error) new-hints converged))

(define (rival-analyze-with-hints machine rect [hint #f])
  (analyze-inner machine rect hint rival_analyze_with_hints #t #t 'rival-analyze-with-hints))

(define (rival-analyze-with-hints/partial machine rect [hint #f])
  (analyze-inner
   machine rect hint rival_analyze_with_hints #f #t 'rival-analyze-with-hints/partial))

(define (rival-analyze machine rect)
  (car (analyze-inner machine rect #f rival_analyze_with_hints #t #f 'rival-analyze)))

(define (rival-analyze/partial machine rect)
  (car (analyze-inner machine rect #f rival_analyze_with_hints #f #f 'rival-analyze/partial)))

(define (baseline-analyze-with-hints machine rect [hint #f])
  (analyze-inner
   machine rect hint rival_analyze_baseline_with_hints #t #t 'baseline-analyze-with-hints))

(define (baseline-analyze-with-hints/partial machine rect [hint #f])
  (analyze-inner
   machine rect hint rival_analyze_baseline_with_hints #f #t 'baseline-analyze-with-hints/partial))

(define (baseline-analyze machine rect)
  (car (analyze-inner machine rect #f rival_analyze_baseline_with_hints #t #f 'baseline-analyze)))

(define (baseline-analyze/partial machine rect)
  (car (analyze-inner
        machine rect #f rival_analyze_baseline_with_hints #f #f 'baseline-analyze/partial)))

(define (instruction-name names instr-idx)
  (cond
    [(negative? instr-idx) "adjust"]
    [(< instr-idx (vector-length names)) (vector-ref names instr-idx)]
    [else ""]))

(define (rival-profile machine param)
  (match param
    ['instructions (rival_machine_instruction_count (machine-wrapper-ptr machine))]
    ['iterations (rival_machine_iterations (machine-wrapper-ptr machine))]
    ['bumps (rival_machine_bumps (machine-wrapper-ptr machine))]
    ['executions
     (define-values (ptr len) (rival_profiler_executions (machine-wrapper-ptr machine)))
     (cond
       [(or (not ptr) (zero? len)) (vector)]
       [else
        (define names (machine-wrapper-name-table machine))
        (for/vector #:length len
                    ([i (in-range len)])
          (define rec-ptr (ptr-add ptr (* i execution-record-size)))
          (match-define (list instr-idx prec time-ms iter) (ptr-ref rec-ptr _execution-record))
          (execution (instruction-name names instr-idx) instr-idx prec time-ms 0 iter))])]
    ['summary
     (define bucket-size (max 1 (quotient (*rival-max-precision*) 25)))
     (match-define (list entries-ptr entries-len bumps iterations)
       (rival_profiler_aggregate (machine-wrapper-ptr machine) bucket-size))
     (define names (machine-wrapper-name-table machine))
     (define summary
       (if (or (not entries-ptr) (zero? entries-len))
           (vector)
           (for/vector #:length entries-len
                       ([i (in-range entries-len)])
             (define entry-ptr (ptr-add entries-ptr (* i aggregated-entry-size)))
             (match-define (list instr-idx prec-bucket time-ms count)
               (ptr-ref entry-ptr _aggregated-entry))
             (list (instruction-name names instr-idx) prec-bucket time-ms count))))
     (list summary bumps iterations)]))

(define (rival-set-profiling! machine enabled)
  (rival_machine_set_profiling (machine-wrapper-ptr machine) (if enabled 'on 'off)))

(define (rival-profiling-enabled? machine)
  (eq? (rival_machine_get_profiling (machine-wrapper-ptr machine)) 'on))

(define (free-ptr p)
  (when p (free p)))
