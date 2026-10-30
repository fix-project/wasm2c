;; to visually check the names of locals
;; tests for:
;; locals all named, all unnamed, mixed

(module
  ;; one local
  (func $one_unnamed (local i32))
  (func $one_named (local $a i32))

  ;; many locals
  (func $many_unnamed (local i32 i64))
  (func $many_named (local $a i32) (local $b i64))
  (func $many_mixed (local $a i32) (local i64) (local $c f32))

  ;; locals are numbered after the params
  (func $after_params (param $x i32) (param i32) (local $a i64) (local i64))

  ;; every type is zero-initialized
  (func $all_types (local i32 i64 f32 f64))
)
