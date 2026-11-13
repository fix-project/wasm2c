;; Simple memory test (by Claude)
;; covers data segments, loads and stores of each width, memory.size,
;; memory.grow and out-of-bounds traps, using only the operators wasm2c
;; supports so far (no locals, so every address is a constant)

(module
  (memory (export "mem") 1 2)  ;; 1 page (64 KiB), can grow to 2
  (data (i32.const 0) "\01\02\03\04\05\06\07\08")
  (data (i32.const 16) "\ff\fe")
  (data (i32.const 65532) "\aa\bb\cc\dd")  ;; the last 4 bytes of page 1

  ;; full-width loads are little-endian, and needn't be aligned
  (func (export "i32_load") (result i32) (i32.load (i32.const 0)))
  (func (export "i64_load") (result i64) (i64.load (i32.const 0)))
  (func (export "i32_load_offset") (result i32) (i32.load offset=4 (i32.const 0)))
  (func (export "i32_load_unaligned") (result i32) (i32.load (i32.const 1)))

  ;; narrow loads: _s sign-extends, _u zero-extends
  (func (export "i32_load8_s") (result i32) (i32.load8_s (i32.const 16)))
  (func (export "i32_load8_u") (result i32) (i32.load8_u (i32.const 16)))
  (func (export "i32_load16_s") (result i32) (i32.load16_s (i32.const 16)))
  (func (export "i32_load16_u") (result i32) (i32.load16_u (i32.const 16)))
  (func (export "i64_load8_s") (result i64) (i64.load8_s (i32.const 16)))
  (func (export "i64_load32_u") (result i64) (i64.load32_u (i32.const 65532)))

  ;; narrow stores keep only the low bytes
  (func (export "load_32") (result i32) (i32.load (i32.const 32)))
  (func (export "store8_32") (i32.store8 (i32.const 32) (i32.const 0x1234)))
  (func (export "store16_32") (i64.store16 (i32.const 32) (i64.const -1)))
  (func (export "load_40") (result i64) (i64.load (i32.const 40)))
  (func (export "store64_40") (i64.store (i32.const 40) (i64.const 0x0102030405060708)))

  ;; reads that end exactly at, or run past, the end of memory
  (func (export "load_last_4") (result i32) (i32.load (i32.const 65532)))
  (func (export "load_crossing_end") (result i32) (i32.load (i32.const 65533)))
  (func (export "load_huge_offset") (result i32) (i32.load offset=0xffffffff (i32.const 1)))
  (func (export "store_crossing_end") (i32.store (i32.const 65535) (i32.const 0)))

  ;; size and grow count 64 KiB pages
  (func (export "size") (result i32) (memory.size))
  (func (export "grow_1") (result i32) (memory.grow (i32.const 1)))
)

(assert_return (invoke "i32_load") (i32.const 0x04030201))
(assert_return (invoke "i64_load") (i64.const 0x0807060504030201))
(assert_return (invoke "i32_load_offset") (i32.const 0x08070605))
(assert_return (invoke "i32_load_unaligned") (i32.const 0x05040302))

(assert_return (invoke "i32_load8_s") (i32.const -1))
(assert_return (invoke "i32_load8_u") (i32.const 255))
(assert_return (invoke "i32_load16_s") (i32.const -257))
(assert_return (invoke "i32_load16_u") (i32.const 65279))
(assert_return (invoke "i64_load8_s") (i64.const -1))
(assert_return (invoke "i64_load32_u") (i64.const 0xddccbbaa))

(assert_return (invoke "load_32") (i32.const 0))
(assert_return (invoke "store8_32"))
(assert_return (invoke "load_32") (i32.const 0x34))
(assert_return (invoke "store16_32"))
(assert_return (invoke "load_32") (i32.const 0xffff))
(assert_return (invoke "store64_40"))
(assert_return (invoke "load_40") (i64.const 0x0102030405060708))

;; out of bounds: one byte past the end traps, and address + offset
;; is not allowed to wrap around to a small address
(assert_return (invoke "load_last_4") (i32.const 0xddccbbaa))
(assert_trap (invoke "load_crossing_end") "out of bounds memory access")
(assert_trap (invoke "load_huge_offset") "out of bounds memory access")
(assert_trap (invoke "store_crossing_end") "out of bounds memory access")

;; growing returns the old size, adds zeroed pages, and stops at the maximum
(assert_return (invoke "size") (i32.const 1))
(assert_return (invoke "grow_1") (i32.const 1))
(assert_return (invoke "size") (i32.const 2))
(assert_return (invoke "load_crossing_end") (i32.const 0x00ddccbb))
(assert_return (invoke "grow_1") (i32.const -1))
(assert_return (invoke "size") (i32.const 2))

;; an empty memory with no maximum (so up to 4 GiB)
(module
  (memory 0)
  (func (export "size") (result i32) (memory.size))
  (func (export "grow_2") (result i32) (memory.grow (i32.const 2)))
  (func (export "load_0") (result i32) (i32.load (i32.const 0)))
)

(assert_return (invoke "size") (i32.const 0))
(assert_trap (invoke "load_0") "out of bounds memory access")
(assert_return (invoke "grow_2") (i32.const 0))
(assert_return (invoke "size") (i32.const 2))
(assert_return (invoke "load_0") (i32.const 0))
