;; Floating-point behaviour that floating.wast (the spec-derived subset)
;; cannot reach. Written by Claude.
;; ====================================================================

;; ceil, floor, trunc, nearest and sqrt applied to signaling NaNs, f32 and f64.
(module
  (func (export "g0") (result i32) (i32.reinterpret_f32 (f32.ceil (f32.const nan:0x1))))
  (func (export "g1") (result i32) (i32.reinterpret_f32 (f32.ceil (f32.const -nan:0x1))))
  (func (export "g2") (result i32) (i32.reinterpret_f32 (f32.ceil (f32.const nan:0x200000))))
  (func (export "g3") (result i32) (i32.reinterpret_f32 (f32.floor (f32.const nan:0x1))))
  (func (export "g4") (result i32) (i32.reinterpret_f32 (f32.floor (f32.const -nan:0x1))))
  (func (export "g5") (result i32) (i32.reinterpret_f32 (f32.floor (f32.const nan:0x200000))))
  (func (export "g6") (result i32) (i32.reinterpret_f32 (f32.trunc (f32.const nan:0x1))))
  (func (export "g7") (result i32) (i32.reinterpret_f32 (f32.trunc (f32.const -nan:0x1))))
  (func (export "g8") (result i32) (i32.reinterpret_f32 (f32.trunc (f32.const nan:0x200000))))
  (func (export "g9") (result i32) (i32.reinterpret_f32 (f32.nearest (f32.const nan:0x1))))
  (func (export "g10") (result i32) (i32.reinterpret_f32 (f32.nearest (f32.const -nan:0x1))))
  (func (export "g11") (result i32) (i32.reinterpret_f32 (f32.nearest (f32.const nan:0x200000))))
  (func (export "g12") (result i32) (i32.reinterpret_f32 (f32.sqrt (f32.const nan:0x1))))
  (func (export "g13") (result i32) (i32.reinterpret_f32 (f32.sqrt (f32.const -nan:0x1))))
  (func (export "g14") (result i32) (i32.reinterpret_f32 (f32.sqrt (f32.const nan:0x200000))))
  (func (export "g15") (result i64) (i64.reinterpret_f64 (f64.ceil (f64.const nan:0x1))))
  (func (export "g16") (result i64) (i64.reinterpret_f64 (f64.ceil (f64.const -nan:0x1))))
  (func (export "g17") (result i64) (i64.reinterpret_f64 (f64.ceil (f64.const nan:0x4000000000000))))
  (func (export "g18") (result i64) (i64.reinterpret_f64 (f64.floor (f64.const nan:0x1))))
  (func (export "g19") (result i64) (i64.reinterpret_f64 (f64.floor (f64.const -nan:0x1))))
  (func (export "g20") (result i64) (i64.reinterpret_f64 (f64.floor (f64.const nan:0x4000000000000))))
  (func (export "g21") (result i64) (i64.reinterpret_f64 (f64.trunc (f64.const nan:0x1))))
  (func (export "g22") (result i64) (i64.reinterpret_f64 (f64.trunc (f64.const -nan:0x1))))
  (func (export "g23") (result i64) (i64.reinterpret_f64 (f64.trunc (f64.const nan:0x4000000000000))))
  (func (export "g24") (result i64) (i64.reinterpret_f64 (f64.nearest (f64.const nan:0x1))))
  (func (export "g25") (result i64) (i64.reinterpret_f64 (f64.nearest (f64.const -nan:0x1))))
  (func (export "g26") (result i64) (i64.reinterpret_f64 (f64.nearest (f64.const nan:0x4000000000000))))
  (func (export "g27") (result i64) (i64.reinterpret_f64 (f64.sqrt (f64.const nan:0x1))))
  (func (export "g28") (result i64) (i64.reinterpret_f64 (f64.sqrt (f64.const -nan:0x1))))
  (func (export "g29") (result i64) (i64.reinterpret_f64 (f64.sqrt (f64.const nan:0x4000000000000))))
)
(assert_return (invoke "g0") (i32.const 0x7fc00001))  ;; f32.ceil quiets sNaN
(assert_return (invoke "g1") (i32.const 0xffc00001))  ;; f32.ceil quiets sNaN
(assert_return (invoke "g2") (i32.const 0x7fe00000))  ;; f32.ceil quiets sNaN
(assert_return (invoke "g3") (i32.const 0x7fc00001))  ;; f32.floor quiets sNaN
(assert_return (invoke "g4") (i32.const 0xffc00001))  ;; f32.floor quiets sNaN
(assert_return (invoke "g5") (i32.const 0x7fe00000))  ;; f32.floor quiets sNaN
(assert_return (invoke "g6") (i32.const 0x7fc00001))  ;; f32.trunc quiets sNaN
(assert_return (invoke "g7") (i32.const 0xffc00001))  ;; f32.trunc quiets sNaN
(assert_return (invoke "g8") (i32.const 0x7fe00000))  ;; f32.trunc quiets sNaN
(assert_return (invoke "g9") (i32.const 0x7fc00001))  ;; f32.nearest quiets sNaN
(assert_return (invoke "g10") (i32.const 0xffc00001))  ;; f32.nearest quiets sNaN
(assert_return (invoke "g11") (i32.const 0x7fe00000))  ;; f32.nearest quiets sNaN
(assert_return (invoke "g12") (i32.const 0x7fc00001))  ;; f32.sqrt quiets sNaN
(assert_return (invoke "g13") (i32.const 0xffc00001))  ;; f32.sqrt quiets sNaN
(assert_return (invoke "g14") (i32.const 0x7fe00000))  ;; f32.sqrt quiets sNaN
(assert_return (invoke "g15") (i64.const 0x7ff8000000000001))  ;; f64.ceil quiets sNaN
(assert_return (invoke "g16") (i64.const 0xfff8000000000001))  ;; f64.ceil quiets sNaN
(assert_return (invoke "g17") (i64.const 0x7ffc000000000000))  ;; f64.ceil quiets sNaN
(assert_return (invoke "g18") (i64.const 0x7ff8000000000001))  ;; f64.floor quiets sNaN
(assert_return (invoke "g19") (i64.const 0xfff8000000000001))  ;; f64.floor quiets sNaN
(assert_return (invoke "g20") (i64.const 0x7ffc000000000000))  ;; f64.floor quiets sNaN
(assert_return (invoke "g21") (i64.const 0x7ff8000000000001))  ;; f64.trunc quiets sNaN
(assert_return (invoke "g22") (i64.const 0xfff8000000000001))  ;; f64.trunc quiets sNaN
(assert_return (invoke "g23") (i64.const 0x7ffc000000000000))  ;; f64.trunc quiets sNaN
(assert_return (invoke "g24") (i64.const 0x7ff8000000000001))  ;; f64.nearest quiets sNaN
(assert_return (invoke "g25") (i64.const 0xfff8000000000001))  ;; f64.nearest quiets sNaN
(assert_return (invoke "g26") (i64.const 0x7ffc000000000000))  ;; f64.nearest quiets sNaN
(assert_return (invoke "g27") (i64.const 0x7ff8000000000001))  ;; f64.sqrt quiets sNaN
(assert_return (invoke "g28") (i64.const 0xfff8000000000001))  ;; f64.sqrt quiets sNaN
(assert_return (invoke "g29") (i64.const 0x7ffc000000000000))  ;; f64.sqrt quiets sNaN

