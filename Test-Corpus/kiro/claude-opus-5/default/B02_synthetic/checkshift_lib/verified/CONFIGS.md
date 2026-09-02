# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`. The axes the C
code actually branches on / dispatches on:

**Axis O — operation selection.** `operation_func` dispatch is the library's only
runtime "mode". It is set three ways: `get_operation(opcode)` with `opcode ∈ 0..3`
(L67–81), or a function pointer handed directly to `execute_operation` (L83) /
`apply_operation` (L128). The four modes are `multiply_with_static`,
`add_with_static`, `xor_operation`, `shift_with_static`, plus the NULL mode
(see `ERRORS.md`).

**Axis S — static tuning state** (L45–47): `static_multiplier = 3`,
`static_addend = 100`, `static_shift_amount = 2`. Internal linkage, never written,
so these are fixed constants that each op path must bake in identically.

**Axis V — integer value shape.** Every op is `int`-in/`int`-out with wrapping
signed arithmetic (`*`, `+`), bitwise xor against `0xABCD`, a signed **left** shift
that discards high bits, and a signed **arithmetic right** shift. So the distinct
value shapes are: zero, ±1, small ±, high-bit-set (left-shift discard), negative
(arithmetic-shift sign fill), `INT_MAX`/`INT_MIN` (overflow), and `0xABCD` /
`0xFFFF` / `0xDEADBEEF`-related constants.

**Axis C — element count / buffer shape** for `compute_checksum` (L97–114):
`count ∈ {1,2,3,4}` (byte loop length `4*count`), plus the `> 4` clamp. The byte
loop reinterprets `int*` as `unsigned char*`, so **host byte order** is part of the
observable contract.

**Axis T — carried struct state** for `ComputeState` (L38–42): `accumulator`,
`operation_count` (incremented only on a successful `apply_operation`), `checksum`.
Distinct shapes: fresh / one op applied / many ops applied / re-initialised.

**Axis X — caller/callee library crossing.** Both `.so`s export *both* the leaf ops
and the higher-order consumers, so a real consumer can mix them: a C function
pointer into Rust's `execute_operation`/`apply_operation` and vice versa, and a
`ComputeState` buffer mutated alternately by both libraries.

**Axis F — Cargo features.** `translation/Cargo.toml` declares no `[features]`
table, so there is exactly one build configuration (default ≡
`--no-default-features`). Asserted by `tests/symbols.rs::no_cargo_features_declared`.

