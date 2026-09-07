# ERRORS.md — Error-surface table

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical grep of the entire library

```
$ grep -nE 'return|assert|NULL|errno|error|ERROR|goto|exit|abort|-1|MIN|MAX|LIMIT' \
        c_src/include/lib.h c_src/src/lib.c
include/lib.h:1:#include <stdint.h>      <- only hit: the include line
$ grep -nE 'if|switch|while|for|#if|\?|\|\||&&' c_src/include/lib.h c_src/src/lib.c
<no hits>
```

Findings, and what each rules out:

| construct searched for | occurrences | consequence |
|---|---|---|
| `return` statement (value or bare) | 0 | function is `void`; there is **no return value to compare** |
| error-return macro (`RETURN_ERROR`-style) | 0 | no macro error surface |
| `assert` / `abort` / `exit` | 0 | no runtime trap |
| `NULL` comparison / null check | 0 | pointers are dereferenced unconditionally |
| `if` / `switch` / `?:` / `&&` / `\|\|` | 0 | **straight-line code, single basic block** |
| loops (`for` / `while`) | 0 | no iteration bounds to overflow |
| `#if` / `#ifdef` | 0 | no compile-time variants |
| `enum` declarations | 0 | **no enum crosses the FFI boundary** |
| length / count / size parameter | 0 | output length is fixed at 16 by the signature |
| min/max constant, range check | 0 | every bit pattern of every input is in range |

**Therefore the library rejects nothing.** `md5_digest` has an empty
rejection surface: `void md5_digest(const tflac_md5*, tflac_u8[16])` accepts
all 2^128 struct values, always writes exactly 16 bytes, and reports no status.
There are zero `RETURN_ERROR`-equivalent branches, so there are zero
"function-returns-error-code" rows to write. Any row claiming otherwise would
be invented rather than derived, which the task forbids.

## Rows actually present

The only inputs the C does *not* define behavior for are the generic C-API
boundaries (undefined behavior, not library-level rejection). These are real
inputs a caller can pass, so each still gets a differential test asserting C
and Rust behave identically — the assertion is on the *observed process
outcome* (signal / exit status), compared side by side, not merely "both
failed".

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `md5_digest` | `m == NULL`, `out` valid — unconditional read of `m->a` at address 0 | no check exists; page fault → process killed by `SIGSEGV` (11) | `err_e1_null_m` |
| E2 | `md5_digest` | `out == NULL`, `m` valid — unconditional store to `out[0]` at address 0 | no check exists; page fault → `SIGSEGV` (11) | `err_e2_null_out` |
| E3 | `md5_digest` | both `m == NULL` and `out == NULL` | `SIGSEGV` (11), faulting on the `m->a` read first | `err_e3_both_null` |
| E4 | `md5_digest` | `out` buffer shorter than 16 bytes (undersized length) — C writes indices 0..15 regardless | writes exactly 16 bytes, overrunning the caller's buffer; no truncation, no error | `err_e4_writes_exactly_16` (guard-page: 16-byte-exact mapping succeeds, 15-byte-exact faults, identically for C and Rust) |
| E5 | `md5_digest` | `out` buffer larger than 16 bytes (oversized length) | writes exactly bytes 0..15; bytes ≥16 left untouched — no error, no extra writes | `err_e5_no_overwrite_past_16` |
| E6 | `md5_digest` | value "one step past a documented valid range" | **N/A — vacuous.** Both parameters are unbounded: `tflac_u32` fields have no documented valid range (all 2^32 values valid) and there is no length/index/count argument. Exhaustively covered instead by the boundary values in `CONFIGS.md` rows C1–C9. | `cfg_*` |
| E7 | `md5_digest` | out-of-range enum value passed across the FFI boundary | **N/A — vacuous.** `grep -c enum` over `include/lib.h` + `src/lib.c` = 0. The API takes no enum, no int-typed mode, and no flag word, so no invalid-variant input exists to construct. | — |

Rows E6 and E7 are recorded as explicitly-checked-and-absent rather than
omitted, so the absence is a derived result and not an oversight.

## Checklist

- [x] E1 `m == NULL`
- [x] E2 `out == NULL`
- [x] E3 both NULL
- [x] E4 undersized `out` (writes 16 regardless)
- [x] E5 oversized `out` (writes no more than 16)
- [x] E6 past-valid-range scalar — N/A, no bounded scalar exists (verified by grep)
- [x] E7 out-of-range enum — N/A, no enum in the API (verified by grep)

## Results

All 7 rows pass against both the debug and release Rust `.so`, under both
`cargo test` and `cargo test --no-default-features`. Rows E1–E4 run the call in
a `fork()`ed child and compare the child's exact terminating signal / exit
status, so a pass means the two libraries produced the *same specific* outcome,
not merely that both failed.

Measured outcomes (identical for C and Rust):

| row | C outcome | Rust outcome |
|---|---|---|
| E1 `m == NULL` | `Signaled(SIGSEGV=11)` | `Signaled(SIGSEGV=11)` |
| E2 `out == NULL` | `Signaled(SIGSEGV=11)` | `Signaled(SIGSEGV=11)` |
| E3 both NULL | `Signaled(SIGSEGV=11)` | `Signaled(SIGSEGV=11)` |
| E4 `out` slack = 16 (fits) | `Exited(0)` | `Exited(0)` |
| E4 `out` slack ∈ {1,2,3,4,8,12,15} | `Signaled(SIGSEGV=11)` | `Signaled(SIGSEGV=11)` |
| E5 oversized `out` | 16 bytes written, tail intact | identical, byte-for-byte |

### Divergence found and fixed — rows E1 and E3

The first fix for the misalignment problem used
`core::ptr::read_unaligned::<u32>`, which expands to `copy_nonoverlapping` and
therefore carries a precondition check:

```
unsafe precondition(s) violated: ptr::copy_nonoverlapping requires that both
pointer arguments are aligned and non-null ...
```

With a NULL `m` that check aborted **before** the faulting load, so the child
died with `SIGABRT` (6) while the C died with `SIGSEGV` (11):

```
[E1 m==NULL] C gave Signaled(11) but Rust gave Signaled(6)
[E3 both NULL] C gave Signaled(11) but Rust gave Signaled(6)
```

Fixed two ways, so no configuration diverges:

1. The Rust now uses a plain single-byte raw dereference instead of any
   `core::ptr::read*` helper, so no `copy_nonoverlapping` precondition check is
   emitted and a NULL pointer produces a genuine faulting load.
2. `[profile.dev]` sets `debug-assertions = false` / `overflow-checks = false`.
   rustc's optional UB checks (`"null pointer dereference occurred"`,
   `"misaligned pointer dereference"`) convert exactly the faulting loads this
   library must reproduce into `SIGABRT` panics. They are absent from the
   release artifact a consumer `dlopen`s, and disabling them for the dev
   artifact makes the differential suite return the same verdict under every
   profile rather than only under `--release`.

### E6/E7 vacuity is machine-checked

`err_e6_e7_no_bounded_scalar_and_no_enum_in_api` re-reads
`c_src/include/lib.h` and `c_src/src/lib.c` at test time and asserts they
contain no `enum` and none of `if `, `if(`, `switch`, `assert`, `return `,
`return;`, `NULL`. So the claim that there is no bounded scalar and no enum to
push out of range is re-derived on every run instead of merely asserted here —
if the C ever gains a check, this test fails and forces the table to be redone.
