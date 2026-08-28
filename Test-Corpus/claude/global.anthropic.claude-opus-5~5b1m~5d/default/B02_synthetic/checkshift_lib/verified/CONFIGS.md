# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes the
C code in `c_src/src/lib.c` actually branches on / is value-sensitive to.

## Axes the C actually distinguishes

**A1 — Entry point.** All 10 exported functions, *including the lowest-level
ones* (`multiply_with_static`, `add_with_static`, `xor_operation`,
`shift_with_static`), the mid-level dispatch/driver layer (`get_operation`,
`execute_operation`, `init_state`, `apply_operation`, `compute_checksum`), and the
one-shot convenience wrapper (`checkshift`). `checkshift` alone can never cover
the others: it only ever calls `get_operation(0..3)` and `compute_checksum(p, 4)`.

**A2 — Opcode / operation selector** (`get_operation`, and which `operation_func`
is passed to `execute_operation` / `apply_operation`): `0` multiply, `1` add,
`2` xor, `3` shift. Each selects a different arithmetic kernel.

**A3 — Integer value shape.** The kernels are value-sensitive with signed
overflow / sign-extension behaviour: `0`, `±1`, small positive, small negative,
`INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`, powers of two, values with the
top bits set (matters for `a << 2` truncation), values with the low 2 bits set
(matters for the arithmetic `b >> 2`), and full-range random values.

**A4 — `compute_checksum` count / shape**: `count` = `1`, `2`, `3`, `4`
(each changes `nbytes = 4*count` and therefore the number of
`checksum = (checksum<<1) ^ byte` rounds), and `count > 4` (clamped to 4). Byte
patterns matter: all-zero, all-`0xFF`, `0x80000000` (sign bit),
little-endian-sensitive values, random.

**A5 — `ComputeState` lifecycle**: freshly `init_state`d vs. already mutated by N
`apply_operation` calls (`operation_count` accumulates, `accumulator` chains), and
the `checksum` field written independently.

**A6 — `op_name` string shape** for `execute_operation` (echoed via `%s` twice):
normal ASCII, empty, long, embedded `%` characters.

**A7 — Function-pointer provenance** (FFI-specific): the `operation_func` given
to `execute_operation` / `apply_operation` may come from the *same* library or
from the *other* library. Both must behave identically.

**A8 — `checkshift` 4-tuple**: the four params flow into different kernels
(`param1`→accumulator seed, `param2`→multiplier *and* shift operand, `param3`→
addend, `param4`→xor operand) plus all four into the checksum, so the tuple shape
matters as a whole.

There are **no** runtime option flags, no global setters, no `#ifdef`s, and no
Cargo features in this library — `grep` for `[features]`, `cfg(`, `#ifdef`
returns nothing. The `static_multiplier`/`static_addend`/`static_shift_amount`
file-statics are never written after initialisation, so they are fixed
configuration (3 / 100 / 2), not an axis.

