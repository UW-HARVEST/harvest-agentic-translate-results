# ERRORS.md — error / rejection surface table (Phase A)

Mechanically derived from `c_src/src/lib.c`. `grep -n -E 'return|assert|NULL|malloc|realloc|strdup|if *\(|while *\('`
gives every branch and every exit of the single public function; the table below
has one row per *distinct* way the C code fails, refuses, or otherwise leaves the
happy path.

Facts about the C source that shape this table:

* There are **no** `assert`s, **no** error enums, **no** error-return macros
  (`RETURN_ERROR` etc.), **no** `errno` use, and **no** explicit range / null /
  min-max constant checks. The only sentinel is `NULL`.
* The only inputs are three `const char *`. There are **no enum parameters**, so
  "out-of-range enum value across FFI" is not an applicable input class for this
  API (recorded as row 12 for completeness).
* Consequently every rejection is either (a) an allocator failure returning
  `NULL`, or (b) an unchecked precondition violation that the C code handles by
  crashing / not terminating. Rows 7-11 are those unchecked preconditions: the C
  is the ground truth, so the Rust must crash / hang the *same way*, and the
  tests assert exactly that (same terminating signal, same non-termination).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `searchAndReplace` | no match (`strstr(orig, search) == NULL`) **and** the `strdup(orig)` on line 24 fails (allocator exhausted) | returns `NULL` (line 25 returns `tmp`, which is `NULL`) |
