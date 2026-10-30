;; to visually check the names of params
;; tests for:
;; no params, one param, many params
;; all named, all unnamed, mixed

(module
  ;; no params
  (func (export "none_exported"))
  (func $none)

  ;; one param
  (func (export "one_unnamed_exported") (param i32))
  (func $one_unnamed (param i32))
  (func (export "one_named_exported") (param $x i32))
  (func $one_named (param $x i32))

  ;; many params
  (func (export "many_unnamed_exported") (param i32 i64))
  (func $many_unnamed (param i32 i64))
  (func (export "many_named_exported") (param $x i32) (param $y i64))
  (func $many_named (param $x i32) (param $y i64))
  (func (export "many_mixed_exported") (param $x i32) (param i64) (param $z f32))
  (func $many_mixed (param $x i32) (param i64) (param $z f32))

  ;; exported twice
  ;; both export and wrapper should have same param names
  (func (export "wrapped_exported") (export "wrapper_exported") (param $x i32) (param i64))
)
