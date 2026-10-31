;; Simple global test (by Claude)
;; will be useful until we pass all of the spec test

(module
  (global $counter (export "a") (mut i32) (i32.const 10))
  (global $total   (export "b") (mut i64) (i64.const 4294967296))
  (global $neg     (export "c") i32 (i32.const -1))
  (global $answer  (export "d") i64 (i64.const 42))

  (func (export "get_a") (result i32) (global.get $counter))
  (func (export "get_b") (result i64) (global.get $total))
  (func (export "get_c") (result i32) (global.get $neg))
  (func (export "get_d") (result i64) (global.get $answer))

  (func (export "set_a_52") (global.set $counter (i32.const 52)))
  (func (export "set_b_90") (global.set $total (i64.const -90)))
)

;; Initial values via `get`
(assert_return (get "a") (i32.const 10))
(assert_return (get "b") (i64.const 4294967296))
(assert_return (get "c") (i32.const -1))
(assert_return (get "d") (i64.const 42))

;; Initial values via `invoke`
(assert_return (invoke "get_a") (i32.const 10))
(assert_return (invoke "get_b") (i64.const 4294967296))
(assert_return (invoke "get_c") (i32.const -1))
(assert_return (invoke "get_d") (i64.const 42))

;; Set, then check both ways
(assert_return (invoke "set_a_52"))
(assert_return (get "a") (i32.const 52))
(assert_return (invoke "get_a") (i32.const 52))

(assert_return (get "b") (i64.const 4294967296))
(assert_return (invoke "get_b") (i64.const 4294967296))

(assert_return (invoke "set_b_90"))
(assert_return (get "b") (i64.const -90))
(assert_return (invoke "get_b") (i64.const -90))

;; Immutable globals are unaffected by the setters
(assert_return (get "c") (i32.const -1))
(assert_return (get "d") (i64.const 42))
