;; Name mangling
;; tests Wasm names that are not valid C++ names

(module
  ;; rule 1: hex digit after "0x"
  (func (export "x0xAA") (result i32) (i32.const 1))
  (func (export "x0x1A") (result i32) (i32.const 2))
  (func (export "x0xZA") (result i32) (i32.const 3)) ;; not hex digit, should not escape

  ;; rule 2
  ;; double underscore, starts with underscore, ends with underscore
  (func (export "a__b") (result i32) (i32.const 4))
  ;; needs prefixes or something
  ;; (func (export "_x") (result i32) (i32.const 5))
  (func (export "x_") (result i32) (i32.const 6))

  ;; rule 3
  ;; non-ASCII bytes
  (func (export "xÅ") (result i32) (i32.const 7))
)

(assert_return (invoke "x0xAA") (i32.const 1))
(assert_return (invoke "x0x1A") (i32.const 2))
(assert_return (invoke "x0xZA") (i32.const 3))
(assert_return (invoke "a__b") (i32.const 4))
;; (assert_return (invoke "_x") (i32.const 5))
(assert_return (invoke "x_") (i32.const 6))
(assert_return (invoke "xÅ") (i32.const 7))
