# ERRORS.md — error / rejection surface table

Derived mechanically from every rejection-ish construct in `c_src/src/`:

```
grep -nE 'return|assert|NULL|if *\(|switch|default|exit|fprintf' c_src/src/*.c c_src/src/*.h
```

Findings: the library has **no** `RETURN_ERROR` macro, **no** `assert`, **no**
`return -1`, **no** `return NULL`, **no** error enum and **no** null-pointer
check. The complete rejection surface is:

1. `mdmain.c:29` — `if (argc < 3) { fprintf(stderr, "usage: %s A B\n", argv[0]); return 2; }`
2. `mdmacros.h:83-92` — `DISPATCH_REP`'s `switch (n)` covers only `case 0..6`;
   `default: break;` silently accepts and ignores every other `n`, so
   `accum_<OP>(n)` returns the *untouched* initial accumulator `INIT_FOR(OP)`.
3. `mdmain.c:33-34` — `atoi()` silently yields `0` for non-numeric argv and
   clamps/truncates on overflow (glibc: `(int)strtol(s, NULL, 10)`); there is no
   validation and no error return.
4. Signed-integer overflow in `op_add`/`op_sub`/`op_mul` and in the
   `STEP_add`/`STEP_sub`/`STEP_mul` accumulator steps is *not* checked; the C
   toolchain used here produces two's-complement wraparound, which the Rust must
   reproduce (`wrapping_*`) rather than panic on.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `main` (`driver` binary) | `argc == 1` (no args) | stderr `usage: <argv0> A B\n`, exit status `2`, empty stdout | [x] |
