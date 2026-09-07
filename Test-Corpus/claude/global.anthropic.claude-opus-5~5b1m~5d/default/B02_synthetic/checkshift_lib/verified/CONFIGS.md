# CONFIGS.md — Configuration surface table (Phase B)

## Axes the C code actually branches on / is value-sensitive to

There are no runtime option setters in this library (the three `static int`
"knobs" — `static_multiplier=3`, `static_addend=100`, `static_shift_amount=2` —
are file-static with no exported mutator, so they are fixed configuration). The
real axes are therefore **which entry point** is called and **the shape/value
class of the inputs**:

- **A1 — entry point**: the 10 exported functions, including the four lowest-level
  op functions (`multiply_with_static`, `add_with_static`, `xor_operation`,
  `shift_with_static`), the dispatcher `get_operation`, the two state functions
  (`init_state`, `apply_operation`), the printing wrapper `execute_operation`,
  the checksum helper `compute_checksum`, and the one-shot driver `checkshift`.
- **A2 — signed value class**: `0`, small positive, small negative, `INT_MAX`,
  `INT_MIN`, values that make `a*b` / `a+b` overflow, values with the sign bit
  set feeding `<<` (`shift_with_static`).
- **A3 — `get_operation` opcode**: `0,1,2,3` (in range) — one row per opcode,
  since each selects a different function pointer.
- **A4 — `compute_checksum` count shape**: `1`, `2`, `3`, `4` (each changes
  `sizeof(int)*copy_count` loop length) and `> 4` (clamped to 4 → identical to
  `count == 4`).
- **A5 — `compute_checksum` byte patterns**: all-zero ints, ints with high bytes
  set, negative ints (`0xFF` bytes), values chosen so `checksum << 1` overflows
  32 bits, and values where the pre-mask result differs but the masked result
  collides.
- **A6 — `apply_operation` sequencing**: 0/1/many calls, and the `operation_count`
  and `accumulator` carry-over across a chain of *different* ops (this is the
  composed-pipeline path that per-function tests miss).
- **A7 — `execute_operation` `op_name`**: normal string, empty string, string
  containing `%` (format-string hazard for `%s`), long string.
- **A8 — stdout stream**: every row that prints must match byte-for-byte, not
  just in return value. Captured by redirecting fd 1 around each call.
- **A9 — `checkshift` end-to-end**: return value **and** the full multi-line
  stdout transcript, over all of A2 for the four parameters.

There are no `#ifdef` branches and no `[features]` in `Cargo.toml`, so the
feature axis has a single value (default == `--no-default-features`).

