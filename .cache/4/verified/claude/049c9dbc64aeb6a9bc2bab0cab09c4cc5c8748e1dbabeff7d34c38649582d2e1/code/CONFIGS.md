# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE

The mirror of `ERRORS.md`: every **valid** configuration the C actually
distinguishes. Axes derived mechanically from `c_src/src/lib.c` +
`c_src/include/lib.h`, not guessed.

## Axes the C code branches on

**Build-time.** None. `Cargo.toml` has no `[features]`;
`c_src/CMakeLists.txt` has no `option()`/`add_definitions`; `lib.c` contains no
`#ifdef`. => 1 configuration, the empty feature set. (`./check_all_features.sh`
prints `features found: 0`.)

**Runtime options / modes** — the library has no setter API; its "options" are
the three file-scope tunables and the dispatch selector:

| axis | values the C distinguishes | where |
|---|---|---|
| `opcode` selector | `0` mult, `1` add, `2` xor, `3` shift (`if (opcode >= 0 && opcode < 4)`) | lib.c:76 |
| `ops[]` lazy-init state | **first** call (`ops[0]==NULL` -> table filled) vs **every later** call | lib.c:69-74 |
| `static_multiplier` = 3 | baked in, multiplies every `multiply_with_static` result | lib.c:46,51 |
| `static_addend` = 100 | baked in, added by `add_with_static` | lib.c:47,55 |
| `static_shift_amount` = 2 | baked in, both shift directions in `shift_with_static` | lib.c:48,63 |
| `MAGIC_NUMBER` / `MASK_LOWER` mix-in | applied **only** on the `values!=NULL && count>0` branch | lib.c:110,113 |
| `copy_count` clamp | `count > 4 ? 4 : count` -> 1,2,3,4 (=> 4,8,12,16 folded bytes) | lib.c:103 |
| `operation_func` provenance | table entry from own `.so` / from the **other** `.so` / caller-minted | lib.c:44,83,129 |

**Input shapes the code special-cases:**

* sign of `a` in `a << 2` (bits shifted out of / into the sign bit) and sign of
  `b` in `b >> 2` (arithmetic sign-extension) — lib.c:63;
* wrap-around of `a*b`, `(a*b)*3`, `a+b`, `(a+b)+100`, `accumulator+shift_result`
  — lib.c:51,55,179;
* host **byte order** — `compute_checksum` folds the raw object representation of
  the `int`s, one `unsigned char` at a time — lib.c:104-108;
* element count 1 / 2 / 3 / 4 / many(clamped) for the `int*` input;
* `ComputeState` prior content (`init_state` overwrites all three fields via
  `memcpy` of a compound literal) — lib.c:122-124;
* number of accumulated operations (`operation_count++`) — lib.c:141;
* `const char* op_name` shape: normal / empty / long / containing `%` — lib.c:85,93.

**Full set of public entry points** (all 10 exported symbols, low-level first):

`multiply_with_static`, `add_with_static`, `xor_operation`, `shift_with_static`
(leaf arithmetic) -> `get_operation` (table lookup) -> `compute_checksum`,
`init_state`, `apply_operation`, `execute_operation` (mid-level, stateful) ->
`checkshift` (the one-shot wrapper in `include/lib.h`).

Rows below deliberately exercise the **low-level** entries directly and then the
**hand-composed pipeline** (C22), not only the `checkshift` convenience wrapper.

## Configuration rows

