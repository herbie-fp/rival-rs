#lang racket

(require json
         math/bigfloat
         rival3)

(define (read-from-string s)
  (read (open-input-string s)))

(define (point-record->point pt*)
  (match pt*
    [(list pt _ ...) pt]
    [pt pt]))

(define (compile-record rec)
  (define exprs (map read-from-string (hash-ref rec 'exprs)))
  (define vars (map read-from-string (hash-ref rec 'vars)))
  (unless (andmap symbol? vars)
    (error 'optimal-search "Invalid variable list ~a" vars))
  (match-define `(bool flonum ...) (map read-from-string (hash-ref rec 'discs)))
  (define discs
    (cons boolean-discretization
          (map (const flonum-discretization) (cdr exprs))))
  (parameterize ([*rival-max-precision* 32256])
    (rival-compile exprs vars discs)))

(define (find-optimal-precisions machine pt)
  (with-handlers ([exn:rival:invalid? (lambda (_) #f)]
                  [exn:rival:unsamplable? (lambda (_) #f)])
    (define optimal-precisions
      (parameterize ([*rival-max-precision* 32256])
        (rival-machine-find-optimal-precisions machine (list->vector (map bf pt)))))
    (vector->list optimal-precisions)))

(define (process-record rec i)
  (define pts (hash-ref rec 'points))
  (define machine (compile-record rec))
  (hash 'exprs (hash-ref rec 'exprs)
        'points (for/list ([pt* (in-list pts)])
                  (find-optimal-precisions machine (point-record->point pt*)))))

(define (run points-port output-port)
  (define output
    (for/list ([rec (in-port read-json points-port)]
               [i (in-naturals)])
      (fprintf (current-error-port) "~a: processing benchmark\n" i)
      (process-record rec i)))
  (write-json output output-port)
  (newline output-port))

(module+ main
  (require racket/cmdline)

  (define output-file "infra/optimal_points.json")

  (command-line
   #:once-each
   [("-o") output "Write output JSON to OUTPUT"
          (set! output-file output)]
   #:args ([points-file "infra/points.json"])
   (call-with-input-file points-file
     (lambda (points-port)
       (call-with-output-file output-file
         (lambda (output-port)
           (run points-port output-port))
         #:mode 'text
         #:exists 'replace))
     #:mode 'text)))