;; abs, neg and copysign applied to signaling NaNs, f32 and f64.
(module
  (func (export "g30") (result i32) (i32.reinterpret_f32 (f32.abs (f32.const -nan:0x1))))
  (func (export "g31") (result i32) (i32.reinterpret_f32 (f32.neg (f32.const nan:0x1))))
  (func (export "g32") (result i32) (i32.reinterpret_f32 (f32.copysign (f32.const nan:0x1) (f32.const -1))))
  (func (export "g33") (result i32) (i32.reinterpret_f32 (f32.copysign (f32.const -nan:0x1) (f32.const 1))))
  (func (export "g34") (result i64) (i64.reinterpret_f64 (f64.abs (f64.const -nan:0x1))))
  (func (export "g35") (result i64) (i64.reinterpret_f64 (f64.neg (f64.const nan:0x1))))
  (func (export "g36") (result i64) (i64.reinterpret_f64 (f64.copysign (f64.const nan:0x1) (f64.const -1))))
  (func (export "g37") (result i64) (i64.reinterpret_f64 (f64.copysign (f64.const -nan:0x1) (f64.const 1))))
)
(assert_return (invoke "g30") (i32.const 0x7f800001))  ;; f32.abs keeps sNaN signaling
(assert_return (invoke "g31") (i32.const 0xff800001))  ;; f32.neg keeps sNaN signaling
(assert_return (invoke "g32") (i32.const 0xff800001))  ;; f32.copysign keeps sNaN signaling
(assert_return (invoke "g33") (i32.const 0x7f800001))  ;; f32.copysign keeps sNaN signaling
(assert_return (invoke "g34") (i64.const 0x7ff0000000000001))  ;; f64.abs keeps sNaN signaling
(assert_return (invoke "g35") (i64.const 0xfff0000000000001))  ;; f64.neg keeps sNaN signaling
(assert_return (invoke "g36") (i64.const 0xfff0000000000001))  ;; f64.copysign keeps sNaN signaling
(assert_return (invoke "g37") (i64.const 0x7ff0000000000001))  ;; f64.copysign keeps sNaN signaling

