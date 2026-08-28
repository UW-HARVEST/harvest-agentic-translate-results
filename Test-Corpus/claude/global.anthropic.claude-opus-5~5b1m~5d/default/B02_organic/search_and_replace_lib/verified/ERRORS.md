# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (90 lines) and `c_src/include/lib.h`:

```sh
grep -n "return"                      c_src/src/lib.c   # 6 return sites
grep -n "NULL"                        c_src/src/lib.c   # 11 NULL mentions
grep -nE "assert|MAX|MIN|errno|enum|#define|-1" c_src/src/lib.c c_src/include/lib.h  # -> NONE
```

Findings of that sweep:

* **6 `return` statements**, of which **4 are explicit `return NULL` error
  returns** (lines 36, 47, 64, 82) — each guarded by its own `if (tmp == NULL)`
  after a distinct `malloc`/`realloc` call site, so they are **4 distinct rows**.
* **1 early return** on "needle not present" (line 25) that returns
  `strdup(orig)` — that call can itself fail and return `NULL` (row 5).
* **1 success return** (line 89).
* **No `assert`**, no `errno` use, no error enum, no numeric range check, no
  min/max constant, no negative sentinel (`-1`), no length/size parameter.
  The API takes three `const char *` and returns `char *`; the **only** error
  channel is a `NULL` return, and the only "invalid input" the code can be given
  is a bad pointer or a needle that makes the loop non-terminating.
* **No enum parameters exist anywhere in the public header**, so the
  "out-of-range enum value across FFI" class is *vacuous* for this library; the
  equivalent "value with no valid variant" inputs are (a) NULL pointers and
  (b) the empty needle, both covered below (rows 6–10).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `searchAndReplace` | `malloc(inx_start + 1)` fails (line 34) — prefix buffer allocation; reached when `inx_start > 0` and the request exceeds `RLIMIT_AS` | `return NULL` (line 36); already-nothing allocated | `e1_malloc_prefix_fails_returns_null` | [x] |
| E2 | `searchAndReplace` | `realloc(tmp, total += value_len)` fails (line 45) — replacement-copy growth inside the `while` loop | `return NULL` (line 47); previous `tmp` leaked | `e2_realloc_value_fails_returns_null` | [x] |
| E3 | `searchAndReplace` | `realloc(tmp, total += gap)` fails (line 62) — inter-match gap copy (needs ≥2 matches with a gap) | `return NULL` (line 64); previous `tmp` leaked | `e3_realloc_gap_fails_returns_null` | [x] |
| E4 | `searchAndReplace` | `realloc(tmp, total += orig_len - from)` fails (line 80) — trailing-tail copy after the last match | `return NULL` (line 82); previous `tmp` leaked | `e4_realloc_tail_fails_returns_null` | [x] |
| E5 | `searchAndReplace` | `strstr(orig, search) == NULL` (no match) **and** `strdup(orig)` fails (line 24) | `return NULL` (returns `tmp`, which is `NULL`) | `e5_strdup_fails_returns_null` | [x] |
| E6 | `searchAndReplace` | `orig == NULL` → `strlen(NULL)` (line 11) dereferences address 0 | process dies with `SIGSEGV` (11); no return value | `e6_null_orig_segv` | [x] |
| E7 | `searchAndReplace` | `search == NULL` → `strlen(NULL)` (line 12) | process dies with `SIGSEGV` (11) | `e7_null_search_segv` | [x] |
| E8 | `searchAndReplace` | `value == NULL` → `strlen(NULL)` (line 13); happens **before** any match test, so it faults even when `search` is absent from `orig` | process dies with `SIGSEGV` (11) | `e8_null_value_segv` / `e8b_null_value_no_match_segv` | [x] |
| E9 | `searchAndReplace` | all three arguments `NULL` | process dies with `SIGSEGV` (11) | `e9_all_null_segv` | [x] |
| E10 | `searchAndReplace` | `search == ""` (empty needle) and `value == ""`: `strstr` returns `orig` forever, `inx_start2 == from` so nothing advances, `value_len == 0` so `total_bytes_allocated` never grows → `while (p != NULL)` **never terminates** and never allocates | infinite loop (no return, no memory growth); process must be killed | `e10_empty_search_empty_value_hangs` | [x] |
| E11 | `searchAndReplace` | `orig == ""` **and** `search == ""` — same non-termination, but this is the only input for which `from == 0`, i.e. the input the `&& from > 0` guard on line 78 exists for | infinite loop (no return) | `e11_empty_orig_empty_search_hangs` | [x] |
| E12 | `searchAndReplace` | `search == ""` (empty needle) and `value != ""`: same infinite loop, but each iteration grows `total_bytes_allocated` by `value_len` → the line-45 `realloc` eventually fails | `return NULL` (line 47) once memory is exhausted | `e12_empty_search_nonempty_value_oom_null` | [x] |
| E13 | `searchAndReplace` | oversized single request: `value` alone larger than the address-space limit, single match at index 0 (`realloc(NULL, 1 + value_len)`) | `return NULL` (line 47) | `e13_oversized_value_returns_null` | [x] |
| E14 | `searchAndReplace` | zero-length inputs that are **valid** and must NOT error: `orig == ""` with non-empty `search` → `strdup("")`; `value == ""` (pure deletion) | non-`NULL` `""` / deleted-text result, never `NULL` | `e14_zero_length_inputs_are_not_errors` | [x] |
| E15 | `searchAndReplace` | one step past the "valid range" of a needle: `strlen(search) == strlen(orig) + 1` (needle one byte longer than the haystack) and `strlen(search) == strlen(orig)` but differing in the last byte | `strstr` → `NULL` → `strdup(orig)` (a copy, not an error) | `e15_needle_one_past_haystack_len` | [x] |
| E16 | `searchAndReplace` | non-NUL-terminated-adjacent boundary: needle matching only the final byte of `orig` (`from == orig_len`, tail length 0) — the `from < orig_len` guard on line 78 is false | success, tail copy skipped, result NUL-terminated at `total-1` | `e16_match_at_last_byte_no_tail` | [x] |
| E17 | (public API) | "out-of-range enum value across the FFI boundary": pinned as **vacuous** — the header exposes no enum and no integer parameter, only three `const char *`. The test fails if the header ever grows one, forcing new rows here | n/a (mechanical guard) | `e17_no_enum_or_integer_parameters_in_public_api` | [x] |
| E18 | `searchAndReplace` | aliased arguments: the *same* pointer passed as `orig`+`search`+`value`, and as `orig`+`search` (nothing in the C forbids it; `realloc` never moves `orig`) | success, identity / whole-string replacement | `e18_aliased_arguments` | [x] |
| E19 | `searchAndReplace` | smallest possible non-empty inputs (1-byte `orig`/`search`/`value`, matching and non-matching, incl. byte `0xff`) | success | `e19_minimal_shapes` | [x] |
| E20 | (harness) | anti-vacuity: the two `.so`s must resolve to *different* code and the comparison must be content-sensitive | n/a (guard) | `e20_harness_is_not_vacuous` | [x] |

