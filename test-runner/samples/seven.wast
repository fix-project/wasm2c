(module (func (export "seven") (result i32) (i32.const 7)))
(assert_return (invoke "seven") (i32.const 7))