;; min and max over every combination of +0 and -0, f32 and f64.
(module
  (func (export "g38") (result i32) (i32.reinterpret_f32 (f32.min (f32.const -0) (f32.const 0))))
  (func (export "g39") (result i32) (i32.reinterpret_f32 (f32.min (f32.const 0) (f32.const -0))))
  (func (export "g40") (result i32) (i32.reinterpret_f32 (f32.max (f32.const -0) (f32.const 0))))
  (func (export "g41") (result i32) (i32.reinterpret_f32 (f32.max (f32.const 0) (f32.const -0))))
  (func (export "g42") (result i32) (i32.reinterpret_f32 (f32.min (f32.const -0) (f32.const -0))))
  (func (export "g43") (result i32) (i32.reinterpret_f32 (f32.max (f32.const 0) (f32.const 0))))
  (func (export "g44") (result i64) (i64.reinterpret_f64 (f64.min (f64.const -0) (f64.const 0))))
  (func (export "g45") (result i64) (i64.reinterpret_f64 (f64.min (f64.const 0) (f64.const -0))))
  (func (export "g46") (result i64) (i64.reinterpret_f64 (f64.max (f64.const -0) (f64.const 0))))
  (func (export "g47") (result i64) (i64.reinterpret_f64 (f64.max (f64.const 0) (f64.const -0))))
  (func (export "g48") (result i64) (i64.reinterpret_f64 (f64.min (f64.const -0) (f64.const -0))))
  (func (export "g49") (result i64) (i64.reinterpret_f64 (f64.max (f64.const 0) (f64.const 0))))
)
(assert_return (invoke "g38") (i32.const 0x80000000))  ;; f32.min of signed zeros
(assert_return (invoke "g39") (i32.const 0x80000000))  ;; f32.min of signed zeros
(assert_return (invoke "g40") (i32.const 0x0))  ;; f32.max of signed zeros
(assert_return (invoke "g41") (i32.const 0x0))  ;; f32.max of signed zeros
(assert_return (invoke "g42") (i32.const 0x80000000))  ;; f32.min of signed zeros
(assert_return (invoke "g43") (i32.const 0x0))  ;; f32.max of signed zeros
(assert_return (invoke "g44") (i64.const 0x8000000000000000))  ;; f64.min of signed zeros
(assert_return (invoke "g45") (i64.const 0x8000000000000000))  ;; f64.min of signed zeros
(assert_return (invoke "g46") (i64.const 0x0))  ;; f64.max of signed zeros
(assert_return (invoke "g47") (i64.const 0x0))  ;; f64.max of signed zeros
(assert_return (invoke "g48") (i64.const 0x8000000000000000))  ;; f64.min of signed zeros
(assert_return (invoke "g49") (i64.const 0x0))  ;; f64.max of signed zeros

