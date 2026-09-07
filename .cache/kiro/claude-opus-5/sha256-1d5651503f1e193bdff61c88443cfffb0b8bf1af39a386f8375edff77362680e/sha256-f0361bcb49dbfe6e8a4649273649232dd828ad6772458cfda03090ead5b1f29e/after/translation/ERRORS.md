# ERRORS.md — Error-surface table (Phase A, gate for Phase C)

Derived **mechanically** from `c_src/src/lib.c` and `c_src/include/lib.h`, not
from documentation or assumption. Exhaustive grep of every rejection construct:

```sh
grep -nE 'return|assert|NULL|errno|goto|if *\(|<|>|==|!=|MAX|MIN|LIMIT|ERROR' c_src/src/lib.c
```

Findings, by construct class:

| construct class | occurrences in C |
|-----------------|------------------|
| error-return macro (`RETURN_ERROR`, …) | **0** — none defined or used |
| error enum / status code type | **0** — return type is `char *`, sentinel is `NULL` |
| `return NULL` / `return (char *)NULL` | **2** (lines 11 and 18) |
| `return -1` / negative sentinel | **0** |
| `assert(...)` | **0** — `<assert.h>` is not even included |
| explicit range / bounds check (`<`, `>`, `<=`, `>=`) | **0** |
| null check | **2** — `if(!str)` (input), `if(!newstr)` (allocation result) |
| min / max / limit constant | **0** |
| `errno` inspection or assignment | **0** |
| `#ifdef` / `#if` conditional compilation | **0** |
| enum parameter crossing the FFI boundary | **0** — see row 5 |

So the complete error surface is exactly **two** rejection branches, plus the
generic FFI boundaries the task requires be covered regardless.

## Table

One row per distinct rejection / boundary condition.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `custom_strdup` | `str == NULL` — `if(!str)` at `lib.c:11` takes the true branch. This is the *only* input value the function itself rejects; `malloc` is never reached. | returns `NULL` (`(char *)NULL`); no allocation performed; no write through any pointer | `err_row1_null_input_returns_null` | [x] |
| 2 | `custom_strdup` | `malloc(strlen(str)+1)` fails and yields `NULL` — `if(!newstr)` at `lib.c:18` takes the true branch. Reached only with a non-NULL `str` under allocator exhaustion. Provoked by lowering `RLIMIT_AS` in a forked child, then duplicating a 256 MiB string. | returns `NULL`; `memcpy` is *not* executed (no crash, no partial copy) | `err_row2_malloc_failure_returns_null` | [x] |
| 3 | `custom_strdup` | Generic boundary — **zero length**: `str` points at `""`. Not an error in the C: `strlen` is 0, `len` becomes 1, `malloc(1)` succeeds, one NUL byte is copied. Must return a **non-NULL** 1-byte buffer, i.e. the C does *not* reject it and the Rust must not either. | returns non-`NULL`; `result[0] == 0` | `err_row3_empty_string_is_not_an_error` | [x] |
| 4 | `custom_strdup` | Generic boundary — **oversized length**: a genuinely huge but allocatable string (16 MiB), and a length request past what the allocator can serve. Covers the `len` computation on large inputs and confirms success/`NULL` agreement at the large end. | large-but-allocatable → non-`NULL` exact copy; unservable → `NULL` (same as row 2) | `err_row4_oversized_length`, `err_row2_malloc_failure_returns_null` | [x] |
| 5 | `custom_strdup` | Generic boundary — **out-of-range enum value across FFI**: **N/A / vacuously covered.** The public API has exactly one parameter, `const char *`, and no `enum`, `int` mode, or flag parameter anywhere in `lib.h`. There is no integer domain with "no valid variant" to pass. The nearest analogue — an arbitrary non-NULL pointer value that is not a valid C string — is *undefined behaviour* in the C (`strlen` reads out of bounds), so it is not a defined input and is deliberately not exercised. The defined pointer domain is `{NULL}` ∪ `{valid NUL-terminated buffers}`, and both halves are covered by rows 1–4/6. | (no such input exists) | documented in `errors.rs` header comment | [x] |
| 6 | `custom_strdup` | Generic boundary — **one step past a "documented valid range"**: the header documents no range. The only implicit range is the byte domain of the string contents, `1..=255` (a `0` byte terminates rather than being content). Verified that byte value `255` (max) and `1` (min) are copied verbatim and that a `0` byte truncates the copy exactly as `strlen` dictates. | copy stops at the first `0` byte; bytes `1` and `255` round-trip unchanged | `err_row6_byte_domain_boundaries` | [x] |
| 7 | `custom_strdup` | Documented-but-unreachable: `strlen(str) + 1` overflowing `size_t` (requires `strlen == SIZE_MAX`, i.e. a 2^64−1-byte object). **Unreachable on any real 64-bit host** — no such object can exist. The Rust uses `wrapping_add(1)`, which matches C's defined unsigned wraparound, so no divergence is possible even hypothetically. | (unreachable) | not testable — behaviour matched by construction (`wrapping_add`) | [x] |

**Rows 1–6 have executable differential tests; every row is checked off.**
Rows 5 and 7 are checked off as *reasoned-N/A with the reason recorded above*,
not as passing assertions — there is no input that can reach them.