## Configuration rows

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `multiply_with_static` | 2000 random `(a,b)` over the full `i32` range — exercises `(a*b)*3` signed overflow wrapping | [x] |
| C2 | `multiply_with_static` | boundary grid: `{0,±1,±2,INT_MAX,INT_MIN,INT_MAX-1,INT_MIN+1,0x40000000,±0xFFFF}` × itself | [x] |
| C3 | `add_with_static` | 2000 random `(a,b)`; `(a+b)+100` overflow wrapping | [x] |
| C4 | `add_with_static` | boundary grid (same set as C2) — hits `INT_MAX+100` and `INT_MIN-100` wrap | [x] |
| C5 | `xor_operation` | 2000 random `(a,b)`; `a^b^0xABCD` | [x] |
| C6 | `xor_operation` | boundary grid, incl. values whose low 16 bits collide with `0xABCD` | [x] |
| C7 | `shift_with_static` | 2000 random `(a,b)`; `(a<<2) \| (b>>2)` — left shift truncation of the 2 high bits | [x] |
| C8 | `shift_with_static` | `a` shaped for shift-out: `INT_MIN`, `INT_MAX`, `0x60000000`, `0xC0000000`, `±1`, and `b` negative → **arithmetic** `>>` sign extension | [x] |
| C9 | `shift_with_static` | boundary grid (same set as C2), both operands | [x] |
| C10 | `get_operation` | every valid opcode `0,1,2,3`: returned pointer non-NULL in both libs, and the returned kernel produces identical results for 500 random `(a,b)` each | [x] |
| C11 | `get_operation` | called repeatedly (exercises the C lazy `static ops[4]` init-on-first-use path at L69: first call initialises, later calls take the already-initialised branch) | [x] |
| C12 | `execute_operation` | each of the 4 opcodes × 300 random `(a,b)`, `op_name="XOR"` — asserts return value **and** the 3 printf lines (`Variable a`, `Variable b`, `Result of`) byte-exact | [x] |
| C13 | `execute_operation` | **cross-library** function pointers (A7): C's `get_operation(k)` result passed to Rust's `execute_operation` and vice versa, all 4 opcodes | [x] |
| C14 | `execute_operation` | `op_name` shapes (A6): normal, empty `""`, long 200-char, containing `%d`/`%s`/`%%` | [x] |
| C15 | `init_state` | `initial_value` boundary grid + 500 random — asserts all 3 struct fields (`accumulator`, `operation_count==0`, `checksum==0`) and stdout | [x] |
| C16 | `init_state` | re-init over a dirty (already mutated) state — must reset `operation_count` and `checksum` | [x] |
| C17 | `apply_operation` | each of the 4 opcodes × 300 random `value` on a freshly `init_state`d state — asserts full struct after the call | [x] |
| C18 | `apply_operation` | **cross-library** function pointers (A7), all 4 opcodes | [x] |
| C19 | `apply_operation` | chained sequence of 25 random `(opcode, value)` steps on one state (A5) — `accumulator` chains, `operation_count` accumulates to 25 | [x] |
| C20 | `apply_operation` | chain seeded with `INT_MIN`/`INT_MAX` and multiply-heavy opcodes to force repeated overflow | [x] |
| C21 | `compute_checksum` | `count=1` × 500 random 1-element arrays | [x] |
| C22 | `compute_checksum` | `count=2` × 500 random 2-element arrays | [x] |
| C23 | `compute_checksum` | `count=3` × 500 random 3-element arrays | [x] |
| C24 | `compute_checksum` | `count=4` × 500 random 4-element arrays | [x] |
| C25 | `compute_checksum` | special byte patterns (A4): all-zero, all-`0xFF`, `0x80000000`, `0x000000FF`, `0xFF000000`, alternating `0xAA55AA55`, endianness-sensitive `0x01020304` — for every `count` 1..4 | [x] |
| C26 | `compute_checksum` | `count` in `5..16` and `INT_MAX` with an over-long backing array — clamp path (also E10), result must equal `count=4` | [x] |
| C27 | `checkshift` | 2000 random 4-tuples over the full `i32` range — return value **and** the full ~15-line stdout transcript byte-exact | [x] |
| C28 | `checkshift` | boundary 4-tuples: all combinations from `{0,1,-1,INT_MAX,INT_MIN}` (625 tuples) — overflow in every stage | [x] |
| C29 | `checkshift` | tuples chosen so the checksum's `0x%04X` formatting is exercised across widths (`0x0000`, `<0x1000`, `>=0x1000`) | [x] |
| C30 | end-to-end pipeline, low-level API | hand-composed pipeline replicating `checkshift` out of `init_state` + `get_operation` + `apply_operation` + `execute_operation` + `compute_checksum` called directly across the FFI, on 500 random 4-tuples, with the state struct compared after **every** step (catches divergence invisible to per-wrapper tests) | [x] |
| C31 | all 10 entry points | randomized interleaved fuzz driver: 3000 random operations chosen uniformly across every entry point, sharing state, comparing return values + struct + stdout at each step | [x] |

## Additional ABI-level rows (Phase D hardening, `tests/phase_d_hardening.rs`)

Properties a real consumer can depend on that the rows above do not force on
their own.

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| H1 | `get_operation` | returned pointer must be the ADDRESS OF THE EXPORTED SYMBOL (in C, `ops[0] = multiply_with_static` makes `get_operation(0) == &multiply_with_static` an exact equality), and the 4 opcodes must map to 4 distinct functions | [x] |
| H2 | `execute_operation`, `apply_operation` | a caller-supplied `operation_func` belonging to NEITHER library: must be invoked exactly once, with the exact arguments, via the C ABI | [x] |
| H3 | `compute_checksum` | **unaligned** `int*` (offsets 0..7 into a byte buffer) × counts 1..4 — the C only ever reaches it through `memcpy`, so it is well defined | [x] |
| H4 | `init_state`, `apply_operation` | **unaligned** `ComputeState*` (offsets 1..3) — **this row found a genuine divergence, see FINDINGS.md #2** | [x] |
| H5 | `execute_operation` + `compute_checksum` | re-entrancy: the library called again from inside a callback it is currently executing | [x] |

## Completion

All 31 configuration rows plus the 5 hardening rows are exercised via both
`.so`s through `libloading` and asserted byte-for-byte (return values, full
`ComputeState` bytes including a poison guard, and complete stdout transcripts),
with a fixed RNG seed for reproducibility, under both the `debug` and `release`
profiles.

Test files: `tests/phase_b_kernels.rs` (C1–C9), `tests/phase_b_dispatch.rs`
(C10–C14), `tests/phase_b_state.rs` (C15–C20), `tests/phase_b_checksum.rs`
(C21–C26), `tests/phase_b_pipeline.rs` (C27–C31),
`tests/phase_d_hardening.rs` (H1–H5).

Run: `cargo test` (or `./verify.sh` for the full matrix).