;; min and max with one NaN operand, in either position, f32 and f64.
(module
  (func (export "g50") (result i32) (i32.reinterpret_f32 (f32.min (f32.const nan:0x1) (f32.const 1))))
  (func (export "g51") (result i32) (i32.reinterpret_f32 (f32.min (f32.const 1) (f32.const nan:0x1))))
  (func (export "g52") (result i32) (i32.reinterpret_f32 (f32.max (f32.const nan:0x1) (f32.const 1))))
  (func (export "g53") (result i32) (i32.reinterpret_f32 (f32.max (f32.const 1) (f32.const nan:0x1))))
  (func (export "g54") (result i64) (i64.reinterpret_f64 (f64.min (f64.const nan:0x1) (f64.const 1))))
  (func (export "g55") (result i64) (i64.reinterpret_f64 (f64.min (f64.const 1) (f64.const nan:0x1))))
  (func (export "g56") (result i64) (i64.reinterpret_f64 (f64.max (f64.const nan:0x1) (f64.const 1))))
  (func (export "g57") (result i64) (i64.reinterpret_f64 (f64.max (f64.const 1) (f64.const nan:0x1))))
)
(assert_return (invoke "g50") (i32.const 0x7fc00001))  ;; f32.min propagates NaN
(assert_return (invoke "g51") (i32.const 0x7fc00001))  ;; f32.min propagates NaN
(assert_return (invoke "g52") (i32.const 0x7fc00001))  ;; f32.max propagates NaN
(assert_return (invoke "g53") (i32.const 0x7fc00001))  ;; f32.max propagates NaN
(assert_return (invoke "g54") (i64.const 0x7ff8000000000001))  ;; f64.min propagates NaN
(assert_return (invoke "g55") (i64.const 0x7ff8000000000001))  ;; f64.min propagates NaN
(assert_return (invoke "g56") (i64.const 0x7ff8000000000001))  ;; f64.max propagates NaN
(assert_return (invoke "g57") (i64.const 0x7ff8000000000001))  ;; f64.max propagates NaN

;; add, sub, mul and div with a signaling NaN operand, f32 and f64.
(module
  (func (export "g58") (result i32) (i32.reinterpret_f32 (f32.add (f32.const nan:0x1) (f32.const 1))))
  (func (export "g59") (result i32) (i32.reinterpret_f32 (f32.sub (f32.const nan:0x1) (f32.const 1))))
  (func (export "g60") (result i32) (i32.reinterpret_f32 (f32.mul (f32.const nan:0x1) (f32.const 1))))
  (func (export "g61") (result i32) (i32.reinterpret_f32 (f32.div (f32.const nan:0x1) (f32.const 1))))
  (func (export "g62") (result i64) (i64.reinterpret_f64 (f64.add (f64.const nan:0x1) (f64.const 1))))
  (func (export "g63") (result i64) (i64.reinterpret_f64 (f64.sub (f64.const nan:0x1) (f64.const 1))))
  (func (export "g64") (result i64) (i64.reinterpret_f64 (f64.mul (f64.const nan:0x1) (f64.const 1))))
  (func (export "g65") (result i64) (i64.reinterpret_f64 (f64.div (f64.const nan:0x1) (f64.const 1))))
)
(assert_return (invoke "g58") (i32.const 0x7fc00001))  ;; f32.add quiets sNaN
(assert_return (invoke "g59") (i32.const 0x7fc00001))  ;; f32.sub quiets sNaN
(assert_return (invoke "g60") (i32.const 0x7fc00001))  ;; f32.mul quiets sNaN
(assert_return (invoke "g61") (i32.const 0x7fc00001))  ;; f32.div quiets sNaN
(assert_return (invoke "g62") (i64.const 0x7ff8000000000001))  ;; f64.add quiets sNaN
(assert_return (invoke "g63") (i64.const 0x7ff8000000000001))  ;; f64.sub quiets sNaN
(assert_return (invoke "g64") (i64.const 0x7ff8000000000001))  ;; f64.mul quiets sNaN
(assert_return (invoke "g65") (i64.const 0x7ff8000000000001))  ;; f64.div quiets sNaN