## How the allocation-failure rows (E1–E5, E12, E13) are triggered

`malloc`/`realloc`/`strdup` are only made to fail by capping the address space of
a **child process**: the test re-executes its own test binary under
`sh -c 'ulimit -v <KiB>; exec … --exact child_dispatch'` with
`SR_CHILD_LIB` (`c`|`rust`) and `SR_CHILD_SCENARIO` in the environment.
Each scenario allocates one `SR_BIG_MB`-sized buffer (200 MiB by default, which
succeeds under the default `SR_ULIMIT_MB=400` cap) and then forces the library to
request a second ~200 MiB block **at exactly one call site**, which fails. C and
Rust are exercised in **separate** children with identical limits so that the
leak one of them performs cannot perturb the other, and the two children's
outcomes (fatal signal, exit code, and the reported `NULL` / `len`+FNV-1a hash of
the returned string) must be identical.

Each allocation-failure row also runs a **positive control**
(`assert_positive_control`): the identical scenario with `SR_BIG_MB=8` and a
generous cap must *succeed* in both implementations with the same length and
hash. That proves the inputs are otherwise valid and that execution really
reaches the allocation site under test, i.e. the `NULL` observed under the tight
cap comes from the capped allocation and not from an earlier rejection.

The non-termination rows (E10, E11) are verified by spawning a child per library,
waiting past a 3 s timeout, asserting it is **still running** (i.e. did not
return), then killing it — both libraries must hang identically.

## Anti-vacuity evidence

`scripts/mutation_check.sh` injects 9 deliberate bugs into `src/lib.rs`
(overlapping restart, empty-needle handling, `strdup` of the wrong argument, a
removed allocation-failure check, off-by-one gap/tail copies, a truncating
`strncpy`, a last-match `strstr`, a skipped prefix copy), rebuilds the `.so` and
requires the suite to FAIL for every one of them. All 9 are detected;
`src/lib.rs` is restored afterwards (checked by md5).

The non-termination rows (E10, E11) are verified by spawning a child per library,
waiting past a timeout, asserting it is **still running** (i.e. did not return),
then killing it — both libraries must hang identically.
