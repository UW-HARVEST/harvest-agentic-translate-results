# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Mechanically derived from `c_src/src/lib.c` (90 lines, one function). Every
statement in the C that rejects input, returns early, returns a sentinel, or
dereferences unchecked memory is listed. Greps used:

```
grep -n 'return\|NULL\|assert\|<\|>\|==' c_src/src/lib.c
```

Findings: **no** `assert`, **no** error enum, **no** error-code macro, **no**
explicit range check, **no** min/max constant, **no** length/size validation.
The only failure signalling mechanism is `return NULL` (5 sites) plus the
early `return strdup(orig)` and unchecked pointer dereferences (`strlen`,
`strstr` on caller pointers). The API takes no enums, so there is no
out-of-range-enum class for this library (recorded as row 12 for completeness).

All rows are covered by `tests/error_paths.rs` and pass against BOTH the
release and the debug Rust `.so`, for both feature combinations
(**16/16 tests pass**).

| #  | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|----|----------|-----------------------------------------|-------------------|------|-----|
| 1  | `searchAndReplace` | `strstr(orig, search) == NULL` (line 22-26): `search` does not occur in `orig` (incl. `search` longer than `orig`, `orig` empty with non-empty `search`) | returns a NEW `malloc`'d copy of `orig` (`strdup`), i.e. non-NULL pointer, contents byte-identical to `orig`; pointer != `orig` | `err01_no_match_returns_copy` | [x] |
| 2  | `searchAndReplace` | line 24 `strdup(orig)` fails (allocation of `strlen(orig)+1` bytes fails) on the no-match path | returns `NULL` (the value of `tmp`) | `err02_strdup_failure_returns_null` | [x] |
| 3  | `searchAndReplace` | line 34-37 `malloc(inx_start + 1)` fails on the "copy content before first match" path (requires first match at offset > 0) | returns `NULL`, no further work, prior state leaked | `err03_prefix_malloc_failure_returns_null` | [x] |
| 4  | `searchAndReplace` | line 45-48 `realloc(tmp, total += value_len)` fails while copying the replacement inside the loop | returns `NULL` (old `tmp` leaked — C does not free it) | `err04_value_realloc_failure_returns_null` | [x] |
| 5  | `searchAndReplace` | line 62-65 `realloc(tmp, total += gap)` fails while copying the content BETWEEN two matches (requires ≥2 matches with a gap > 0) | returns `NULL` (old `tmp` leaked) | `err05_gap_realloc_failure_returns_null` | [x] |
| 6  | `searchAndReplace` | line 80-83 `realloc(tmp, total += orig_len - from)` fails while copying the tail after the last match (requires `0 < from < orig_len`) | returns `NULL` (old `tmp` leaked) | `err06_tail_realloc_failure_returns_null` | [x] |
| 7  | `searchAndReplace` | `orig == NULL` → `strlen(NULL)` at line 11 (unchecked deref, UB) | process fault (`SIGSEGV`) — no NULL check exists | `err07_null_orig_faults, err07_09_all_null_faults` | [x] |
| 8  | `searchAndReplace` | `search == NULL` → `strlen(NULL)` at line 12 (unchecked deref, UB) | process fault (`SIGSEGV`) | `err08_null_search_faults` | [x] |
| 9  | `searchAndReplace` | `value == NULL` → `strlen(NULL)` at line 13, evaluated BEFORE the `strstr` early-out, so it faults even when `search` does not occur in `orig` (UB) | process fault (`SIGSEGV`) | `err09_null_value_faults` | [x] |
| 10 | `searchAndReplace` | `search == ""` (empty needle) with `value == ""`: `strstr` returns `orig` every iteration, `inx_start`/`from` never advance and `total_bytes_allocated` never grows → line 42 loop cannot terminate | never returns (spins forever; killed only by a signal). Both implementations must hang identically | `err10_empty_search_empty_value_never_returns` | [x] |
| 11 | `searchAndReplace` | `search == ""` (empty needle) with `value != ""`: same non-terminating loop, but `total_bytes_allocated += value_len` every iteration → memory grows without bound until `realloc` (line 45) fails | returns `NULL` on the first failing `realloc` (memory exhaustion) | `err11_empty_search_nonempty_value_exhausts_memory` | [x] |
| 12 | `searchAndReplace` | out-of-range enum / flag value passed across the FFI boundary | **not applicable** — the public API (`c_src/include/lib.h`) declares no enum, flag, mode, or integer parameter; the only parameters are three `const char *`. No such input exists to differ on. | `err12_no_enum_parameters_in_api` | [x] |

## Notes on how the rows are exercised (see `tests/error_paths.rs`)

* Rows 2–6 and 11 are allocation-failure paths. They are triggered
  deterministically by `fork()`ing a child, lowering `RLIMIT_AS` to the child's
  current VM size plus a small slack (8 MiB), and shaping the input so that
  exactly the targeted allocation is the one that exceeds the slack. C and Rust
  each run in their own freshly `fork()`ed child (identical memory state), and
  the two children's exit codes are compared.
  The targeted allocation is **128 MiB**, deliberately larger than glibc's
  `DEFAULT_MMAP_THRESHOLD_MAX` (32 MiB on 64-bit): smaller requests can be
  satisfied out of a thread arena's already-reserved 64 MiB of address space, in
  which case `RLIMIT_AS` would not stop them and the row would not actually be
  exercised (this was observed and fixed during Phase C).
* Rows 7–9 are UB/fault paths: each is run in a `fork()`ed child and the
  termination status (killed-by-signal + signal number, or normal exit) is
  compared between C and Rust.
* Row 10 is a non-termination path: each implementation runs in a `fork()`ed
  child with `alarm(N)`; both must be killed by `SIGALRM` (i.e. neither returns).
* Row 1 is a normal-path rejection and is also covered by many `CONFIGS.md`
  rows.

## Generic boundary cases beyond the table (also differential, also passing)

| case | test |
|------|------|
| every empty/non-empty combination of the three arguments (excluding the non-terminating empty `search`) | `errX_zero_length_inputs` |
| the same buffer aliased as `orig`/`search`/`value` (and `orig`==`value`) | `errX_aliased_pointers` |
| `search` exactly one byte longer than `orig` (one step past the length that can match), plus the equal-length boundary that CAN match, plus empty `orig` | `errX_one_past_valid_search_length` |
| result buffer must come from the C allocator (every returned pointer is `free()`d by the harness) | all tests, via `harness::call` |
