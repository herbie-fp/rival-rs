#lang racket/base

(require racket/match
         racket/dict)

(provide rival-functions
         rival-type
         rival-types)

(define rival-types '(bool real))

(define (make-op otype itypes)
  (cons otype itypes))

(define rival-functions
  (hash 'PI
        (make-op 'real '())
        'E
        (make-op 'real '())
        'TRUE
        (make-op 'bool '())
        'FALSE
        (make-op 'bool '())
        'INFINITY
        (make-op 'real '())
        'NAN
        (make-op 'real '())
        '+
        (make-op 'real '(real real))
        '-
        (make-op 'real '(real real))
        '*
        (make-op 'real '(real real))
        '/
        (make-op 'real '(real real))
        'neg
        (make-op 'real '(real))
        'sqrt
        (make-op 'real '(real))
        'cbrt
        (make-op 'real '(real))
        'pow
        (make-op 'real '(real real))
        'pow2
        (make-op 'real '(real))
        'exp
        (make-op 'real '(real))
        'exp2
        (make-op 'real '(real))
        'expm1
        (make-op 'real '(real))
        'log
        (make-op 'real '(real))
        'log2
        (make-op 'real '(real))
        'log10
        (make-op 'real '(real))
        'log1p
        (make-op 'real '(real))
        'logb
        (make-op 'real '(real))
        'sin
        (make-op 'real '(real))
        'cos
        (make-op 'real '(real))
        'tan
        (make-op 'real '(real))
        'asin
        (make-op 'real '(real))
        'acos
        (make-op 'real '(real))
        'atan
        (make-op 'real '(real))
        'atan2
        (make-op 'real '(real real))
        'sinh
        (make-op 'real '(real))
        'cosh
        (make-op 'real '(real))
        'tanh
        (make-op 'real '(real))
        'asinh
        (make-op 'real '(real))
        'acosh
        (make-op 'real '(real))
        'atanh
        (make-op 'real '(real))
        'erf
        (make-op 'real '(real))
        'erfc
        (make-op 'real '(real))
        'lgamma
        (make-op 'real '(real))
        'tgamma
        (make-op 'real '(real))
        'rint
        (make-op 'real '(real))
        'round
        (make-op 'real '(real))
        'ceil
        (make-op 'real '(real))
        'floor
        (make-op 'real '(real))
        'trunc
        (make-op 'real '(real))
        'fmin
        (make-op 'real '(real real))
        'fmax
        (make-op 'real '(real real))
        'fdim
        (make-op 'real '(real real))
        'copysign
        (make-op 'real '(real real))
        'fmod
        (make-op 'real '(real real))
        'remainder
        (make-op 'real '(real real))
        'hypot
        (make-op 'real '(real real))
        'fma
        (make-op 'real '(real real real))
        'fabs
        (make-op 'real '(real))
        'not
        (make-op 'bool '(bool))
        'and
        (make-op 'bool '(bool bool))
        'or
        (make-op 'bool '(bool bool))
        '==
        (make-op 'bool '(real real))
        '!=
        (make-op 'bool '(real real))
        '<
        (make-op 'bool '(real real))
        '>
        (make-op 'bool '(real real))
        '<=
        (make-op 'bool '(real real))
        '>=
        (make-op 'bool '(real real))
        'assert
        (make-op 'bool '(bool))
        'error
        (make-op 'bool '(real))
        'if
        (make-op 'real '(bool real real))))

(define (rival-type expr env)
  (match expr
    [(? number?) 'real]
    [(? symbol?) (dict-ref env expr)]
    [(list 'if c t f)
     (define ct (rival-type c env))
     (define tt (rival-type t env))
     (define ft (rival-type f env))
     (and (equal? ct 'bool) (equal? tt ft) tt)]
    [(list 'error a)
     (define at (rival-type a env))
     (and at 'bool)]
    [(list op args ...)
     (match-define (cons otype itypes) (hash-ref rival-functions op (cons #f '())))
     (and (equal? (length itypes) (length args))
          (andmap equal? itypes (map (lambda (a) (rival-type a env)) args))
          otype)]
    [_ #f]))
