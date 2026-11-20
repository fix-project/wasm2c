;; In C++, `-9223372036854775808` (i64 min) parses as unary minus applied to
;; `9223372036854775808`, which exceeds LLONG_MAX.
;; Literals need to be emitted as unsigned.
(module (func (export "min_i64") (result i64) (i64.const -9223372036854775808)))
(assert_return (invoke "min_i64") (i64.const -9223372036854775808))