| 2 | `main` (`driver` binary) | `argc == 2` (one arg) | stderr `usage: <argv0> A B\n`, exit status `2`, empty stdout | [x] |
| 3 | `main` (`driver` binary) | `argc >= 4` (extra args) — **not** rejected, surplus ignored | normal run using only argv[1], argv[2]; exit `0` | [x] |
| 4 | `main` | argv[1] / argv[2] not numeric at all (`"abc"`, `""`, `"+"`, `"-"`) | `atoi` ⇒ `0`, no diagnostic, exit `0` | [x] |
| 5 | `main` | argv numeric with leading whitespace / sign (`" \t\n-42"`, `"+7"`) | `atoi` accepts, no diagnostic | [x] |
| 6 | `main` | argv with trailing garbage (`"12xyz"`, `"3.9"`) | `atoi` stops at first non-digit ⇒ `12`, `3` | [x] |
| 7 | `main` | argv overflowing `long` (`"99999999999999999999"`) | `strtol` clamps to `LONG_MAX`/`LONG_MIN`, truncated to `int` ⇒ `-1` / `0` | [x] |
| 8 | `main` | argv in `(INT_MAX, LONG_MAX]` (`"2147483648"`, `"4294967296"`) | no clamp; `long`→`int` truncation ⇒ `-2147483648`, `0` | [x] |
| 9 | `use_generated` → `accum_<OP>` | `n == 7` (one past the last `case`, and equal to the largest legal `REPEAT`) | `default:` ⇒ returns `INIT_FOR(OP)` = `0` (add/sub) / `1` (mul); prints `gen.acc=<init>` | [x] |
| 10 | `use_generated` → `accum_<OP>` | `n == 8` and any `n > 7` | `default:` ⇒ `INIT_FOR(OP)` | [x] |
| 11 | `use_generated` → `accum_<OP>` | `n == -1` (one below the first `case`) | `default:` ⇒ `INIT_FOR(OP)` | [x] |
| 12 | `use_generated` → `accum_<OP>` | `n == INT_MIN` | `default:` ⇒ `INIT_FOR(OP)` | [x] |
| 13 | `use_generated` → `accum_<OP>` | `n == INT_MAX` | `default:` ⇒ `INIT_FOR(OP)` | [x] |
| 14 | `op_add` | `a + b` overflows `INT_MAX` (`INT_MAX, 1`) — unchecked signed overflow | two's-complement wrap ⇒ `INT_MIN` | [x] |
| 15 | `op_add` | `a + b` underflows `INT_MIN` (`INT_MIN, -1`) | wrap ⇒ `INT_MAX` | [x] |
| 16 | `op_sub` | `a - b` overflows (`INT_MAX, -1`) | wrap ⇒ `INT_MIN` | [x] |
| 17 | `op_sub` | `a - b` underflows (`INT_MIN, 1`) | wrap ⇒ `INT_MAX` | [x] |
| 18 | `op_sub` | `0 - INT_MIN` (negation of `INT_MIN`) | wrap ⇒ `INT_MIN` | [x] |
| 19 | `op_mul` | `a * b` overflows (`INT_MAX, 2`; `INT_MIN, -1`; `65536, 65536`) | wrap (low 32 bits) | [x] |
| 20 | `helper_call` | `r + acc` overflows after an overflowing `OP` (`INT_MAX, INT_MAX`) | wrap; stdout still `helper.call=<r> helper.acc=<acc>` | [x] |
| 21 | `helper_call` / `helper_ptr` / `op_*` | `a` or `b` = `INT_MIN` / `INT_MAX` boundary values | no rejection — full 32-bit domain is valid input | [x] |
| 22 | `helper_call` (`OP=mul`, `REPEAT=7`) | `STEP_mul` accumulator overflow: `1*1*2*…*7` = `5040` (no overflow), but `REPEAT` unroll of `mul` is the only accumulator that can overflow — verified no divergence across all `REPEAT` | wrap | [x] |
| 23 | `G_OP` (exported *writable* data) | caller overwrites `G_OP` with another function pointer, then calls `helper_ptr` | `helper_ptr` uses `OP_FN(OP)` **directly**, *not* `G_OP` ⇒ result unchanged by the overwrite; a subsequent direct `G_OP(a,b)` call *does* use the new pointer | [x] |
| 24 | `G_OP_NAME` (exported writable pointer) | caller reads the pointee | NUL-terminated `"add"`/`"sub"`/`"mul"` (exactly 3 bytes + NUL) | [x] |
| 25 | out-of-range "enum"-like int across FFI | `use_generated` is the only function taking a value the C `switch`es on; every `int` bit pattern is accepted (`default:`), so there is no invalid-variant trap — asserted for a randomized sweep incl. `INT_MIN..INT_MAX` extremes | returns `INIT_FOR(OP)` outside `0..=6` | [x] |
| 26 | null pointers | **no** function in the public API takes a pointer parameter (`nm -D` + headers: all params are `int`), so there is no null-pointer rejection path to test | n/a — documented absence | [x] |
| 27 | zero / oversized lengths | **no** function takes a length or buffer parameter | n/a — documented absence | [x] |

All 27 rows have a passing differential test in `tests/error_paths.rs`
(rows 1-8 in `tests/binary_diff.rs`), run for every `OP` × `REPEAT`
configuration.

## Verification results

All 27 rows have a passing differential test in every one of the 24
`OP`×`REPEAT` builds plus the 18 Cargo-feature-resolution sets
(`scripts/run_all.sh`, `scripts/run_resolution.sh`).

Two rows initially failed and drove Rust-side fixes (see `CONFIGS.md` →
*Divergences found and fixed*):

* **Row 23** — writing to `G_OP` SIGSEGV'd against the Rust `.so`
  (`.data.rel.ro` vs C's `.data`); and `helper_ptr` wrongly observed the
  overwrite. Both fixed.
* **Row 24** — same section problem for `G_OP_NAME`. Fixed.

Rows 26 and 27 are *documented absences*: `nm -D` plus the prototypes in
`mdmacros.h` show every public function takes only `int` parameters, so the C
has no null-pointer, zero-length or oversized-length rejection path to mirror.
`tests/error_paths.rs::err26_err27_no_pointer_or_length_parameters` asserts that
absence mechanically against the header so the row cannot silently go stale.