Every row is exercised with many randomized inputs (fixed seed, SplitMix64 —
`tests/common/mod.rs`) in addition to the listed boundary grid.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `multiply_with_static` | O=mul; V=uniform-random `(a,b)` over the full `i32` range, 20000 pairs | [x] |
| 2 | `multiply_with_static` | O=mul; V=boundary grid `{0,1,-1,2,-2,3,-3,0xABCD,0xFFFF,0x10000,INT_MAX,INT_MIN,INT_MAX/3,INT_MIN/3}²` (signed-overflow wrap) | [x] |
| 3 | `add_with_static` | O=add; V=uniform-random `(a,b)`, 20000 pairs | [x] |
| 4 | `add_with_static` | O=add; V=boundary grid incl. `INT_MAX-100`, `INT_MIN+100`, `-100` (the `static_addend` edge) | [x] |
| 5 | `xor_operation` | O=xor; V=uniform-random `(a,b)`, 20000 pairs | [x] |
| 6 | `xor_operation` | O=xor; V=boundary grid incl. `0xABCD`, `!0xABCD`, `-1`, `INT_MIN` | [x] |
| 7 | `shift_with_static` | O=shift; V=uniform-random `(a,b)`, 20000 pairs | [x] |
| 8 | `shift_with_static` | O=shift; V=`a` with high bits set (`0x2000_0000`…`0x7FFF_FFFF`, `INT_MIN`) → left-shift discard; `b < 0` → arithmetic right-shift sign fill; boundary grid | [x] |
| 9 | `get_operation` | O=all; opcode swept over `-8..=8` ∪ `{INT_MIN,INT_MAX,OP_ADD..OP_SHIFT}`: null-ness must match, and the returned pointer must compute the same result as the matching leaf export for 2000 random `(a,b)` | [x] |
| 10 | `get_operation` | repeated calls (1000×, interleaved opcodes) — exercises the lazy `static` table guard; behaviour must be stable | [x] |
| 11 | `execute_operation` | O=each of 4 via same-library `get_operation`; V=random `(a,b)`, 5000 per op; return value **and** stdout bytes compared | [x] |
| 12 | `execute_operation` | X=cross-library: C leaf ptr → Rust `execute_operation`, Rust leaf ptr → C `execute_operation`; O=each of 4; V=random | [x] |
| 13 | `execute_operation` | `op_name` shape: normal, empty `""`, 300-byte string, string with `%d` in it (must be treated as data, not format) | [x] |
| 14 | `compute_checksum` | C=1; V=random `values[1]`, 5000 draws (4-byte loop) | [x] |
| 15 | `compute_checksum` | C=2; V=random `values[2]`, 5000 draws | [x] |
| 16 | `compute_checksum` | C=3; V=random `values[3]`, 5000 draws | [x] |
| 17 | `compute_checksum` | C=4; V=random `values[4]`, 5000 draws | [x] |
| 18 | `compute_checksum` | C=clamp (`count ∈ {5,6,16,1000,INT_MAX}`) over a 4-int buffer; must equal the `count==4` result | [x] |
| 19 | `compute_checksum` | V=byte-order-sensitive patterns: `0x00000000`, `0xFFFFFFFF`, `0x01020304`, `0x000000FF`, `0xFF000000`, `0x80000000`, `MAGIC_NUMBER`, and 1-hot bytes in every one of the 16 positions | [x] |
| 20 | `init_state` | T=fresh 12-byte buffer pre-poisoned with `0xAA`; V=`initial_value` random + boundary grid; **all 12 struct bytes** compared | [x] |
| 21 | `init_state` | T=re-initialise a buffer that already holds a used state (must reset `operation_count`/`checksum` to 0) | [x] |
| 22 | `apply_operation` | O=each of 4; T=fresh state; V=random `(initial, value)`, 5000 per op; full struct bytes compared | [x] |
| 23 | `apply_operation` | T=chained `n ∈ {0,1,2,3,5,17,50}` applications with a pseudo-random op sequence; `accumulator` + `operation_count` compared after each step | [x] |
| 24 | `apply_operation` | X=cross-library leaf ptr; and X=same `ComputeState` buffer driven alternately by C's and Rust's `apply_operation` | [x] |
| 25 | `checkshift` | full pipeline; V=uniform-random `(p1,p2,p3,p4)`, 20000 quadruples | [x] |
| 26 | `checkshift` | full pipeline; V=boundary grid `{0,1,-1,2,-2,4,100,-100,0xABCD,0xFFFF,0x10000,0x40000000,INT_MAX,INT_MIN}⁴` sampled + all-same-value cases | [x] |
| 27 | `checkshift` | full pipeline; stdout bytes compared verbatim for a spread of inputs (incl. negatives and `INT_MIN`, to pin the `%d` and `0x%04X` formats) | [x] |
| 28 | low-level composition | `checkshift` re-implemented by the *test* out of `malloc` + `init_state` + `get_operation` + `apply_operation`×2 + `execute_operation`×2 + `compute_checksum`, run entirely against the C exports and entirely against the Rust exports; both must equal each library's own `checkshift` | [x] |
| 29 | low-level composition | same manual pipeline, but every stage alternated between the C and the Rust `.so` (all 2⁶ stage-assignment masks) — catches divergence that per-function tests hide | [x] |
| 30 | all 10 exports | F=default feature set (the only one); symbol parity + every row above re-run under `scripts/check_features.sh` | [x] |
| 31 | `get_operation`, 4 leaf ops, `compute_checksum` | 8 threads × 20 000 iterations concurrently against both `.so`s — the C fills its dispatch table lazily (a benign race), the Rust does not, and the Rust `.so` is built `panic = "abort"`, so it must not abort where the C proceeds | [x] |
| 32 | `compute_checksum` | V=**misaligned** `int*` at every byte offset 0..3, count 1..4, 2000 random payloads — the C reads `values` only via `memcpy`, so misalignment is a valid input shape | [x] |
| 33 | `checkshift` | allocator-call shape: N calls must produce the same number of `malloc(12)` and `free` calls on both sides (LD_PRELOAD counter). This is the regression guard for the divergence recorded in `ERRORS.md`, where the Rust allocation had been optimised away entirely | [x] |
| 34 | all 10 exports | build-profile axis: every row above re-run with the Rust `.so` built in `release` (LTO-ish, no overflow checks, `panic = "abort"`) **and** in `dev` (overflow checks on), selected via `RUST_SO` | [x] |

## Row → test traceability

| rows | test target | test |
|------|-------------|------|
| 1–10, 31 | `tests/phase_b_leaf.rs` | `cfg01`…`cfg10`, `cfg10b_concurrent_dispatch_and_leaf_ops` |
| 11–24 | `tests/phase_b_higher.rs` | `cfg11`…`cfg24` (+ `cfg09b`, `cfg19b`, `cfg19c`) |
| 25–29 | `tests/phase_b_pipeline.rs` | `cfg25`…`cfg29` (+ `cfg25b`, `cfg26b`, `cfg27b`) |
| 32 | `tests/phase_c_errors.rs` | `err_misaligned_values_pointer` |
| 33 | `tests/phase_c_malloc.rs` | `err18b_allocator_call_parity` |
| 30, 34 | `scripts/check_features.sh` | feature powerset × `{release, dev}` |

## What is compared

For every row, **both** of these must match byte-for-byte between the two `.so`s:

1. the return value(s) and, where a `ComputeState` is involved, all 12 raw struct
   bytes (not just the fields the test happens to care about);
2. the exact bytes written to file descriptor 1. Both libraries `printf` through the
   process-wide libc `stdout`, so fd 1 is redirected to a temp file around each
   call (`common::capture_stdout`) and the resulting buffers are compared. gcc
   rewrites the C library's no-vararg `printf("…\n")` calls to `puts`, which is why
   the C `.so` imports `puts`; the emitted bytes are identical either way, and this
   is asserted rather than assumed.

The three `harness = false` test targets exist because the default libtest harness
writes its own progress text to fd 1 from worker threads, which lands inside the
captured buffer and corrupts the comparison (this produced a false "divergence"
during development).
