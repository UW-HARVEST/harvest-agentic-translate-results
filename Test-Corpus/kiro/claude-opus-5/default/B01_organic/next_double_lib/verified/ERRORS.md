# ERRORS.md — Phase A: error-surface table

Derived mechanically. The grep that produced this table:

```
grep -nE 'return|assert|NULL|errno|if|switch|#if|\?|-1|abort|exit|malloc|free' \
     c_src/src/lib.c c_src/include/lib.h
```

## Mechanical findings

| construct searched for | occurrences in C |
|---|---|
| `RETURN_ERROR` / error macro | 0 |
| `return -1` / `return NULL` / error sentinel | 0 (the only `return`s are `return x + y;` and `return *(double *)&result - 1.0;`, both unconditional value returns) |
| `assert` | 0 |
| `if` / `switch` / ternary / `#if` branch | 0 — both functions are straight-line code with no conditional at all |
| explicit range / bounds check | 0 |
| NULL check | 0 |
| error `enum` / status type | 0 (no enums exist in the API) |
| min/max constant | 0 declared; the only literal constants are the shift amounts `23, 17, 26, 52, 12` and `1023`, `1.0` — all unconditional arithmetic, not limits |
| `errno` / `abort` / `exit` | 0 |
| `malloc` / `free` (allocation-failure path) | 0 |

**The C library has NO explicit rejection path.** `next_double` accepts every
possible `cn_rnd_t` state (all 2^128 bit patterns are valid input; there is no
"bad seed" the C rejects — not even all-zeros) and always returns a `double`.
`uint64_t` arithmetic is defined modulo 2^64, so there is no overflow trap, and
all shift counts (23, 17, 26, 52, 12) are `< 64`, so no shift is UB.

Consequently the table below has exactly one row: the sole way to make the C
misbehave is to hand it a pointer it may not dereference, because it
dereferences `rnd` unconditionally.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `next_double` | `rnd == NULL` — there is no NULL guard; `rnd->state[0]` on line 4 dereferences it unconditionally | Undefined behaviour; in practice a fatal `SIGSEGV` (signal 11) from the read at offset 0. **Rust must behave identically: also die by `SIGSEGV`, and must NOT return a value, and must NOT report a distinct/graceful error** (adding a NULL check would be "fixing" the C). Verified out-of-process in `tests/differential.rs::row_e1_null_pointer_both_segfault`. |

## Generic FFI boundaries also covered (not C rejections, but required by the task)

These are *not* rejection paths in the C — they are inputs the C accepts and
must therefore be accepted identically by Rust. They are tested in Phase C so
the "one step past a valid range" / "no valid variant" class of bug cannot hide.

| # | boundary probed | why the C accepts it | test |
|---|-----------------|----------------------|------|
| G1 | `state = {0, 0}` (degenerate all-zero seed, the fixed point of the generator) | no seed validation; generator stays at 0 forever and every call returns `+0.0` | `boundary_g1_zero_state_fixed_point` |
| G2 | `state = {u64::MAX, u64::MAX}` (all-ones, maximal seed) | modular arithmetic, no overflow check | `boundary_g2_all_ones_state` |
| G3 | one word zero / other word extreme (`{0, MAX}`, `{MAX, 0}`, `{0,1}`, `{1,0}`) | no relation between the two words is validated | `boundary_g3_mixed_extremes` |
| G4 | single bit set in each of the 128 state-bit positions (128 cases) — exercises every bit that the `<<23`, `>>17`, `>>26` shifts can move in or out | all bit patterns valid | `boundary_g4_single_bit_walk` |
| G5 | values chosen so `value >> 12 == 0` and `== 0xFFFFFFFFFFFFF` (mantissa min/max → result exactly `0.0` and the largest `double < 1.0`) — one step past these is impossible, the field is exactly 52 bits wide | mantissa is masked by construction, never checked | `boundary_g5_mantissa_extremes` |
| G6 | out-of-range "enum" value across FFI | **N/A — the API declares no enum and takes no `int`/flag parameter.** The only parameter is `cn_rnd_t *`. There is no integer variant space to push out of range; the pointee's 128 bits are *all* valid variants (see G1–G4, which walk that space). Recorded explicitly so the omission is a derived fact, not an oversight. | — |
| G7 | zero-length / oversized length argument | **N/A — the API has no length/size parameter.** `cn_rnd_t` is a fixed 16-byte struct; there is no count to make 0 or oversized. Recorded explicitly. | — |
| G8 | misaligned `cn_rnd_t *` (pointer 1 byte past an aligned allocation) | C does no alignment check; on x86-64 the unaligned 8-byte loads succeed | `boundary_g8_misaligned_pointer` |
| G9 | repeated/aliased calls on the same object (state mutation is observable) | the function mutates `*rnd` in place; the "input" for call *n* is the output state of call *n-1* | `boundary_g9_long_stream` (also Phase B) |

## Phase C results

| # / id | test | status |
|---|---|---|
| 1 | `row_e1_null_pointer_both_segfault` | [x] PASS — both `.so`s die by SIGSEGV (signal 11). Independently reconfirmed outside the Rust harness: a ctypes driver calling `next_double(NULL)` exits `139` (`128+11`) for the C `.so` and for the release Rust `.so`. |
| G1 | `boundary_g1_zero_state_fixed_point` | [x] PASS |
| G2 | `boundary_g2_all_ones_state` | [x] PASS |
| G3 | `boundary_g3_mixed_extremes` | [x] PASS |
| G4 | `boundary_g4_single_bit_walk` | [x] PASS |
| G5 | `boundary_g5_mantissa_extremes` | [x] PASS |
| G6 | N/A — no enum / no integer parameter in the API (derived above) | [x] N/A |
| G7 | N/A — no length/size parameter in the API (derived above) | [x] N/A |
| G8 | `boundary_g8_misaligned_pointer` | [x] PASS **after a fix** (see below) |
| G9 | `boundary_g9_long_stream` | [x] PASS |

### Divergence found and fixed (G8)

The original Rust took `let rnd: &mut cn_rnd_t = &mut *rnd;`. Forming a Rust
reference requires 8-byte alignment, so a misaligned-but-valid `cn_rnd_t *`
aborted the Rust side ("misaligned pointer dereference … aborting") while the C
performed the unaligned 8-byte loads and returned a value. Fixed in
`src/lib.rs` by reading and writing the two state words with
`core::ptr::read_unaligned` / `write_unaligned` through a raw `*mut u64`, which
is what the C compiler emits. Identical code and results for aligned pointers;
G8 now passes for both the debug and the release `.so`.

### Profile caveat for row 1

`row_e1` compares the C `.so` against the **release** Rust `.so`. A debug-profile
Rust `.so` carries rustc's optional UB checks, which turn `next_double(NULL)`
into a `SIGABRT` panic before the CPU faults; no `core::ptr` API can avoid that
under `debug_assertions`. That is build instrumentation, not translation
behaviour, and the artifact an external caller links against (`cdylib`,
`panic = "abort"`, release) faults exactly like C. Every **valid**-input test is
still run against both profiles' `.so`s.
