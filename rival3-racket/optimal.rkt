#lang racket

(require ffi/unsafe
         ffi/unsafe/define
         racket/runtime-path
         math/bigfloat
         (only-in math/private/bigfloat/mpfr _mpfr-pointer)
         "main.rkt")

(provide rival-machine-find-optimal-precisions
         rival-machine-optimal-precision)

(define-runtime-path native-root (build-path "private" "native"))
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
(define _optimal-precision-result
  (_list-struct _rival-error
                _stdbool
                _pointer
                _size
                _double
                _pointer
                _size
                _double))

(define-rival rival_version (_fun -> _uint32))
(define-rival rival_machine_find_optimal_precisions
              (_fun _pointer _pointer -> _optimal-precision-result))

(let ([v (rival_version)])
  (unless (= v 3)
    (error 'rival3/optimal "ABI version mismatch: expected 3, got ~a" v)))

(define (rival-machine-find-optimal-precisions machine pt)
  (unless (rival-machine? machine)
    (raise-argument-error 'rival-machine-find-optimal-precisions "rival-machine?" machine))
  (unless (vector? pt)
    (raise-argument-error 'rival-machine-find-optimal-precisions "vector?" pt))

  (define n-args (vector-length pt))
  (define arg-bfs
    (for/vector #:length n-args
                ([arg (in-vector pt)])
      (if (boolean? arg)
          (bf (if arg 1 0))
          arg)))
  (define arg-ptrs (and (> n-args 0) (malloc _pointer n-args 'raw)))

  (define result
    (dynamic-wind
     void
     (lambda ()
       (when arg-ptrs
         (for ([i (in-range n-args)]
               [arg (in-vector arg-bfs)])
           (ptr-set! arg-ptrs _mpfr-pointer i arg)))
       (rival_machine_find_optimal_precisions
        machine
        arg-ptrs))
     (lambda ()
       (when arg-ptrs
         (free arg-ptrs)))))

  (match-define (list status found? optimal-ptr optimal-len optimal-time tuned-ptr tuned-len tuned-time) result)
  (match status
    ['ok
     (and found?
          (list (if (or (not optimal-ptr) (zero? optimal-len))
                    (vector)
                    (for/vector #:length optimal-len
                                ([i (in-range optimal-len)])
                      (ptr-ref optimal-ptr _uint32 i)))
                optimal-time
                (if (or (not tuned-ptr) (zero? tuned-len))
                    (vector)
                    (for/vector #:length tuned-len
                                ([i (in-range tuned-len)])
                      (ptr-ref tuned-ptr _uint32 i)))
                tuned-time))]
    ['invalid_input (raise (exn:rival:invalid "Invalid input" (current-continuation-marks) pt))]
    ['unsamplable (raise (exn:rival:unsamplable "Unsamplable input" (current-continuation-marks) pt))]
    [_ (error 'rival-machine-find-optimal-precisions "Unknown result code: ~a" status)]))

(define rival-machine-optimal-precision rival-machine-find-optimal-precisions)
