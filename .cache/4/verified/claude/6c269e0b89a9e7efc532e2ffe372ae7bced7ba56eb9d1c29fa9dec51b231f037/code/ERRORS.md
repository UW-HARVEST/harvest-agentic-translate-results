# ERRORS.md — Error-surface table (Phase A → gates Phase C)

Mechanically derived from `c_src/src/lib.c`. Every rejection/error construct in
the C source was grepped for:

```sh
grep -nE "RETURN_ERROR|return -1|return NULL|abort|assert|exit\(|errno" \
     c_src/src/lib.c c_src/include/lib.h
# c_src/src/lib.c:13:        abort();
```

There is exactly **one** rejection construct in the whole library — the
`abort()` on line 13 — but it is reached from a **two-term short-circuit `||`**,
so it has two logically distinct triggers (plus their boundary values):

```c
if (bin_len >= (18446744073709551615UL) / 2 || hex_maxlen <= bin_len * 2U) {
    abort();
}
```

* `18446744073709551615UL` is `SIZE_MAX` on LP64; `SIZE_MAX / 2` =
  `9223372036854775807` = `0x7FFF_FFFF_FFFF_FFFF`.
* `bin_len * 2U`: `2U` (`unsigned int`) converts to `size_t`, so this is a
  wrapping `size_t` multiply.
* The library has **no** error return code, no `NULL` return, no `errno` use and
  no `assert`: the only observable failure mode is process death by `SIGABRT`.
  `bin2hex` therefore either aborts or returns its `hex` argument unchanged.

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| E1 | `bin2hex` | term 1 at its exact boundary: `bin_len == SIZE_MAX / 2 == 0x7FFF_FFFF_FFFF_FFFF` (with `hex_maxlen = SIZE_MAX`, i.e. term 2 false) | `abort()` → `SIGABRT` | [x] |
| E2 | `bin2hex` | term 1, far past boundary: `bin_len == SIZE_MAX` (`hex_maxlen = SIZE_MAX`) | `abort()` → `SIGABRT` | [x] |
| E3 | `bin2hex` | term 1, other over-range values: `bin_len ∈ {SIZE_MAX/2 + 1, 0x8000_0000_0000_0000, SIZE_MAX - 1}` | `abort()` → `SIGABRT` | [x] |
| E4 | `bin2hex` | term 2 at its exact boundary: `hex_maxlen == bin_len * 2` (e.g. `bin_len = 4`, `hex_maxlen = 8`) — buffer has room for the digits but **not** for the NUL | `abort()` → `SIGABRT` | [x] |
| E5 | `bin2hex` | term 2, strictly smaller: `hex_maxlen < bin_len * 2` (e.g. `bin_len = 4`, `hex_maxlen ∈ {0,1,7}`) | `abort()` → `SIGABRT` | [x] |
| E6 | `bin2hex` | term 2 with the degenerate empty input: `bin_len == 0 && hex_maxlen == 0` (`0 <= 0` is true) | `abort()` → `SIGABRT` | [x] |
| E7 | `bin2hex` | term 2 reached only because term 1 is false — short-circuit ordering check: `bin_len = SIZE_MAX/2 - 1` (term 1 false), `hex_maxlen = 0` (term 2 true) | `abort()` → `SIGABRT` (no huge loop, no read of `bin`) | [x] |
| E8 | `bin2hex` | term 1 false **and** term 2 false with an over-large `bin_len`: `bin_len = SIZE_MAX/2 - 1`, `hex_maxlen = SIZE_MAX` — C does **not** reject this; it runs the loop and walks off the end of both buffers | **no** `abort()`; both implementations die with `SIGSEGV` (C UB faithfully reproduced) | [x] |

## Generic FFI boundaries also covered by Phase C tests

| # | condition | expected C result | [x] |
|---|-----------|-------------------|-----|
| G1 | `hex == NULL`, `bin_len == 0`, `hex_maxlen == 1` — validation passes, then `hex[0] = 0` dereferences NULL | `SIGSEGV` in both | [x] |
| G2 | `bin == NULL`, `bin_len == 0`, `hex_maxlen >= 1` — **valid**: `bin` is never dereferenced | returns `hex`, writes `hex[0] = '\0'` | [x] |
| G3 | `bin == NULL`, `bin_len > 0` — first loop iteration dereferences NULL | `SIGSEGV` in both | [x] |
| G4 | `hex == NULL` **and** `bin == NULL`, `bin_len == 0`, `hex_maxlen == 0` | validation fires first → `SIGABRT` in both (not `SIGSEGV`) | [x] |
| G5 | `hex_maxlen == SIZE_MAX` (oversized length) with a small `bin_len` | accepted, normal conversion | [x] |
| G6 | `hex_maxlen == bin_len * 2 + 1` (exact minimum accepted length), incl. `bin_len = 0` | accepted, writes exactly `2*bin_len + 1` bytes | [x] |
| G7 | out-of-range enum value across FFI | **N/A** — the public API (`lib.h`) declares no `enum`, no flags and no
mode parameter; the only parameters are two `size_t` lengths and two pointers, all of whose ranges are covered by E1–E8/G1–G6. | [x] |

## Notes on how the error paths are tested

`abort()`/`SIGSEGV` kill the process, so `tests/differential_errors.rs` runs each
call in a `fork()`ed child (`RLIMIT_CORE` set to 0 to suppress core dumps) and
compares the **exact** `waitpid` status of the C child and the Rust child:
`WIFSIGNALED` + `WTERMSIG` (`SIGABRT` = 6, `SIGSEGV` = 11) or `WIFEXITED` +
`WEXITSTATUS`. Asserting the same signal number — not merely "both died" — is
what makes these differential tests.