Every row is driven with many randomized inputs (SplitMix64, fixed seed
`0x5EED_1234_ABCD_F00D`) **plus** the edge set
`{0, 1, -1, 2, -2, 3, 7, INT_MIN, INT_MAX, INT_MIN+1, INT_MAX-1, 0x55555555,
-0x55555556, 0xFFFF, 0x10000, 0x7FFF}`, and compares C vs Rust **return value
and captured stdout, byte for byte**.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|--------------------------------------------|------|---|
| C1 | `multiply_with_static` | both operands random over the full `i32` range (product wraps in `a*b` and again in `*3`) | `c1_multiply_random` | [x] |
| C2 | `multiply_with_static` | edge-value cross product `edges × edges` (incl. `0`, `INT_MIN`, `INT_MAX`, `-1`) | `c2_multiply_edges` | [x] |
| C3 | `add_with_static` | random full-range operands (wrap in `a+b` and again in `+100`) | `c3_add_random` | [x] |
| C4 | `add_with_static` | edge cross product — `INT_MAX + 1`, `INT_MIN - 1`, `INT_MAX-1 + 100` wrap paths | `c4_add_edges` | [x] |
| C5 | `xor_operation` | random operands; sign bit set/clear; `0xABCD` mix-in | `c5_xor_random` | [x] |
| C6 | `xor_operation` | edge cross product (`INT_MIN`, `INT_MAX`, `-1`, `0`, `0xABCD` itself) | `c6_xor_edges` | [x] |
| C7 | `shift_with_static` | `a >= 0`, `b >= 0` — left shift stays in-range, right shift zero-extends | `c7_shift_pos_pos` | [x] |
| C8 | `shift_with_static` | `a < 0` — left shift of a negative value (bits discarded past the sign bit) | `c8_shift_neg_a` | [x] |
| C9 | `shift_with_static` | `b < 0` — **arithmetic** right shift, sign-extended low bits ORed in | `c9_shift_neg_b` | [x] |
| C10 | `shift_with_static` | both negative + full random full-range + edge cross product (`INT_MIN`, `INT_MAX`, `±1`, `±3`) | `c10_shift_random_and_edges` | [x] |
| C11 | `get_operation` | valid `opcode` 0,1,2,3 — returned pointer must be non-NULL **and** must equal that `.so`'s own exported `multiply_with_static`/`add_with_static`/`xor_operation`/`shift_with_static` (`dlsym`) | `c11_get_operation_pointer_identity` | [x] |
| C12 | `get_operation` | **lazy-init axis**: first-ever call vs 1000 subsequent calls, interleaved opcodes — table must be idempotent and pointer-stable in both `.so`s | `c12_get_operation_lazy_init_idempotent` | [x] |
| C13 | `get_operation` + the returned pointer | invoke the pointer returned for each opcode over random operands: dispatch must land on the same arithmetic as the directly-exported symbol | `c13_get_operation_dispatch_matches_direct` | [x] |
| C14 | `execute_operation` | `func` = each of the 4 table ops (`opcode` 0..3), random operands, `op_name` = normal ASCII -> return value + the 3 printed lines | `c14_execute_operation_all_ops` | [x] |
| C15 | `execute_operation` | `op_name` shape sweep: empty `""`, 1 char, long (200 chars), and a name containing `%d`/`%s`/`%%` (must be forwarded as a `%s` **argument**, never re-interpreted as a format) | `c15_execute_operation_op_name_shapes` | [x] |
| C16 | `execute_operation` | `func` = caller-minted `extern "C"` fn from the **test binary** (foreign `operation_func`), random operands | `c16_execute_operation_foreign_func` | [x] |
| C17 | `compute_checksum` | `count` = 1, 2, 3, 4 (=> 4/8/12/16 folded bytes) over random `int` arrays — byte-order-dependent fold | `c17_checksum_counts_1_to_4` | [x] |
| C18 | `compute_checksum` | byte-pattern shapes at each count: all-`0x00`, all-`0xFF`, `INT_MIN`, `INT_MAX`, `0x01020304`, alternating `0x55/0xAA` (exercises the `<<1` overflow-out of the 32-bit accumulator and the `MAGIC_NUMBER`/`MASK_LOWER` stages) | `c18_checksum_byte_patterns` | [x] |
| C19 | `init_state` | `initial_value` over random + edges, into a **freshly zeroed** struct and into a struct **pre-filled with garbage** — all 3 fields (12 bytes) compared, plus the printed line | `c19_init_state_fresh_and_dirty` | [x] |
| C20 | `apply_operation` | `func` = each of the 4 ops, one application, random `accumulator`/`value`, random pre-existing `operation_count` (incl. `INT_MAX`, exercising the `++` wrap) — full struct compared | `c20_apply_operation_single` | [x] |
| C21 | `apply_operation` | **accumulated** chains: random sequences of 2, 3, 10 and 64 operations with random opcodes and values on one state — accumulator evolution + `operation_count` | `c21_apply_operation_chains` | [x] |
| C22 | `init_state` + `get_operation` + `apply_operation` + `execute_operation` + `compute_checksum` | **hand-composed pipeline**: the whole `checkshift` algorithm driven from the low-level entry points by the test (mixing the two mid-level stateful calls with the two logging calls, then the final `(acc + shift) ^ checksum` fold) — verifies the composed data flow, not just each wrapper | `c22_composed_pipeline` | [x] |
| C23 | `checkshift` | random 4-tuples over the full `i32` range — return value + the complete ~15-line stdout transcript | `c23_checkshift_random` | [x] |
| C24 | `checkshift` | edge 4-tuples: all-zero, all-`INT_MIN`, all-`INT_MAX`, `-1`s, mixed sign, and each edge value in each of the 4 positions with the rest fixed | `c24_checkshift_edges` | [x] |
| C25 | cross-`.so` | `operation_func` **minted by C's `get_operation` fed into Rust's `execute_operation`/`apply_operation`**, and vice versa; plus a `ComputeState` written by C's `init_state` and advanced by Rust's `apply_operation` and vice versa (struct + fn-ptr ABI parity) | `c25_cross_module_abi` | [x] |
| C25b | `init_state` + `apply_operation` | `ComputeState` object representation: `size_of` = 12, `align_of` = 4, and the 12 raw bytes each `.so` writes for a distinctive value must be identical | `c25b_compute_state_layout_agrees` | [x] |
