#lang racket

(require ffi/unsafe
         ffi/unsafe/define
         racket/runtime-path
         math/bigfloat
         math/flonum
         (only-in math/private/bigfloat/mpfr _mpfr-pointer)
         "ops.rkt")

(provide rival-compile
         rival-apply
         rival-machine-find-optimal-precisions
         rival-machine-optimal-precision
         baseline-compile
         baseline-apply
         rival-analyze-with-hints
         rival-analyze
         baseline-analyze-with-hints
         baseline-analyze
         rival-profile
         rival-set-profiling!
         rival-profiling-enabled?
         (struct-out exn:rival)
         (struct-out exn:rival:invalid)
         (struct-out exn:rival:unsamplable)
         (struct-out execution)
         (struct-out discretization)
         (struct-out ival)
         boolean-discretization
         flonum-discretization
         rival-machine?
         rival-hints?
         *rival-max-precision*
         *rival-max-iterations*  ;; TODO: to delete
         *rival-profile-executions*
         execution-name
         execution-number
         execution-precision
         execution-time
         execution-memory
         execution-iteration
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
(define *rival-max-iterations* (make-parameter 5))  ;; TODO: to delete
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
(define _optimal-precision-result (_list-struct _rival-error _pointer _size))
(define _profile-summary (_list-struct _pointer _size _uint32 _uint32))
(define _execution-record (_list-struct _int32 _uint32 _double _uint32))
(define execution-record-size (ctype-sizeof _execution-record))
(define _aggregated-entry (_list-struct _int32 _uint32 _double _size))
(define aggregated-entry-size (ctype-sizeof _aggregated-entry))

(define _rival-profiling-mode (_enum '(off = 0 on = 1) _uint32))
(define _rival-disc-type (_enum '(bool = 0 f32 = 1 f64 = 2) _uint32))

(define-rival rival_version (_fun -> _uint32))
(define-rival rival_disc_f64 (_fun _uint32 -> _pointer))
(define-rival rival_disc_f32 (_fun _uint32 -> _pointer))
(define-rival rival_disc_bool (_fun -> _pointer))
(define-rival rival_disc_mixed (_fun _pointer _size _uint32 -> _pointer))
(define-rival rival_disc_free (_fun _pointer -> _void))

(define-rival rival_expr_arena_new (_fun -> _pointer))
(define-rival rival_expr_arena_free (_fun _pointer -> _void))

(define-rival rival_expr_var (_fun _pointer _string -> _uint32))
(define-rival rival_expr_f64 (_fun _pointer _double -> _uint32))
(define-rival rival_expr_rational (_fun _pointer _int64 _int64 -> _uint32))
(define-rival rival_expr_bigint (_fun _pointer _string -> _uint32))
(define-rival rival_expr_bigrational (_fun _pointer _string _string -> _uint32))
(define-rival rival_expr_pi (_fun _pointer -> _uint32))
(define-rival rival_expr_e (_fun _pointer -> _uint32))

(define-rival rival_expr_neg (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_fabs (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_sqrt (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_cbrt (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_pow2 (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_exp (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_exp2 (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_expm1 (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_log (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_log2 (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_log10 (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_log1p (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_logb (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_sin (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_cos (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_tan (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_asin (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_acos (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_atan (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_sinh (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_cosh (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_tanh (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_asinh (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_acosh (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_atanh (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_erf (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_erfc (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_lgamma (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_tgamma (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_rint (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_round (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_ceil (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_floor (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_trunc (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_not (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_assert (_fun _pointer _uint32 -> _uint32))
(define-rival rival_expr_error (_fun _pointer _uint32 -> _uint32))

(define-rival rival_expr_add (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_sub (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_mul (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_div (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_pow (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_hypot (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_fmin (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_fmax (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_fdim (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_copysign (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_fmod (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_remainder (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_atan2 (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_and (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_or (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_eq (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_ne (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_lt (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_le (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_gt (_fun _pointer _uint32 _uint32 -> _uint32))
(define-rival rival_expr_ge (_fun _pointer _uint32 _uint32 -> _uint32))

(define-rival rival_expr_sinu (_fun _pointer _uint64 _uint32 -> _uint32))
(define-rival rival_expr_cosu (_fun _pointer _uint64 _uint32 -> _uint32))
(define-rival rival_expr_tanu (_fun _pointer _uint64 _uint32 -> _uint32))

(define-rival rival_expr_fma (_fun _pointer _uint32 _uint32 _uint32 -> _uint32))
(define-rival rival_expr_if (_fun _pointer _uint32 _uint32 _uint32 -> _uint32))

(define-rival rival_machine_new
              (_fun _pointer _pointer _size _pointer _size _pointer _uint32 _size -> _pointer))
(define-rival rival_machine_free (_fun _pointer -> _void))
(define-rival rival_machine_configure_baseline (_fun _pointer -> _stdbool))
(define-rival rival_machine_instruction_count (_fun _pointer -> _size))
(define-rival rival_machine_iterations (_fun _pointer -> _uint32))
(define-rival rival_machine_bumps (_fun _pointer -> _uint32))
(define-rival rival_machine_set_profiling (_fun _pointer _rival-profiling-mode -> _void))
(define-rival rival_machine_get_profiling (_fun _pointer -> _rival-profiling-mode))

(define-rival rival_apply
              (_fun _pointer _pointer _size _pointer _size _pointer _uint32 -> _rival-error))

(define-rival rival_apply_baseline
              (_fun _pointer _pointer _size _pointer _size _pointer _uint32 -> _rival-error))

(define-rival rival_machine_find_optimal_precisions
              (_fun _pointer _pointer -> _optimal-precision-result))

(define-rival rival_analyze_with_hints (_fun _pointer _pointer _size _pointer -> _analyze-result))
(define-rival rival_analyze_baseline_with_hints
              (_fun _pointer _pointer _size _pointer -> _analyze-result))

(define-rival rival_hints_free (_fun _pointer -> _void))
(define-rival rival_hints_len (_fun _pointer -> _size))

(define-rival rival_profiler_reset (_fun _pointer -> _void))
(define-rival rival_profiler_aggregate (_fun _pointer _uint32 -> _profile-summary))
(define-rival rival_profiler_executions
              (_fun _pointer (out : (_ptr o _size)) -> (ptr : _pointer) -> (values ptr out)))

(define-rival rival_instruction_names
              (_fun _pointer (out : (_ptr o _size)) -> (ptr : _pointer) -> (values ptr out)))

(let ([v (rival_version)])
  (unless (= v 3)
    (error 'rival3 "ABI version mismatch: expected 3, got ~a" v)))

(struct machine-wrapper (ptr n-vars n-exprs discs arg-buf arg-bfs out-buf out-bfs rect-buf name-table)
  #:property prop:cpointer
  (struct-field-index ptr))

(struct hints-wrapper (ptr) #:property prop:cpointer (struct-field-index ptr))

(define rival-machine? machine-wrapper?)
(define rival-hints? hints-wrapper?)

(define (machine-destroy wrapper)
  (when (machine-wrapper-ptr wrapper)
    (rival_machine_free (machine-wrapper-ptr wrapper)))
  (free-ptr (machine-wrapper-arg-buf wrapper))
  (free-ptr (machine-wrapper-out-buf wrapper))
  (free-ptr (machine-wrapper-rect-buf wrapper)))

(define (hints-destroy wrapper)
  (when (hints-wrapper-ptr wrapper)
    (rival_hints_free (hints-wrapper-ptr wrapper))))

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

;; Fold a list of args using a binary FFI function
(define (fold-binary-ffi arena binary-fn args)
  (foldl (lambda (arg acc) (binary-fn arena acc (expr->ffi arena arg)))
         (expr->ffi arena (car args))
         (cdr args)))

;; Build chained comparisons (< a b c) => (and (< a b) (< b c))
(define (chain-compare-ffi arena cmp-fn args)
  (define ffi-args (map (lambda (a) (expr->ffi arena a)) args))
  (define comparisons
    (for/list ([i (in-range (sub1 (length ffi-args)))])
      (cmp-fn arena (list-ref ffi-args i) (list-ref ffi-args (add1 i)))))
  (foldl (lambda (cmp acc) (rival_expr_and arena acc cmp)) (car comparisons) (cdr comparisons)))

(define unary-ops
  (hasheq 'neg rival_expr_neg
          'fabs rival_expr_fabs
          'sqrt rival_expr_sqrt
          'cbrt rival_expr_cbrt
          'pow2 rival_expr_pow2
          'exp rival_expr_exp
          'exp2 rival_expr_exp2
          'expm1 rival_expr_expm1
          'log rival_expr_log
          'log2 rival_expr_log2
          'log10 rival_expr_log10
          'log1p rival_expr_log1p
          'logb rival_expr_logb
          'sin rival_expr_sin
          'cos rival_expr_cos
          'tan rival_expr_tan
          'asin rival_expr_asin
          'acos rival_expr_acos
          'atan rival_expr_atan
          'sinh rival_expr_sinh
          'cosh rival_expr_cosh
          'tanh rival_expr_tanh
          'asinh rival_expr_asinh
          'acosh rival_expr_acosh
          'atanh rival_expr_atanh
          'erf rival_expr_erf
          'erfc rival_expr_erfc
          'lgamma rival_expr_lgamma
          'tgamma rival_expr_tgamma
          'rint rival_expr_rint
          'round rival_expr_round
          'ceil rival_expr_ceil
          'floor rival_expr_floor
          'trunc rival_expr_trunc
          'not rival_expr_not
          'assert rival_expr_assert
          'error rival_expr_error))

(define binary-ops
  (hasheq '+ rival_expr_add
          '- rival_expr_sub
          '* rival_expr_mul
          '/ rival_expr_div
          'pow rival_expr_pow
          'hypot rival_expr_hypot
          'fmin rival_expr_fmin
          'fmax rival_expr_fmax
          'fdim rival_expr_fdim
          'copysign rival_expr_copysign
          'fmod rival_expr_fmod
          'remainder rival_expr_remainder
          'atan2 rival_expr_atan2
          'and rival_expr_and
          'or rival_expr_or
          '== rival_expr_eq
          '!= rival_expr_ne
          '< rival_expr_lt
          '<= rival_expr_le
          '> rival_expr_gt
          '>= rival_expr_ge))

(define variadic-ops (hasheq '+ rival_expr_add '* rival_expr_mul 'and rival_expr_and 'or rival_expr_or))
(define chainable-cmp-ops (hasheq '< rival_expr_lt '<= rival_expr_le '> rival_expr_gt '>= rival_expr_ge))

(define (expr->ffi arena expr)
  (match expr
    [(or 'PI '(PI)) (rival_expr_pi arena)]
    [(or 'E '(E)) (rival_expr_e arena)]
    [(or 'TRUE '(TRUE)) (rival_expr_f64 arena 1.0)]
    [(or 'FALSE '(FALSE)) (rival_expr_f64 arena 0.0)]
    [(or 'INFINITY '(INFINITY)) (rival_expr_f64 arena +inf.0)]
    [(or 'NAN '(NAN)) (rival_expr_f64 arena +nan.0)]
    [(? symbol?) (rival_expr_var arena (symbol->string expr))]
    [(? exact-integer?)
     (if (exactly-representable-at-current-bf-precision? expr)
         (rival_expr_bigint arena (number->string expr))
         (rival_expr_bigrational arena (number->string expr) "1"))]
    [(? rational?)
     (define exact-val (inexact->exact expr))
     (if (integer? expr)
         (rival_expr_bigint arena (number->string exact-val))
         (rival_expr_bigrational arena
                                 (number->string (numerator exact-val))
                                 (number->string (denominator exact-val))))]
    [(? real?) (rival_expr_f64 arena (exact->inexact expr))]
    [`(- ,x) (rival_expr_neg arena (expr->ffi arena x))]
    [`((sinu ,n) ,x) (rival_expr_sinu arena n (expr->ffi arena x))]
    [`((cosu ,n) ,x) (rival_expr_cosu arena n (expr->ffi arena x))]
    [`((tanu ,n) ,x) (rival_expr_tanu arena n (expr->ffi arena x))]
    [`(fma ,a ,b ,c)
     (rival_expr_fma arena (expr->ffi arena a) (expr->ffi arena b) (expr->ffi arena c))]
    [`(if ,c ,t ,f)
     (rival_expr_if arena (expr->ffi arena c) (expr->ffi arena t) (expr->ffi arena f))]
    [`(,op ,x)
     #:when (hash-ref unary-ops op #f)
     ((hash-ref unary-ops op) arena (expr->ffi arena x))]
    [`(,op ,x ,y)
     #:when (hash-ref binary-ops op #f)
     ((hash-ref binary-ops op) arena (expr->ffi arena x) (expr->ffi arena y))]
    [`(,op ,x ,y ,rest ...)
     #:when (hash-ref variadic-ops op #f)
     (fold-binary-ffi arena (hash-ref variadic-ops op) (list* x y rest))]
    [`(,op ,x ,y ,rest ...)
     #:when (hash-ref chainable-cmp-ops op #f)
     (chain-compare-ffi arena (hash-ref chainable-cmp-ops op) (list* x y rest))]
    [_ (error 'expr->ffi "Unknown expression: ~a" expr)]))

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

  (define arena (rival_expr_arena_new))
  (define expr-handles (map (lambda (e) (expr->ffi arena e)) exprs))
  (define exprs-arr (malloc _uint32 n-exprs 'raw))
  (for ([i (in-naturals)]
        [handle (in-list expr-handles)])
    (ptr-set! exprs-arr _uint32 i handle))

  (define vars-arr (malloc _pointer n-vars 'raw))
  (for ([i (in-naturals)]
        [var (in-list vars)])
    (ptr-set! vars-arr _pointer i (malloc-c-string (symbol->string var))))

  (define disc-ptr (discs->ffi discs))

  (define machine-ptr
    (rival_machine_new arena
                       exprs-arr
                       n-exprs
                       vars-arr
                       n-vars
                       disc-ptr
                       (*rival-max-precision*)
                       (*rival-profile-executions*)))

  (free-ptr exprs-arr)
  (free-c-string-array vars-arr n-vars)
  (rival_disc_free disc-ptr)
  (rival_expr_arena_free arena)

  (when (not machine-ptr)
    (error 'rival-compile "Failed to create machine"))

  (define arg-buf (malloc _pointer n-vars 'raw))
  (define out-buf (malloc _pointer n-exprs 'raw))
  (define rect-buf (malloc _pointer (* 2 n-vars) 'raw))

  (define arg-bfs
    (build-vector n-vars
                  (lambda (_)
                    (parameterize ([bf-precision 53])
                      (bf 0.0)))))
  (for ([i (in-range n-vars)]
        [bf (in-vector arg-bfs)])
    (ptr-set! arg-buf _mpfr-pointer i bf))

  (define out-bfs
    (build-vector n-exprs
                  (lambda (_)
                    (parameterize ([bf-precision (*rival-max-precision*)])
                      (bf 0.0)))))
  (for ([i (in-range n-exprs)]
        [bf (in-vector out-bfs)])
    (ptr-set! out-buf _mpfr-pointer i bf))

  (define-values (names-ptr names-len) (rival_instruction_names machine-ptr))
  (define name-table
    (if (and names-ptr (> names-len 0))
        (list->vector (string-split (bytes->string/utf-8 (bytes-from-ptr names-ptr names-len)) "\0"))
        (vector)))

  (define wrapper
    (machine-wrapper machine-ptr
                     n-vars
                     n-exprs
                     discs
                     arg-buf
                     arg-bfs
                     out-buf
                     out-bfs
                     rect-buf
                     name-table))
  (register-finalizer wrapper machine-destroy)
  wrapper)

(define (baseline-compile exprs vars discs)
  (define machine (rival-compile exprs vars discs))
  (unless (rival_machine_configure_baseline (machine-wrapper-ptr machine))
    (error 'baseline-compile "Failed to configure baseline machine"))
  machine)

(define (apply-inner machine pt hints call-fn error-name)
  (define n-args (vector-length pt))
  (define arg-ptrs (machine-wrapper-arg-buf machine))
  (when (> n-args 0)
    (for ([i (in-range n-args)]
          [arg (in-vector pt)])
      (ptr-set! arg-ptrs _mpfr-pointer i (input->bf arg))))
  (define n-outs (machine-wrapper-n-exprs machine))
  (define out-bfs (machine-wrapper-out-bfs machine))
  (define out-ptrs (machine-wrapper-out-buf machine))
  (define hints-ptr (and hints (hints-wrapper-ptr hints)))
  (define args-to-pass (and (> n-args 0) arg-ptrs))
  (define status-code
    (call-fn (machine-wrapper-ptr machine) args-to-pass n-args out-ptrs n-outs hints-ptr))
  (match status-code
    ['ok
     (define discs (machine-wrapper-discs machine))
     (for/vector #:length n-outs
                 ([bf (in-vector out-bfs)]
                  [disc (in-list discs)])
       (if (bfnan? bf)
           (raise (exn:rival:invalid "Invalid output" (current-continuation-marks) pt))
           ((discretization-convert disc) bf)))]
    ['invalid_input (raise (exn:rival:invalid "Invalid input" (current-continuation-marks) pt))]
    ['unsamplable (raise (exn:rival:unsamplable "Unsamplable input" (current-continuation-marks) pt))]
    [else (error error-name "Unknown result code: ~a" status-code)]))

(define (rival-apply machine pt [hints #f])
  (apply-inner machine
               pt
               hints
               (lambda (m a na o no h)
                 (rival_apply m a na o no h (*rival-max-precision*)))
               'rival-apply))

(define (rival-machine-find-optimal-precisions machine pt)
  (define n-args (vector-length pt))
  (unless (= n-args (machine-wrapper-n-vars machine))
    (raise (exn:rival:invalid "Invalid input" (current-continuation-marks) pt)))
  
  (define arg-ptrs (machine-wrapper-arg-buf machine))
  (when (> n-args 0)
    (for ([i (in-range n-args)]
          [arg (in-vector pt)])
      (ptr-set! arg-ptrs _mpfr-pointer i (input->bf arg))))
  
  (match-define (list status optimal-ptr optimal-len)
    (rival_machine_find_optimal_precisions
     (machine-wrapper-ptr machine)
     (and (> n-args 0) arg-ptrs)))
  (match status
    ['ok
     (if (or (not optimal-ptr) (zero? optimal-len))
         (vector)
         (for/vector #:length optimal-len
                     ([i (in-range optimal-len)])
           (ptr-ref optimal-ptr _uint32 i)))]
    ['invalid_input (raise (exn:rival:invalid "Invalid input" (current-continuation-marks) pt))]
    ['unsamplable (raise (exn:rival:unsamplable "Unsamplable input" (current-continuation-marks) pt))]
    [else (error 'rival-machine-find-optimal-precisions "Unknown result code: ~a" status)]))

(define rival-machine-optimal-precision rival-machine-find-optimal-precisions)

(define (baseline-apply machine pt [hints #f])
  (apply-inner machine
               pt
               hints
               (lambda (m a na o no h) (rival_apply_baseline m a na o no h (*rival-max-precision*)))
               'baseline-apply))

(define (analyze-inner machine rect hint ffi-fn error-name)
  (define n-args (vector-length rect))
  (define rect-ptrs (machine-wrapper-rect-buf machine))
  (for ([i (in-range n-args)]
        [iv (in-vector rect)])
    (ptr-set! rect-ptrs _mpfr-pointer (* 2 i) (input->bf (ival-lo iv)))
    (ptr-set! rect-ptrs _mpfr-pointer (+ (* 2 i) 1) (input->bf (ival-hi iv))))
  (define hint-ptr (and hint (hints-wrapper-ptr hint)))
  (match-define (list status-code is-error maybe-error converged hints-ptr)
    (ffi-fn (machine-wrapper-ptr machine) rect-ptrs n-args hint-ptr))
  (match status-code
    ['invalid_input (raise (exn:rival:invalid "Invalid input" (current-continuation-marks) rect))]
    ['ok (void)]
    [else (error error-name "Unknown result code: ~a" status-code)])
  (define new-hints
    (if hints-ptr
        (let ([wrapper (hints-wrapper hints-ptr)])
          (register-finalizer wrapper hints-destroy)
          wrapper)
        #f))
  (list (ival is-error maybe-error) new-hints converged))

(define (rival-analyze-with-hints machine rect [hint #f])
  (analyze-inner machine rect hint rival_analyze_with_hints 'rival-analyze-with-hints))

(define (rival-analyze machine rect)
  (car (rival-analyze-with-hints machine rect)))

(define (baseline-analyze-with-hints machine rect [hint #f])
  (analyze-inner machine rect hint rival_analyze_baseline_with_hints 'baseline-analyze-with-hints))

(define (baseline-analyze machine rect)
  (car (baseline-analyze-with-hints machine rect)))

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
          (define name
            (cond
              [(and (vector? names) (<= 0 instr-idx) (< instr-idx (vector-length names)))
               (vector-ref names instr-idx)]
              [(< instr-idx 0) "adjust"]
              [else ""]))
          (execution name instr-idx prec time-ms 0 iter))])]
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
             (define name
               (cond
                 [(and (vector? names) (<= 0 instr-idx) (< instr-idx (vector-length names)))
                  (vector-ref names instr-idx)]
                 [(< instr-idx 0) "adjust"]
                 [else ""]))
             (list name prec-bucket time-ms count))))
     (list summary bumps iterations)]))

(define (rival-set-profiling! machine enabled)
  (rival_machine_set_profiling (machine-wrapper-ptr machine) (if enabled 'on 'off)))

(define (rival-profiling-enabled? machine)
  (eq? (rival_machine_get_profiling (machine-wrapper-ptr machine)) 'on))

(define (free-ptr p)
  (when p (free p)))
