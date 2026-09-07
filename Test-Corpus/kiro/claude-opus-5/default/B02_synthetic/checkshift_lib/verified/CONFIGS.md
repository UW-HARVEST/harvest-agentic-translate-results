# CONFIGS.md — Phase B configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` + `c_src/include/lib.h`, not from assumptions.

### A. Runtime options / modes

This library exposes **no** setter, flag, mode or `#ifdef`. The only
"configuration" state is the three file-scope `static`s, which are never
written after initialisation:

| state | value | effect |
|-------|-------|--------|
| `static_multiplier`   | `3`   | `multiply_with_static`: `(a*b)*3` |
| `static_addend`       | `100` | `add_with_static`: `(a+b)+100` |
| `static_shift_amount` | `2`   | `shift_with_static`: `(a<<2)|(b>>2)` |
| `ops[4]` (function-`static` in `get_operation`) | lazily filled on first call | first-call vs. subsequent-call path (`if (ops[0] == NULL)`) |

The *selectable* mode is therefore the `opcode` dispatch in `get_operation`
(0=multiply, 1=add, 2=xor, 3=shift) and which function pointer a caller hands
to `execute_operation` / `apply_operation`.

### B. Input shapes the code special-cases

* `opcode`: `0,1,2,3` valid; `<0` / `>=4` rejected (see ERRORS.md).
* `count` in `compute_checksum`: `count > 4 ? 4 : count` — distinct byte
  lengths `4,8,12,16` for `count = 1,2,3,4`, and clamping for `count > 4`.
* signed-arithmetic shapes: zero, positive, negative, `INT_MAX`, `INT_MIN`,
  values that wrap on `*`, `+`, and `<<`.
* `shift_with_static`: sign bit shifted out of `a<<2`; arithmetic (sign
  extending) `b>>2` for negative `b`.
* `compute_checksum` byte shapes: bytes with the high bit set (`unsigned char`
  promotion), little-endian byte order of `int`, all-zero, all-`0xFF`.
* `ComputeState` shapes: fresh vs. already-populated struct, repeated
  `apply_operation` calls (`operation_count` accumulation), `checksum` field
  untouched by `apply_operation`.

### C. Full set of public entry points (lowest level first)

`multiply_with_static`, `add_with_static`, `xor_operation`,
`shift_with_static` (leaf ops) → `get_operation` (dispatch) →
`execute_operation`, `compute_checksum`, `init_state`, `apply_operation`
(mid level) → `checkshift` (the only one declared in `lib.h`; the composed
pipeline).

## Configuration table

Every row is exercised with **many randomized inputs** from a fixed-seed
xorshift PRNG (`SEED = 0x5EED_C0FFEE`) plus the listed edge values, and both
`.so`s' return values **and** captured stdout bytes are compared.
Row checked off only after it passes across all randomized inputs.