## Rows

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `multiply_with_static` | randomized `(a,b)` over full `i32` range, seeded; includes overflow of `a*b` and of `*3` | [x] |
| 2 | `multiply_with_static` | boundary grid: `{0, 1, -1, 2, INT_MAX, INT_MIN, INT_MAX/3, INT_MIN/3}²` | [x] |
| 3 | `add_with_static` | randomized `(a,b)` full range, seeded; includes `a+b` overflow and `+100` overflow past `INT_MAX` | [x] |
| 4 | `add_with_static` | boundary grid incl. `INT_MAX-99`, `INT_MAX-100`, `INT_MIN`, `0` | [x] |
| 5 | `xor_operation` | randomized `(a,b)` full range; plus `a=0`, `b=0`, `a=b`, `a=0xABCD`, `INT_MIN`, `-1` | [x] |
| 6 | `shift_with_static` | randomized `(a,b)` full range — exercises `(a<<2)` losing top 2 bits, sign-bit-set `a`, and arithmetic `(b>>2)` for negative `b` | [x] |
| 7 | `shift_with_static` | boundary grid: `a ∈ {0,1,-1,INT_MAX,INT_MIN,0x40000000,0x20000000}` × `b ∈ {0,1,-1,3,-3,-4,INT_MAX,INT_MIN}` (negative-`b` rounding of `>>`) | [x] |
| 8 | `get_operation` | opcode `0` → pointer is non-NULL and behaves as `multiply_with_static` (verified by calling the returned pointer on randomized args) | [x] |
| 9 | `get_operation` | opcode `1` → behaves as `add_with_static` (randomized args) | [x] |
| 10 | `get_operation` | opcode `2` → behaves as `xor_operation` (randomized args) | [x] |
| 11 | `get_operation` | opcode `3` → behaves as `shift_with_static` (randomized args) | [x] |
| 12 | `get_operation` | repeated calls (lazy `static ops[]` init path: first call vs. subsequent calls return the same pointer) | [x] |
| 13 | `execute_operation` | valid func from each opcode `0..3`, randomized `(a,b)`, `op_name="XOR"` — compares return value **and** the 3 printed lines | [x] |
| 14 | `execute_operation` | `op_name` shapes: `""`, `"%d%s"`, 200-char string, with a valid func | [x] |
| 15 | `execute_operation` | Rust func pointer passed to the **C** lib and C func pointer passed to the **Rust** lib (cross-library callback through the `operation_func` typedef) | [x] |
| 16 | `compute_checksum` | `count = 1`, randomized `int` values (4-byte loop) | [x] |
| 17 | `compute_checksum` | `count = 2`, randomized (8-byte loop) | [x] |
| 18 | `compute_checksum` | `count = 3`, randomized (12-byte loop) | [x] |
| 19 | `compute_checksum` | `count = 4`, randomized (16-byte loop, full buffer) | [x] |
| 20 | `compute_checksum` | `count > 4` (`5`, `100`, `INT_MAX`) with a 4-element array → clamped, must equal the `count = 4` result | [x] |
| 21 | `compute_checksum` | byte-pattern edges: all `0`, all `-1` (`0xFF…`), `INT_MIN`, `INT_MAX`, `0x01000000`-style single-high-byte values, and values engineered so the 16-bit mask hides differing high bits | [x] |
| 22 | `init_state` | randomized `initial_value` incl. `0`, `INT_MIN`, `INT_MAX` — compares the printed line **and** all three struct fields written into caller-owned memory | [x] |
| 23 | `init_state` | called twice on the same buffer (re-init resets `operation_count`/`checksum` to 0) | [x] |
| 24 | `apply_operation` | one call, each opcode `0..3`, randomized pre-existing `accumulator` and `value`; asserts `accumulator` and `operation_count` | [x] |
| 25 | `apply_operation` | many calls (chain of 16 randomized `(opcode, value)` pairs) on a state seeded by `init_state` — composed pipeline, `operation_count` accumulation | [x] |
| 26 | `apply_operation` | non-zero starting `operation_count` (pre-set by the caller) to confirm `++` from an arbitrary base, incl. `INT_MAX` wraparound | [x] |
| 27 | manual pipeline | `init_state` → `apply_operation`×2 → `execute_operation`×2 → `compute_checksum` driven step-by-step from the test (the same sequence `checkshift` performs, but composed by the caller) — full stdout + all intermediate values | [x] |
| 28 | `checkshift` | randomized `(p1,p2,p3,p4)` over the full `i32` range, seeded — return value + complete stdout transcript | [x] |
| 29 | `checkshift` | all-zero params | [x] |
| 30 | `checkshift` | boundary params: `INT_MAX`/`INT_MIN`/`-1`/`1` in each of the four positions | [x] |
| 31 | `checkshift` | small "documentation-style" params (`1,2,3,4`, `10,20,30,40`) | [x] |
| 32 | `checkshift` | repeated invocation (state is heap-allocated per call; second call must produce identical output to the first for the same params) | [x] |
| 33 | all entry points | run the whole suite under `--no-default-features` **and** under both the `dev` and `release` profiles (`./run_tests.sh` iterates all four) | [x] |

## Result

`./run_tests.sh` → **ALL GREEN**: 32/32 Phase-B tests + 21 Phase-C tests pass in
all four configurations (`release`/`dev` × default/`--no-default-features`), with
symbol parity 10/10 in each.

Non-vacuity was confirmed by mutation testing the Rust source (each mutation was
reverted afterwards):

| mutation | detected by |
|----------|-------------|
| `STATIC_ADDEND` 100 → 101 | 15 Phase-B tests |
| `b >> 2` arithmetic → logical shift | 12 Phase-B tests |
| removed the final `printf` of `checkshift` | 5 Phase-B tests (stdout diff) |
| dropped the `& MASK_LOWER` on the checksum | 12 Phase-B tests |
| `get_operation` range `< 4` → `<= 4` | Phase-C `err_13` |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)` — no
`add_executable`. `translation/Cargo.toml` declares only `crate-type = ["cdylib"]`
with no `[[bin]]`. There is therefore no driver binary to diff on stdout; the
stdout comparison is done at the FFI level instead (axis A8), which covers the
same output surface.
