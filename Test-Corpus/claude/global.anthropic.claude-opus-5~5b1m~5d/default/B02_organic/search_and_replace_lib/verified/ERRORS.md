# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c` (90 lines, one function). Every
`return` other than the final `return tmp;` is enumerated below, plus every
implicit rejection (dereference of a caller pointer, unbounded loop).

Grep evidence:

```
$ grep -n 'return\|assert\|NULL\|== 0\|> 0\|< ' c_src/src/lib.c
22:    p = strstr(orig, search);
23:    if (p == NULL) {
24:        tmp = strdup(orig);
25:        return tmp;
32:    if (inx_start > 0) {
35:        if (tmp == NULL) {
36:            return NULL;
46:        if (tmp == NULL) {
47:            return NULL;
55:        if (p != NULL) {
59:            if (inx_start2 > from) {
63:            if (tmp == NULL) {
64:                return NULL;
78:    if ((from < orig_len) && from > 0) {
81:        if (tmp == NULL) {
82:            return NULL;
89:    return tmp;
```

There are **no** `assert`s, **no** error enums, **no** `errno` writes, **no**
range checks and **no** min/max constants in the C source. The only failure
signal the API has is the `NULL` return value; every non-`NULL` return is a
`malloc`'d NUL-terminated C string owned by the caller.

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `searchAndReplace` | `malloc` at **lib.c:34** fails (leading-context buffer, `inx_start + 1` bytes) — reached when `search` first matches at index > 0 and `inx_start + 1` exceeds the address-space limit | returns `NULL` (leaks nothing yet) | `err_e1_malloc_leading_fails` | ✅ |
| E2 | `searchAndReplace` | `realloc` at **lib.c:45** fails (replacement-value append, `+= value_len`) — reached when `inx_start == 0` (so no leading `malloc`) and `value_len + 1` exceeds the limit | returns `NULL` (leaks `tmp`) | `err_e2_realloc_value_fails` | ✅ |
| E3 | `searchAndReplace` | `realloc` at **lib.c:62** fails (between-matches gap append, `+= gap`) — reached with ≥ 2 matches where the second is far enough away that `gap` exceeds the limit, while the earlier small allocations succeed | returns `NULL` (leaks `tmp`) | `err_e3_realloc_gap_fails` | ✅ |
| E4 | `searchAndReplace` | `realloc` at **lib.c:80** fails (trailing-context append, `+= orig_len - from`) — reached with exactly one match followed by a tail larger than the limit | returns `NULL` (leaks `tmp`) | `err_e4_realloc_tail_fails` | ✅ |
| E5 | `searchAndReplace` | `strdup` at **lib.c:24** fails (no-match fast path, `orig_len + 1` bytes) — reached when `strstr(orig, search) == NULL` and `orig` is larger than the limit | returns `NULL` | `err_e5_strdup_nomatch_fails` | ✅ |
| E6 | `searchAndReplace` | `orig == NULL` — dereferenced unconditionally by `strlen(orig)` at **lib.c:11** | no rejection: undefined behaviour, `SIGSEGV` in practice | `err_e6_null_orig_segv` (fork + compare signals) | ✅ |
| E7 | `searchAndReplace` | `search == NULL` — dereferenced by `strlen(search)` at **lib.c:12** | no rejection: undefined behaviour, `SIGSEGV` in practice | `err_e7_null_search_segv` | ✅ |
| E8 | `searchAndReplace` | `value == NULL` — dereferenced by `strlen(value)` at **lib.c:13** | no rejection: undefined behaviour, `SIGSEGV` in practice | `err_e8_null_value_segv` | ✅ |
| E9 | `searchAndReplace` | `search == ""` (zero-length needle). `strstr(h, "")` returns `h`, so `p` is never `NULL`; `inx_start2 == inx_start` is never `> from`, so `inx_start` never advances and the `while (p != NULL)` loop at **lib.c:42** never terminates. With `value_len > 0` it `realloc`s unboundedly until the allocator fails and it returns `NULL`; with `value_len == 0` it spins forever with no allocation. | **non-termination** (hang), or eventual `NULL` after exhausting memory | `err_e9a_empty_search_exhausts_memory`, `err_e9b_empty_search_and_empty_value_hangs` (bounded-timeout forks; assert C and Rust behave identically, incl. hanging identically) | ✅ |
| E10 | `searchAndReplace` | all three pointers `NULL` simultaneously | undefined behaviour, `SIGSEGV` (first deref is `orig`) | `err_e10_all_null_segv` | ✅ |

### Notes on the generic boundaries required by Phase C

* **Null pointers** — E6/E7/E8/E10. The C performs *no* null checks, so the
  correct translation must also crash rather than return an error. The tests
  `fork()` and compare the child's termination signal for C vs Rust so a
  divergence (e.g. a Rust `NULL`-check that "helpfully" returns `NULL`, or a
  Rust panic/abort with `SIGABRT` instead of `SIGSEGV`) is caught.
* **Zero lengths** — `orig == ""`, `value == ""` are *valid* inputs, not errors;
  they live in `CONFIGS.md` (rows C1, C2, C10, C11). `search == ""` is the
  degenerate non-terminating case E9.
* **Oversized lengths** — E1–E5 (allocation-size failures). Sizes in this API
  are all derived from `strlen`, so an "oversized length" can only be produced
  by an oversized string; the tests constrain the address space with
  `setrlimit(RLIMIT_AS, …)` in a forked child *after* the inputs are built.
* **Full scalar input domain** — `err_boundary_full_byte_domain` sweeps every
  byte value `0x01..=0xFF` as a one-byte needle (present once / at both ends /
  absent / whole haystack), and
  `err_boundary_empty_and_one_byte_combinations` runs the full cross-product of
  `{"", "a", "X", "aXa"}` for all three arguments.
* **One step past a valid range / out-of-range enum values** — the API takes
  **no** integers, **no** enums, **no** flags and **no** lengths: its whole
  signature is three `const char *`. There is therefore no enum-domain
  boundary to probe; the equivalent "value with no valid variant" for a C
  string API is a non-NUL-terminated / NULL pointer, covered by E6–E8, and the
  degenerate empty needle, covered by E9.

## Result

All 10 rows have a passing differential test in `tests/error_paths.rs`
(13 `#[test]`s in total, including the two generic boundary sweeps). Every one
passes in all six build configurations — see `all_configs.log`
(`13 passed; 0 failed`).

The mutation self-check (`mutation_selfcheck.sh`, see `CONFIGS.md`) confirms
these tests are load-bearing: injecting a NULL-argument check that the C does
not have is caught by exactly `err_e6/e7/e8/e10`, breaking `strstr`'s
empty-needle contract is caught by exactly `err_e9a/e9b`, and changing the
allocation-growth expression is caught by `err_e2_realloc_value_fails` and
`err_e9a_empty_search_exhausts_memory`.