| 2 | `searchAndReplace` | first match at `inx_start > 0` **and** the prefix `malloc(inx_start + 1)` on line 34 fails | returns `NULL` (line 36) |
| 3 | `searchAndReplace` | any loop iteration where the replacement-copy `realloc(tmp, total += value_len)` on line 45 fails | returns `NULL` (line 47); previously allocated `tmp` is leaked |
| 4 | `searchAndReplace` | two matches separated by a gap (`inx_start2 > from`) **and** the gap `realloc(tmp, total += gap)` on line 62 fails | returns `NULL` (line 64); `tmp` leaked |
| 5 | `searchAndReplace` | trailing text after the last match (`from < orig_len && from > 0`) **and** the tail `realloc(tmp, total += orig_len - from)` on line 80 fails | returns `NULL` (line 82); `tmp` leaked |
| 6 | `searchAndReplace` | allocator failure on a *later* allocation than the first one (e.g. prefix `malloc` succeeds, k-th `realloc` fails) — one row per k is covered by sweeping k | returns `NULL` at exactly the k-th allocation, for the same k in C and Rust |
| 7 | `searchAndReplace` | `orig == NULL` (unchecked; `strlen(orig)` on line 10 dereferences it) | fatal `SIGSEGV` before any allocation |
| 8 | `searchAndReplace` | `search == NULL` (unchecked; `strlen(search)` on line 11) | fatal `SIGSEGV` |
| 9 | `searchAndReplace` | `value == NULL` (unchecked; `strlen(value)` on line 12, evaluated even when `value` is never used because there is no match) | fatal `SIGSEGV` |
| 10 | `searchAndReplace` | `search == ""` and `value == ""`: `strstr` matches at 0 forever, `inx_start`/`from` never advance, `total_bytes_allocated` never grows → `while (p != NULL)` never exits | does not terminate (infinite loop, no memory growth); killed only by the caller |
| 11 | `searchAndReplace` | `search == ""` and `value != ""`: same non-terminating loop but `total_bytes_allocated += value_len` every iteration → `realloc` grows without bound until it fails | non-terminating; under an address-space limit the `realloc` on line 45 eventually fails and it returns `NULL` (row 3's branch, reached by exhaustion) |
| 12 | `searchAndReplace` | out-of-range enum value passed across the FFI boundary | **not applicable** — the API has no enum / integer parameters; the only parameter type is `const char *`. Recorded so the omission is deliberate and not an oversight. |

## Boundary cases that are *valid* input, not errors

Listed here so they are not mistaken for missing error rows; they are verified in
`CONFIGS.md` / Phase B:

* `orig == ""` with `search != ""` → no match → `strdup("")` → returns `""`.
* `value == ""` with a match → deletion; `total_bytes_allocated` stays `1` for a
  match at offset 0, so the result can be the empty string.
* `strlen(search) > strlen(orig)` → no match → `strdup(orig)`.
* Match covering the whole of `orig` → `from == orig_len`, so the
  `from < orig_len` tail branch is skipped.
* A match at offset 0 → the `inx_start > 0` prefix branch is skipped and `tmp`
  stays `NULL` into the first `realloc`, which therefore behaves as `malloc`.
* Overlapping occurrences (`search = "aa"`, `orig = "aaaa"`) → the rescan starts
  at `inx_start + search_len`, so overlaps are not matched again.
* Bytes `0x80..0xFF` in any argument → `strstr`/`strncpy` are byte-oriented; the
  `char`-signedness of the platform must not change the result.

## Test mapping

| row | test |
|---|---|
| 1 | `errors::row01_strdup_failure_returns_null` |
| 2 | `errors::row02_prefix_malloc_failure_returns_null` |
| 3 | `errors::row03_replacement_realloc_failure_returns_null` |
| 4 | `errors::row04_gap_realloc_failure_returns_null` |
| 5 | `errors::row05_tail_realloc_failure_returns_null` |
| 6 | `errors::row06_allocation_failure_sweep_matches` |
| 7 | `errors::row07_null_orig_same_signal` |
| 8 | `errors::row08_null_search_same_signal` |
| 9 | `errors::row09_null_value_same_signal` |
| 10 | `errors::row10_empty_search_empty_value_both_hang` |
| 11 | `errors::row11_empty_search_nonempty_value_both_exhaust_allocator` |
| 12 | `errors::row12_no_enum_parameters_documented` (compile-time/documentation row; asserts the header exposes no non-pointer parameter) |

## Status

**12/12 rows pass** (`cargo test --test errors`), for the C `.so` against both
Rust `.so` profiles, under both feature combinations.

How the rows are made reachable:

* Rows 1-6 use an `LD_PRELOAD` interposer (`tests/support/failalloc.c`, compiled
  by the test into `target/test-support/failalloc.so`) that fails the k-th
  `malloc`/`realloc`/`strdup` performed *inside* the call. Both `.so`s are driven
  through the identical mechanism, so "the `realloc` on line 62 fails" is a
  constructible input rather than a thought experiment. Row 6 sweeps
  `k = 1..count+2` over six shapes and requires C and Rust to fail at the same
  index, request the same sizes in the same order, and agree once `k` is past the
  last allocation.
* Rows 7-11 run in a child process (this test binary re-executed with `PROBE_*`
  env vars) so that a `SIGSEGV` or an infinite loop is an observable, comparable
  outcome: rows 7-9 assert the *same* signal (11) from both, row 10 asserts both
  are still spinning after 2 s.

### Finding: `malloc` vs `realloc(NULL, n)` in the optimised build

The allocation trace comparison initially reported a difference on the first
allocation of a match at offset 0 (where `tmp` is provably `NULL`):

```
C            -> r:3,r:5   (realloc(NULL, 3), realloc(p, 5))
rust-release -> m:3,r:5   (malloc(3),        realloc(p, 5))
rust-debug   -> r:3,r:5   (identical to C)
```

This is LLVM folding `realloc(NULL, n)` into `malloc(n)` in the release build —
an equivalence the C standard itself states (C17 7.22.3.5: `realloc` with a null
pointer behaves like `malloc`). The allocation *count*, the requested *sizes*,
the index at which an injected failure takes effect, and the returned bytes are
all identical, so no source change was made; instead the trace comparison folds
`malloc`/`realloc` into one class (`a`) while still comparing sizes and order
exactly, and keeps `strdup` (`d`) distinct because substituting it would change
what gets copied. The unoptimised Rust `.so` reproduces the C trace verbatim,
which is what confirms the translation itself is call-for-call faithful.
