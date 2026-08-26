# ERRORS.md — error-surface table (Phase C gate)

Derived **mechanically** from `c_src/src/slicing.c`. The whole library is one
function; grep for every rejection construct:

```
$ grep -n 'return\|assert\|NULL\|Error' c_src/src/slicing.c
46:            printf("Error: start is off the end of the string!\n");
47:            return 1;
56:            printf("Error: stop is off the end of the string!\n");
57:            return 1;
60:            printf("Error: stop must come after start!\n");
61:            return 1;
69:    return 0;
```

* error-return sites: **3** (`return 1`, each preceded by exactly one `printf`)
* `assert` / `abort` / `exit`: **none**
* `return NULL` / pointer-returning functions: **none** (`slice` returns `int`)
* error enums / `errno` writes: **none**
* explicit range checks: `start > len`, `stop > len`, `stop <= start`
* null checks: `if (start_ptr)`, `if (stop_ptr)` — these are *not* rejections;
  a null pointer selects a **default** (`start = 0`, `stop = len`)
* min/max constants: none in the source. The implicit bounds are `0 .. strlen`
  for both indices, plus the `int`/`size_t` conversion boundary at `INT_MIN` /
  `INT_MAX` and `(size_t)` sign-extension of negatives.
* `mystr` is dereferenced by `strlen(mystr)` with **no** null check → passing
  `NULL` is UB (observed: `SIGSEGV`). Rust must fault identically.

Exact rejection messages (must be byte-identical on stdout):

| id | message written to stdout |
|----|---------------------------|
| E1 | `Error: start is off the end of the string!\n` |
| E2 | `Error: stop is off the end of the string!\n` |
| E3 | `Error: stop must come after start!\n` |

Semantics that make the table non-obvious, and that each row pins down:

1. `start > len` and `stop > len` compare `int` against `size_t`. Usual
   arithmetic conversions convert the `int` to `size_t`, so **negative indices
   become huge unsigned values** and are rejected by the "off the end" check —
   they are *not* treated as Python-style negative indices.
2. `stop > len` is checked **before** `stop <= start`, so a negative `stop`
   produces E2, never E3.
3. `start` is validated before `*stop_ptr` is even read, so E1 wins over E2/E3.
4. `stop <= start` uses `<=`: `stop == start` (an empty slice) is an **error**,
   even though `start_ptr == NULL, stop_ptr == NULL` on an empty string is fine.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `slice` | `start_ptr != NULL`, `*start_ptr == len + 1` (one past the documented range) | stdout `E1`, returns `1` | `err-01` | [x] |
