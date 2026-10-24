;; single result
(module
  (func (export "true_branch") (result i32)
    i32.const 10
    i32.const 1
    if (result i32)
      i32.const 20
    else
      i32.const 30
    end
    i32.add)

  (func (export "false_branch") (result i32)
    i32.const 10
    i32.const 0
    if (result i32)
      i32.const 20
    else
      i32.const 30
    end
    i32.add)
)

(assert_return (invoke "true_branch") (i32.const 30))
(assert_return (invoke "false_branch") (i32.const 40))

;; parameters
(module
  (func (export "true_branch") (result i32)
    i32.const 10
    i32.const 20
    i32.const 1
    if (param i32 i32) (result i32)
      drop
    else
      drop
    end)

  (func (export "false_branch") (result i32)
    i32.const 10
    i32.const 20
    i32.const 0
    if (param i32 i32) (result i32)
      drop
    else
      drop
    end)
)

(assert_return (invoke "true_branch") (i32.const 10))
(assert_return (invoke "false_branch") (i32.const 10))

;; multi result
(module
  (func (export "true_branch") (result i32 i32)
    i32.const 1
    if (result i32 i32)
      i32.const 10
      i32.const 20
    else
      i32.const 30
      i32.const 40
    end)

  (func (export "false_branch") (result i32 i32)
    i32.const 0
    if (result i32 i32)
      i32.const 10
      i32.const 20
    else
      i32.const 30
      i32.const 40
    end)
)

(assert_return (invoke "true_branch") (i32.const 10) (i32.const 20))
(assert_return (invoke "false_branch") (i32.const 30) (i32.const 40))

;; implicit else
(module
  (func (export "false_branch") (result i32)
    i32.const 42
    i32.const 0
    if (param i32) (result i32)
      drop
      i32.const 99
    end)

  (func (export "true_branch") (result i32)
    i32.const 42
    i32.const 1
    if (param i32) (result i32)
      drop
      i32.const 99
    end)
)

(assert_return (invoke "false_branch") (i32.const 42))
(assert_return (invoke "true_branch") (i32.const 99))
