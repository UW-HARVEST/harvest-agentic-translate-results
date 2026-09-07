# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping for every `return`
that is not the normal success return, every `NULL` check, every `assert`,
every range/bounds test and every named limit:

```
grep -n 'return\|NULL\|assert\|<=\|>=\|<\|>' c_src/src/lib.c
```

Findings: **no** `assert`, **no** error enum, **no** `RETURN_ERROR`-style
macro, **no** named min/max constant, **no** explicit input range check, and
**no** `errno` use anywhere in the library. The only failure mode the C code
models is *allocation failure*, reported by returning `NULL`. Every remaining
row below is an implicit boundary of the API (undefined-behaviour or
non-termination) that the Rust translation must reproduce identically.

## Explicit rejections in the C source

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `searchAndReplace` (lib.c:22-26) | `strstr(orig, search) == NULL` — `search` does not occur in `orig` (includes `search` longer than `orig`, and `orig == ""` with non-empty `search`) | early return of `strdup(orig)`: a fresh heap copy of `orig`, **not** an error, and no replacement performed | `differential.rs::err_01_no_match_returns_copy` |
| 2 | `searchAndReplace` (lib.c:24) | no match **and** `strdup` cannot allocate `strlen(orig)+1` bytes | returns `NULL` | `robustness.rs` probe `oom-strdup` |
| 3 | `searchAndReplace` (lib.c:34-37) | first match not at offset 0 (`inx_start > 0`) **and** `malloc(inx_start + 1)` fails | returns `NULL` (nothing allocated yet) | `robustness.rs` probe `oom-prefix-malloc` |
| 4 | `searchAndReplace` (lib.c:45-48) | `realloc(tmp, total + value_len)` fails while appending the replacement `value` | returns `NULL` (old `tmp` leaked) | `robustness.rs` probe `oom-value-realloc` |
| 5 | `searchAndReplace` (lib.c:62-65) | `realloc(tmp, total + gap)` fails while copying the gap between two matches (`inx_start2 > from`) | returns `NULL` (old `tmp` leaked) | `robustness.rs` probe `oom-gap-realloc` |
| 6 | `searchAndReplace` (lib.c:80-83) | `realloc(tmp, total + orig_len - from)` fails while copying the tail after the last match (`from < orig_len && from > 0`) | returns `NULL` (old `tmp` leaked) | `robustness.rs` probe `oom-tail-realloc` |

## Implicit boundaries (no check in the C — must still match)

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 7 | `searchAndReplace` (lib.c:11) | `orig == NULL` — `strlen(NULL)` is dereferenced with no null check | fatal `SIGSEGV` | `robustness.rs` probe `null-orig` |
| 8 | `searchAndReplace` (lib.c:12) | `search == NULL` | fatal `SIGSEGV` | `robustness.rs` probe `null-search` |
| 9 | `searchAndReplace` (lib.c:13) | `value == NULL` (with non-`NULL` `orig`/`search`) | fatal `SIGSEGV` | `robustness.rs` probe `null-value` |
| 10 | `searchAndReplace` (lib.c:42-75) | `search == ""` (empty needle, any `orig`). `strstr(x, "")` returns `x`, so `p` is never `NULL`, `inx_start2 == inx_start`, `from == inx_start` and the `while` loop never terminates | **non-termination** (hangs; grows the buffer by `value_len` per iteration, so it also OOMs whenever `value != ""`) | `robustness.rs` probe `empty-search-hang` (both sides must still be running after the timeout) |
| 11 | `searchAndReplace` | `orig == "" && search == ""` | non-termination with *no* buffer growth (`realloc(tmp, 1)` every iteration) | covered by row 10 probe (uses empty `value` so the spin is allocation-free) |

## Not applicable

* **Out-of-range enum values across FFI** — the API has no enum, no flag and
  no integer parameter at all (three `const char *` in, one `char *` out), so
  there is no invalid-discriminant input class to test. Confirmed by the
  single-line header in `c_src/include/lib.h`.
* **Zero / oversized length arguments** — the API takes no length parameter;
  lengths are derived internally with `strlen`. Zero-length is covered as
  `orig == ""` (rows 1, 11) and `value == ""` (a valid deletion configuration,
  see `CONFIGS.md`). "Oversized" is covered by the allocation-failure rows
  2-6.
* `tmp[total_bytes_allocated - 1] = '\0'` (lib.c:87) cannot be reached with
  `tmp == NULL`: it is only reached when the first `strstr` matched, and the
  loop body then always `realloc`s (returning early on failure).

## Status

- [x] row 1
- [x] row 2
- [x] row 3
- [x] row 4
- [x] row 5
- [x] row 6
- [x] row 7
- [x] row 8
- [x] row 9
- [x] row 10
- [x] row 11