| # | entry point(s) | configuration (options set + input shape) | ok |
|---|----------------|-------------------------------------------|----|
| 1 | `multiply_with_static` | random `(a,b)` full `i32` range, 20000 cases (exercises `*3` wrap) | [x] |
| 2 | `multiply_with_static` | edge grid: `0, 1, -1, 2, -2, INT_MAX, INT_MIN, INT_MAX/3, 0x55555555, 0xAAAAAAAA` × same (overflow of both `a*b` and `*3`) | [x] |
| 3 | `multiply_with_static` | small-magnitude random `(a,b)` in `[-1000,1000]` (no-overflow regime) | [x] |
| 4 | `add_with_static` | random `(a,b)` full `i32` range, 20000 cases | [x] |
| 5 | `add_with_static` | edge grid incl. `INT_MAX`, `INT_MAX-99`, `INT_MAX-100`, `INT_MIN`, `-100` (the `+100` boundary wrap) | [x] |
| 6 | `xor_operation` | random `(a,b)` full range, 20000 cases (`^0xABCD`) | [x] |
| 7 | `xor_operation` | edge grid incl. `0, -1, 0xABCD, ~0xABCD, INT_MIN, INT_MAX` | [x] |
| 8 | `shift_with_static` | random `(a,b)` full range, 20000 cases (`a<<2` bit loss, arithmetic `b>>2`) | [x] |
| 9 | `shift_with_static` | edge grid: `a` with high bits set / sign bit set (`INT_MIN`, `0x40000000`, `0x60000000`, `-1`), `b` negative and positive, `b` in `[-4,4]` (rounding of arithmetic shift) | [x] |
| 10 | `get_operation` | valid `opcode` 0,1,2,3 — returned pointer identity checked against `dlsym` of the matching op name in the *same* library; called through and compared | [x] |
| 11 | `get_operation` | first-call vs. repeated-call (`ops[0] == NULL` lazy-init branch): call opcode 0 first, then all opcodes, then re-call in reverse order | [x] |
| 12 | `execute_operation` | `func = get_operation(0)` (multiply), random `(a,b)`, `op_name = "MULT"` — return value + full stdout (`Variable a`, `Variable b`, `Result of %s`) | [x] |
| 13 | `execute_operation` | `func = get_operation(1)` (add), random `(a,b)`, `op_name = "ADD"` | [x] |
| 14 | `execute_operation` | `func = get_operation(2)` (xor), random `(a,b)`, `op_name = "XOR"` | [x] |
| 15 | `execute_operation` | `func = get_operation(3)` (shift), random `(a,b)`, `op_name = "SHIFT"` | [x] |
| 16 | `execute_operation` | `op_name` shape variation: empty string, 1 char, long (200 char) name, name with `%` characters (format-string passthrough), with a valid `func` | [x] |
| 17 | `execute_operation` | **cross-library** function pointer: C's `xor_operation` passed to Rust's `execute_operation` and vice versa, random `(a,b)` | [x] |
| 18 | `compute_checksum` | `count = 1` (4 bytes copied), random `int` values, 5000 cases | [x] |
| 19 | `compute_checksum` | `count = 2` (8 bytes), random values, 5000 cases | [x] |
| 20 | `compute_checksum` | `count = 3` (12 bytes), random values, 5000 cases | [x] |
| 21 | `compute_checksum` | `count = 4` (16 bytes, full buffer), random values, 5000 cases | [x] |
| 22 | `compute_checksum` | `count = 5` (clamp boundary, array of 8 valid ints) | [x] |
| 23 | `compute_checksum` | `count = 1000` and `count = INT_MAX` with a 4-element array (clamp to 4 — must not over-read) | [x] |
| 24 | `compute_checksum` | byte-pattern shapes: all zero, all `0xFF` (`-1`), alternating `0x80`-high-bit bytes, single high byte, `INT_MIN`/`INT_MAX` elements, for each `count` 1..=4 | [x] |
| 25 | `init_state` | valid state, `initial_value` random + edges (`0`, `INT_MIN`, `INT_MAX`) — compare all 12 struct bytes and stdout | [x] |
| 26 | `init_state` | pre-dirtied state buffer (non-zero garbage in all three fields) to confirm all fields are overwritten | [x] |
| 27 | `apply_operation` | fresh state (via `init_state`), `func = get_operation(0..3)`, random `value` — struct bytes + stdout | [x] |
| 28 | `apply_operation` | repeated application: 1, 2, and 10 successive calls mixing all four ops, random values (accumulates `accumulator` and `operation_count`) | [x] |
| 29 | `apply_operation` | state with non-zero `checksum` preset — confirm `checksum` is left untouched while `accumulator`/`operation_count` change | [x] |
| 30 | `apply_operation` | **cross-library** function pointer (C op into Rust `apply_operation` and vice versa) | [x] |
| 31 | manual pipeline | replicate `checkshift`'s composition by hand from the low-level exports (`init_state` → `apply_operation`×2 → `execute_operation`×2 → `compute_checksum`), random params — asserts the composed intermediate state matches, not just the final wrapper | [x] |
| 32 | `checkshift` | random `(p1,p2,p3,p4)` full `i32` range, 4000 cases — return value + complete stdout transcript | [x] |
| 33 | `checkshift` | all-zero params `(0,0,0,0)` | [x] |
| 34 | `checkshift` | edge grid of `INT_MIN`/`INT_MAX`/`-1`/`1`/`0` in all four positions (5^4 = 625 combinations) | [x] |
| 35 | `checkshift` | small-magnitude params `[-16,16]` (no-overflow regime, exercises `%04X` checksum formatting with small values) | [x] |
| 36 | `checkshift` | params chosen so the checksum has fewer than 4 hex digits (`0x%04X` zero padding) and so it has exactly 4 | [x] |
