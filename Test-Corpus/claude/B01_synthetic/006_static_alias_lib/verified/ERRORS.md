# ERRORS.md — Error-surface table

Mechanically derived from every line of `c_src/src/staticalias.c` and
`c_src/include/staticalias.h`.

## Mechanical grep result (the important part)

```
$ grep -nE 'return|assert|NULL|errno|exit\(|abort|if *\(' c_src/src/staticalias.c
28:static_alias(int *outer) {
30:  if(*outer >= inner) {
32:    return &inner;
35:    return outer;
49:  return;
```

The C library contains:

* **0** error-return macros (`RETURN_ERROR` &c.) — none exist in the tree
* **0** `assert` / `abort` / `exit` calls
* **0** `NULL` checks (the string `NULL` does not appear in the sources)
* **0** explicit range / bounds checks
* **0** error enums, error codes, or `errno` use
* **0** min/max constants
* **0** functions that can return a failure sentinel: `static_alias` returns a
  *always-non-NULL* `int*` (either `&inner` or its own non-checked argument),
  and `driver` returns `void`

There is therefore **no explicit rejection surface at all**: the API never
validates and never reports failure. Both `return` statements in `static_alias`
are success paths (they are rows 1–2 of `CONFIGS.md`, not error rows).

Because there are no error codes to compare, the rows below are the *implicit*
rejection/boundary behaviours that this C API does have — the generic boundaries
the task requires (null pointers, zero/negative/oversized lengths, values one
step past a valid range, out-of-range enum values). Each row is still a
differential test: C and Rust must behave **identically**, including which of
them crashes and with what signal.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `static_alias` | `outer == NULL` — line 30 dereferences `*outer` with no null check | no rejection: NULL read faults → process dies on `SIGSEGV` (11). Rust must fault the same way, **not** panic/unwind with a Rust message | `err_e1_null_pointer_segv_both` | [x] |
| E2 | `static_alias` | `outer` is a wild/unmapped non-NULL pointer (e.g. `0x1`) | no rejection: `SIGSEGV` (11), same as C | `err_e2_wild_pointer_segv_both` | [x] |
| E3 | `static_alias` | `*outer == INT_MAX` while `inner >= 1` → line 31 `inner += *outer` **signed overflow** (UB), one step past the representable range | no rejection: wraps 2's-complement; returns `&inner` holding the wrapped value | `err_e3_inner_add_overflow_intmax` | [x] |
| E4 | `static_alias` | repeated aliased calls (`outer == &inner`) drive `inner += inner` until it overflows `INT_MAX` | no rejection: doubles and wraps, eventually reaching a fixed point at `0` | `err_e4_aliased_doubling_overflow_to_zero` | [x] |
| E5 | `static_alias` | `*outer == INT_MIN` with `inner > INT_MIN` → else branch, line 34 `*outer += inner` **signed overflow** (UB) at the bottom of the range | no rejection: wraps; returns `outer` (the caller's pointer), `*outer` wrapped | `err_e5_outer_add_overflow_intmin` | [x] |
| E6 | `static_alias` | `*outer == inner - 1` — the value exactly one step below the `>=` boundary | no rejection: takes the else branch, returns `outer` | `err_e6_one_below_branch_boundary` | [x] |
| E7 | `driver` | `iterations == 0` — "zero length" | no rejection: loop body never runs, **no bytes printed**, `inner` unchanged | `err_e7_zero_iterations_no_output` | [x] |
| E8 | `driver` | `iterations < 0` (incl. `INT_MIN`) — "negative length"; `for (i = 0; i < iterations; i++)` is false immediately | no rejection: loop body never runs, no output, `inner` unchanged | `err_e8_negative_iterations_no_output` | [x] |
| E9 | `driver` | `initial_value == INT_MIN` / `INT_MAX` — extreme scalar args, overflow inside the loop | no rejection: wraps; printed bytes must match exactly | `err_e9_extreme_initial_values` | [x] |
| E10 | `driver` | "oversized length": `iterations` far larger than any overflow-free run (e.g. 200), so every arithmetic op wraps repeatedly | no rejection: prints `iterations` lines, all matching | `err_e10_oversized_iterations` | [x] |
| E11 | both | **out-of-range "enum"/int values across the FFI boundary.** The API declares no `enum`, so the widest int domain *is* the accepted domain: every 32-bit bit pattern is a legal `int` argument. Passing arbitrary/garbage `int`s (incl. values that no sensible caller would use) must not be rejected by either side | no rejection: pure arithmetic on the value; C and Rust agree for all sampled bit patterns | `err_e11_arbitrary_int_bit_patterns` | [x] |

Notes on rows E3–E5, E9–E10: signed-overflow is UB in C, so "expected C result"
is *the behaviour of the actually-compiled C `.so`* (gcc, default cmake flags),
which is what the differential tests observe. The Rust translation uses
`wrapping_add`, matching it; the tests assert this empirically rather than
assuming it.

**Gate: every row E1–E11 has a passing differential test. PASS.**

## Divergence found and fixed (rows E1, E2)

The error-path phase caught one real translation bug — invisible to every
happy-path test, and exactly the class of blind spot this table exists to find.

| | `static_alias(NULL)` | `static_alias(0x1)` |
|---|---|---|
| C | dies on `SIGSEGV` (11) | dies on `SIGSEGV` (11) |
| Rust *before* fix | dies on `SIGABRT` (6) | dies on `SIGABRT` (6) |
| Rust *after* fix | `SIGSEGV` (11) | `SIGSEGV` (11) |

Cause: the translation read the caller's pointer with a plain `*outer`. Rust
1.78+ emits a *debug-assertion* validity precondition on raw-pointer
dereferences, so with `-Cdebug-assertions` (the default `dev` profile) a bad
pointer produced

```
panicked at src/lib.rs:64: null pointer dereference occurred
thread caused non-unwinding panic. aborting.
```

i.e. `SIGABRT`, where the C simply faults with `SIGSEGV`. Every Rust accessor
carries some precondition, so none of them reproduces C's raw load:

| access form | `outer == NULL` | `outer == 0x1` (misaligned) |
|-------------|-----------------|------------------------------|
| `*outer` | SIGABRT | SIGABRT |
| `ptr::read_volatile` | SIGSEGV | SIGABRT (alignment check) |
| `ptr::read_unaligned` | SIGABRT (`copy_nonoverlapping` non-null check) | SIGSEGV |
| `extern "C" memcpy` | **SIGSEGV** | **SIGSEGV** |
| C's `*outer` | SIGSEGV | SIGSEGV |

Fix: `src/lib.rs` now routes the caller-pointer load/store through `c_load` /
`c_store`, thin wrappers over libc `memcpy`, which carry no Rust-inserted checks
at any optimization level. `inner` is the library's own always-valid static, so
it still uses a direct access. Verified check-free at `-C opt-level=0` and `3`.