| 2 | `slice` | `start_ptr != NULL`, `*start_ptr == -1` (negative → `(size_t)0xFFFF…FFFF > len`) | stdout `E1`, returns `1` | `err-02` | [x] |
| 3 | `slice` | `start_ptr != NULL`, `*start_ptr == INT_MIN` | stdout `E1`, returns `1` | `err-03` | [x] |
| 4 | `slice` | `start_ptr != NULL`, `*start_ptr == INT_MAX` (with `len < INT_MAX`) | stdout `E1`, returns `1` | `err-04` | [x] |
| 5 | `slice` | `start_ptr != NULL`, `*start_ptr` an arbitrary value in `(len, INT_MAX]` (randomised) | stdout `E1`, returns `1` | `err-05` | [x] |
| 6 | `slice` | `start_ptr != NULL`, `*start_ptr` an arbitrary value in `[INT_MIN, -1]` (randomised) | stdout `E1`, returns `1` | `err-06` | [x] |
| 7 | `slice` | `start_ptr == NULL`, `stop_ptr != NULL`, `*stop_ptr == len + 1` | stdout `E2`, returns `1` | `err-07` | [x] |
| 8 | `slice` | both non-NULL, `*start_ptr` valid, `*stop_ptr == len + 1` | stdout `E2`, returns `1` | `err-08` | [x] |
| 9 | `slice` | `stop_ptr != NULL`, `*stop_ptr == -1` → E2 wins over E3 (`stop > len` checked first) | stdout `E2` (**not** `E3`), returns `1` | `err-09` | [x] |
| 10 | `slice` | `stop_ptr != NULL`, `*stop_ptr == INT_MIN` | stdout `E2`, returns `1` | `err-10` | [x] |
| 11 | `slice` | `stop_ptr != NULL`, `*stop_ptr == INT_MAX` (with `len < INT_MAX`) | stdout `E2`, returns `1` | `err-11` | [x] |
| 12 | `slice` | `stop_ptr != NULL`, `*stop_ptr` random in `(len, INT_MAX]` / `[INT_MIN,-1]` | stdout `E2`, returns `1` | `err-12` | [x] |
| 13 | `slice` | both non-NULL, `*stop_ptr == *start_ptr`, both in `[0, len]` (boundary of `<=`) | stdout `E3`, returns `1` | `err-13` | [x] |
| 14 | `slice` | both non-NULL, `0 <= *stop_ptr < *start_ptr <= len` (randomised) | stdout `E3`, returns `1` | `err-14` | [x] |
| 15 | `slice` | `start_ptr == NULL` (default `start = 0`), `*stop_ptr == 0` | stdout `E3`, returns `1` | `err-15` | [x] |
| 16 | `slice` | both non-NULL and **aliased** (`start_ptr == stop_ptr`) → `stop == start` | stdout `E3`, returns `1` | `err-16` | [x] |
| 17 | `slice` | `*start_ptr > len` **and** `*stop_ptr > len` → precedence: start checked first | stdout `E1` only (not `E2`), returns `1` | `err-17` | [x] |
| 18 | `slice` | `*start_ptr > len` and `stop_ptr` a **wild/unreadable** pointer → must return before dereferencing it | stdout `E1`, returns `1`, no fault | `err-18` | [x] |
| 19 | `slice` | `len == 0` (empty string), `start_ptr != NULL`, `*start_ptr == 1` | stdout `E1`, returns `1` | `err-19` | [x] |
| 20 | `slice` | `len == 0`, `start_ptr == NULL`, `*stop_ptr == 0` | stdout `E3`, returns `1` | `err-20` | [x] |
| 21 | `slice` | `len == 0`, `start_ptr != NULL` `*start_ptr == 0`, `*stop_ptr == 0` | stdout `E3`, returns `1` | `err-21` | [x] |
| 22 | `slice` | zero-length input with **both** pointers NULL — documents that there is *no* minimum-length rejection | prints `"\n"`, returns `0` | `err-22` | [x] |
| 23 | `slice` | "out-of-range enum value across FFI": the API has **no enum**; the analogous input is an `int` index with no valid meaning. Sweep every `start`/`stop` choice from {NULL} ∪ `[-3, len+3]` for `len` in `0..=6` (which covers all four pointer combos) and require identical `(retval, stdout)` | identical `(0/1, bytes)` for all Σ(len+8)² = 875 cases | `err-23` | [x] |
| 24 | `slice` | `mystr == NULL` → unchecked `strlen(NULL)`; UB, observed `SIGSEGV` | child killed by the same signal in C and Rust | `err-24` | [x] |
| 25 | `slice` | `mystr` non-NUL-terminated / unreadable buffer | UB — **not tested** (no deterministic observable behaviour; excluded by design, see below) | — | n/a |
| 26 | `slice` | the output channel itself fails: `slice` called with **fd 1 closed** so every `printf`/`puts` fails with `EBADF`. The C code never inspects `printf`'s result, so it must still return its usual sentinel | success path returns `0`, all three rejection paths return `1`; `errno == 9` (`EBADF`) in both | `err-26` | [x] |
| 27 | `slice` | rejections at `strlen(mystr) > INT_MAX` (2 GiB string): `start = -1`/`INT_MIN` → `E1`; `stop = -1`/`INT_MIN` → `E2`; `stop == start`, `stop < start` → `E3`; and `start = INT_MAX` is *valid* at this length (contrast rows 4/11) | as listed, identical in C and Rust | `tests/huge_string.rs`, row `huge-01` | [x] |

Row 25 is the only row without a test: an unterminated buffer makes `strlen`
read past the allocation, whose behaviour is not deterministic in either
language (it depends on heap contents), so there is no well-defined C result to
compare against. Every other row has a passing differential test.

## Test mapping

| rows | test binary |
|------|-------------|
| 1–24, 26 | `tests/error_paths.rs` (`cargo test --test error_paths`) |
| 27 | `tests/huge_string.rs` (`cargo test --test huge_string`) |

Row 24 (`mystr == NULL`) is run in a forked child and compares the raw
`waitpid` status; both children die of signal 11 (`SIGSEGV`).

## Status

- [x] 26/26 testable rows have a differential test that asserts C and Rust
      return the *same* sentinel (`0`/`1`) **and** the same stdout bytes
      (2 188 + 20 error-path cases). Each row additionally pins the *documented*
      C behaviour via `cmp_expect`, so a row cannot pass by "both failed
      somehow".
- [x] `./mutation_check.sh` confirms the suite detects wrong sentinels
      (`return 1` → `return 2`), swapped/relaxed range checks and single-byte
      message changes.
