#lang info

(define collection "rival3")
(define version "1.0")

(define pkg-desc "Racket bindings to Rival 3")
(define license 'MIT)
(define build-platforms
  '("win32-x86_64"
    "linux-x86_64"
    "linux-aarch64"
    "macosx-x86_64"
    "macosx-aarch64"))

(define deps '(("base" #:version "8.0") "math-lib"))
(define build-deps '("scribble-lib" "racket-doc" "math-doc"))
(define scribblings '(("scribblings/rival3.scrbl" (multi-page) (library))))