;; trunc_sat to i32 and i64, signed and unsigned, from f32 and f64: NaN,
;; infinities, zeros, a fractional value and out-of-range magnitudes.
(module
  (func (export "ts_i32_s_f32_0") (result i32) (i32.trunc_sat_f32_s (f32.const nan)))
  (func (export "ts_i32_s_f32_1") (result i32) (i32.trunc_sat_f32_s (f32.const -nan)))
  (func (export "ts_i32_s_f32_2") (result i32) (i32.trunc_sat_f32_s (f32.const nan:0x1)))
  (func (export "ts_i32_s_f32_3") (result i32) (i32.trunc_sat_f32_s (f32.const inf)))
  (func (export "ts_i32_s_f32_4") (result i32) (i32.trunc_sat_f32_s (f32.const -inf)))
  (func (export "ts_i32_s_f32_5") (result i32) (i32.trunc_sat_f32_s (f32.const 0.0)))
  (func (export "ts_i32_s_f32_6") (result i32) (i32.trunc_sat_f32_s (f32.const -0.0)))
  (func (export "ts_i32_s_f32_7") (result i32) (i32.trunc_sat_f32_s (f32.const 3.9)))
  (func (export "ts_i32_s_f32_8") (result i32) (i32.trunc_sat_f32_s (f32.const -3.9)))
  (func (export "ts_i32_s_f32_9") (result i32) (i32.trunc_sat_f32_s (f32.const 1e30)))
  (func (export "ts_i32_s_f32_10") (result i32) (i32.trunc_sat_f32_s (f32.const -1e30)))
  (func (export "ts_i32_s_f64_11") (result i32) (i32.trunc_sat_f64_s (f64.const nan)))
  (func (export "ts_i32_s_f64_12") (result i32) (i32.trunc_sat_f64_s (f64.const -nan)))
  (func (export "ts_i32_s_f64_13") (result i32) (i32.trunc_sat_f64_s (f64.const nan:0x1)))
  (func (export "ts_i32_s_f64_14") (result i32) (i32.trunc_sat_f64_s (f64.const inf)))
  (func (export "ts_i32_s_f64_15") (result i32) (i32.trunc_sat_f64_s (f64.const -inf)))
  (func (export "ts_i32_s_f64_16") (result i32) (i32.trunc_sat_f64_s (f64.const 0.0)))
  (func (export "ts_i32_s_f64_17") (result i32) (i32.trunc_sat_f64_s (f64.const -0.0)))
  (func (export "ts_i32_s_f64_18") (result i32) (i32.trunc_sat_f64_s (f64.const 3.9)))
  (func (export "ts_i32_s_f64_19") (result i32) (i32.trunc_sat_f64_s (f64.const -3.9)))
  (func (export "ts_i32_s_f64_20") (result i32) (i32.trunc_sat_f64_s (f64.const 1e30)))
  (func (export "ts_i32_s_f64_21") (result i32) (i32.trunc_sat_f64_s (f64.const -1e30)))
  (func (export "ts_i32_u_f32_22") (result i32) (i32.trunc_sat_f32_u (f32.const nan)))
  (func (export "ts_i32_u_f32_23") (result i32) (i32.trunc_sat_f32_u (f32.const -nan)))
  (func (export "ts_i32_u_f32_24") (result i32) (i32.trunc_sat_f32_u (f32.const nan:0x1)))
  (func (export "ts_i32_u_f32_25") (result i32) (i32.trunc_sat_f32_u (f32.const inf)))
  (func (export "ts_i32_u_f32_26") (result i32) (i32.trunc_sat_f32_u (f32.const -inf)))
  (func (export "ts_i32_u_f32_27") (result i32) (i32.trunc_sat_f32_u (f32.const 0.0)))
  (func (export "ts_i32_u_f32_28") (result i32) (i32.trunc_sat_f32_u (f32.const -0.0)))
  (func (export "ts_i32_u_f32_29") (result i32) (i32.trunc_sat_f32_u (f32.const 3.9)))
  (func (export "ts_i32_u_f32_30") (result i32) (i32.trunc_sat_f32_u (f32.const -3.9)))
  (func (export "ts_i32_u_f32_31") (result i32) (i32.trunc_sat_f32_u (f32.const 1e30)))
  (func (export "ts_i32_u_f32_32") (result i32) (i32.trunc_sat_f32_u (f32.const -1e30)))
  (func (export "ts_i32_u_f64_33") (result i32) (i32.trunc_sat_f64_u (f64.const nan)))
  (func (export "ts_i32_u_f64_34") (result i32) (i32.trunc_sat_f64_u (f64.const -nan)))
  (func (export "ts_i32_u_f64_35") (result i32) (i32.trunc_sat_f64_u (f64.const nan:0x1)))
  (func (export "ts_i32_u_f64_36") (result i32) (i32.trunc_sat_f64_u (f64.const inf)))
  (func (export "ts_i32_u_f64_37") (result i32) (i32.trunc_sat_f64_u (f64.const -inf)))
  (func (export "ts_i32_u_f64_38") (result i32) (i32.trunc_sat_f64_u (f64.const 0.0)))
  (func (export "ts_i32_u_f64_39") (result i32) (i32.trunc_sat_f64_u (f64.const -0.0)))
  (func (export "ts_i32_u_f64_40") (result i32) (i32.trunc_sat_f64_u (f64.const 3.9)))
  (func (export "ts_i32_u_f64_41") (result i32) (i32.trunc_sat_f64_u (f64.const -3.9)))
  (func (export "ts_i32_u_f64_42") (result i32) (i32.trunc_sat_f64_u (f64.const 1e30)))
  (func (export "ts_i32_u_f64_43") (result i32) (i32.trunc_sat_f64_u (f64.const -1e30)))
  (func (export "ts_i64_s_f32_44") (result i64) (i64.trunc_sat_f32_s (f32.const nan)))
  (func (export "ts_i64_s_f32_45") (result i64) (i64.trunc_sat_f32_s (f32.const -nan)))
  (func (export "ts_i64_s_f32_46") (result i64) (i64.trunc_sat_f32_s (f32.const nan:0x1)))
  (func (export "ts_i64_s_f32_47") (result i64) (i64.trunc_sat_f32_s (f32.const inf)))
  (func (export "ts_i64_s_f32_48") (result i64) (i64.trunc_sat_f32_s (f32.const -inf)))
  (func (export "ts_i64_s_f32_49") (result i64) (i64.trunc_sat_f32_s (f32.const 0.0)))
  (func (export "ts_i64_s_f32_50") (result i64) (i64.trunc_sat_f32_s (f32.const -0.0)))
  (func (export "ts_i64_s_f32_51") (result i64) (i64.trunc_sat_f32_s (f32.const 3.9)))
  (func (export "ts_i64_s_f32_52") (result i64) (i64.trunc_sat_f32_s (f32.const -3.9)))
  (func (export "ts_i64_s_f32_53") (result i64) (i64.trunc_sat_f32_s (f32.const 1e30)))
  (func (export "ts_i64_s_f32_54") (result i64) (i64.trunc_sat_f32_s (f32.const -1e30)))
  (func (export "ts_i64_s_f64_55") (result i64) (i64.trunc_sat_f64_s (f64.const nan)))
  (func (export "ts_i64_s_f64_56") (result i64) (i64.trunc_sat_f64_s (f64.const -nan)))
  (func (export "ts_i64_s_f64_57") (result i64) (i64.trunc_sat_f64_s (f64.const nan:0x1)))
  (func (export "ts_i64_s_f64_58") (result i64) (i64.trunc_sat_f64_s (f64.const inf)))
  (func (export "ts_i64_s_f64_59") (result i64) (i64.trunc_sat_f64_s (f64.const -inf)))
  (func (export "ts_i64_s_f64_60") (result i64) (i64.trunc_sat_f64_s (f64.const 0.0)))
  (func (export "ts_i64_s_f64_61") (result i64) (i64.trunc_sat_f64_s (f64.const -0.0)))
  (func (export "ts_i64_s_f64_62") (result i64) (i64.trunc_sat_f64_s (f64.const 3.9)))
  (func (export "ts_i64_s_f64_63") (result i64) (i64.trunc_sat_f64_s (f64.const -3.9)))
  (func (export "ts_i64_s_f64_64") (result i64) (i64.trunc_sat_f64_s (f64.const 1e30)))
  (func (export "ts_i64_s_f64_65") (result i64) (i64.trunc_sat_f64_s (f64.const -1e30)))
  (func (export "ts_i64_u_f32_66") (result i64) (i64.trunc_sat_f32_u (f32.const nan)))
  (func (export "ts_i64_u_f32_67") (result i64) (i64.trunc_sat_f32_u (f32.const -nan)))
  (func (export "ts_i64_u_f32_68") (result i64) (i64.trunc_sat_f32_u (f32.const nan:0x1)))
  (func (export "ts_i64_u_f32_69") (result i64) (i64.trunc_sat_f32_u (f32.const inf)))
  (func (export "ts_i64_u_f32_70") (result i64) (i64.trunc_sat_f32_u (f32.const -inf)))
  (func (export "ts_i64_u_f32_71") (result i64) (i64.trunc_sat_f32_u (f32.const 0.0)))
  (func (export "ts_i64_u_f32_72") (result i64) (i64.trunc_sat_f32_u (f32.const -0.0)))
  (func (export "ts_i64_u_f32_73") (result i64) (i64.trunc_sat_f32_u (f32.const 3.9)))
  (func (export "ts_i64_u_f32_74") (result i64) (i64.trunc_sat_f32_u (f32.const -3.9)))
  (func (export "ts_i64_u_f32_75") (result i64) (i64.trunc_sat_f32_u (f32.const 1e30)))
  (func (export "ts_i64_u_f32_76") (result i64) (i64.trunc_sat_f32_u (f32.const -1e30)))
  (func (export "ts_i64_u_f64_77") (result i64) (i64.trunc_sat_f64_u (f64.const nan)))
  (func (export "ts_i64_u_f64_78") (result i64) (i64.trunc_sat_f64_u (f64.const -nan)))
  (func (export "ts_i64_u_f64_79") (result i64) (i64.trunc_sat_f64_u (f64.const nan:0x1)))
  (func (export "ts_i64_u_f64_80") (result i64) (i64.trunc_sat_f64_u (f64.const inf)))
  (func (export "ts_i64_u_f64_81") (result i64) (i64.trunc_sat_f64_u (f64.const -inf)))
  (func (export "ts_i64_u_f64_82") (result i64) (i64.trunc_sat_f64_u (f64.const 0.0)))
  (func (export "ts_i64_u_f64_83") (result i64) (i64.trunc_sat_f64_u (f64.const -0.0)))
  (func (export "ts_i64_u_f64_84") (result i64) (i64.trunc_sat_f64_u (f64.const 3.9)))
  (func (export "ts_i64_u_f64_85") (result i64) (i64.trunc_sat_f64_u (f64.const -3.9)))
  (func (export "ts_i64_u_f64_86") (result i64) (i64.trunc_sat_f64_u (f64.const 1e30)))
  (func (export "ts_i64_u_f64_87") (result i64) (i64.trunc_sat_f64_u (f64.const -1e30)))
)
(assert_return (invoke "ts_i32_s_f32_0") (i32.const 0))
(assert_return (invoke "ts_i32_s_f32_1") (i32.const 0))
(assert_return (invoke "ts_i32_s_f32_2") (i32.const 0))
(assert_return (invoke "ts_i32_s_f32_3") (i32.const 2147483647))
(assert_return (invoke "ts_i32_s_f32_4") (i32.const -2147483648))
(assert_return (invoke "ts_i32_s_f32_5") (i32.const 0))
(assert_return (invoke "ts_i32_s_f32_6") (i32.const 0))
(assert_return (invoke "ts_i32_s_f32_7") (i32.const 3))
(assert_return (invoke "ts_i32_s_f32_8") (i32.const -3))
(assert_return (invoke "ts_i32_s_f32_9") (i32.const 2147483647))
(assert_return (invoke "ts_i32_s_f32_10") (i32.const -2147483648))
(assert_return (invoke "ts_i32_s_f64_11") (i32.const 0))
(assert_return (invoke "ts_i32_s_f64_12") (i32.const 0))
(assert_return (invoke "ts_i32_s_f64_13") (i32.const 0))
(assert_return (invoke "ts_i32_s_f64_14") (i32.const 2147483647))
(assert_return (invoke "ts_i32_s_f64_15") (i32.const -2147483648))
(assert_return (invoke "ts_i32_s_f64_16") (i32.const 0))
(assert_return (invoke "ts_i32_s_f64_17") (i32.const 0))
(assert_return (invoke "ts_i32_s_f64_18") (i32.const 3))
(assert_return (invoke "ts_i32_s_f64_19") (i32.const -3))
(assert_return (invoke "ts_i32_s_f64_20") (i32.const 2147483647))
(assert_return (invoke "ts_i32_s_f64_21") (i32.const -2147483648))
(assert_return (invoke "ts_i32_u_f32_22") (i32.const 0))
(assert_return (invoke "ts_i32_u_f32_23") (i32.const 0))
(assert_return (invoke "ts_i32_u_f32_24") (i32.const 0))
(assert_return (invoke "ts_i32_u_f32_25") (i32.const 4294967295))
(assert_return (invoke "ts_i32_u_f32_26") (i32.const 0))
(assert_return (invoke "ts_i32_u_f32_27") (i32.const 0))
(assert_return (invoke "ts_i32_u_f32_28") (i32.const 0))
(assert_return (invoke "ts_i32_u_f32_29") (i32.const 3))
(assert_return (invoke "ts_i32_u_f32_30") (i32.const 0))
(assert_return (invoke "ts_i32_u_f32_31") (i32.const 4294967295))
(assert_return (invoke "ts_i32_u_f32_32") (i32.const 0))
(assert_return (invoke "ts_i32_u_f64_33") (i32.const 0))
(assert_return (invoke "ts_i32_u_f64_34") (i32.const 0))
(assert_return (invoke "ts_i32_u_f64_35") (i32.const 0))
(assert_return (invoke "ts_i32_u_f64_36") (i32.const 4294967295))
(assert_return (invoke "ts_i32_u_f64_37") (i32.const 0))
(assert_return (invoke "ts_i32_u_f64_38") (i32.const 0))
(assert_return (invoke "ts_i32_u_f64_39") (i32.const 0))
(assert_return (invoke "ts_i32_u_f64_40") (i32.const 3))
(assert_return (invoke "ts_i32_u_f64_41") (i32.const 0))
(assert_return (invoke "ts_i32_u_f64_42") (i32.const 4294967295))
(assert_return (invoke "ts_i32_u_f64_43") (i32.const 0))
(assert_return (invoke "ts_i64_s_f32_44") (i64.const 0))
(assert_return (invoke "ts_i64_s_f32_45") (i64.const 0))
(assert_return (invoke "ts_i64_s_f32_46") (i64.const 0))
(assert_return (invoke "ts_i64_s_f32_47") (i64.const 9223372036854775807))
(assert_return (invoke "ts_i64_s_f32_48") (i64.const -9223372036854775808))
(assert_return (invoke "ts_i64_s_f32_49") (i64.const 0))
(assert_return (invoke "ts_i64_s_f32_50") (i64.const 0))
(assert_return (invoke "ts_i64_s_f32_51") (i64.const 3))
(assert_return (invoke "ts_i64_s_f32_52") (i64.const -3))
(assert_return (invoke "ts_i64_s_f32_53") (i64.const 9223372036854775807))
(assert_return (invoke "ts_i64_s_f32_54") (i64.const -9223372036854775808))
(assert_return (invoke "ts_i64_s_f64_55") (i64.const 0))
(assert_return (invoke "ts_i64_s_f64_56") (i64.const 0))
(assert_return (invoke "ts_i64_s_f64_57") (i64.const 0))
(assert_return (invoke "ts_i64_s_f64_58") (i64.const 9223372036854775807))
(assert_return (invoke "ts_i64_s_f64_59") (i64.const -9223372036854775808))
(assert_return (invoke "ts_i64_s_f64_60") (i64.const 0))
(assert_return (invoke "ts_i64_s_f64_61") (i64.const 0))
(assert_return (invoke "ts_i64_s_f64_62") (i64.const 3))
(assert_return (invoke "ts_i64_s_f64_63") (i64.const -3))
(assert_return (invoke "ts_i64_s_f64_64") (i64.const 9223372036854775807))
(assert_return (invoke "ts_i64_s_f64_65") (i64.const -9223372036854775808))
(assert_return (invoke "ts_i64_u_f32_66") (i64.const 0))
(assert_return (invoke "ts_i64_u_f32_67") (i64.const 0))
(assert_return (invoke "ts_i64_u_f32_68") (i64.const 0))
(assert_return (invoke "ts_i64_u_f32_69") (i64.const 18446744073709551615))
(assert_return (invoke "ts_i64_u_f32_70") (i64.const 0))
(assert_return (invoke "ts_i64_u_f32_71") (i64.const 0))
(assert_return (invoke "ts_i64_u_f32_72") (i64.const 0))
(assert_return (invoke "ts_i64_u_f32_73") (i64.const 3))
(assert_return (invoke "ts_i64_u_f32_74") (i64.const 0))
(assert_return (invoke "ts_i64_u_f32_75") (i64.const 18446744073709551615))
(assert_return (invoke "ts_i64_u_f32_76") (i64.const 0))
(assert_return (invoke "ts_i64_u_f64_77") (i64.const 0))
(assert_return (invoke "ts_i64_u_f64_78") (i64.const 0))
(assert_return (invoke "ts_i64_u_f64_79") (i64.const 0))
(assert_return (invoke "ts_i64_u_f64_80") (i64.const 18446744073709551615))
(assert_return (invoke "ts_i64_u_f64_81") (i64.const 0))
(assert_return (invoke "ts_i64_u_f64_82") (i64.const 0))
(assert_return (invoke "ts_i64_u_f64_83") (i64.const 0))
(assert_return (invoke "ts_i64_u_f64_84") (i64.const 3))
(assert_return (invoke "ts_i64_u_f64_85") (i64.const 0))
(assert_return (invoke "ts_i64_u_f64_86") (i64.const 18446744073709551615))
(assert_return (invoke "ts_i64_u_f64_87") (i64.const 0))
