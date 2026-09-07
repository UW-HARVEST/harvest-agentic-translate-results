# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every rejection path in the C
source, one row each.

Grep audit of the C source for rejection constructs:

```sh
grep -n 'abort\|assert\|return -1\|return NULL\|RETURN_ERROR\|errno' c_src/src/lib.c
# 13:        abort();
```

The whole C function is:

```c
char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin,
                    size_t bin_len) {
    ...
    if (bin_len >= (18446744073709551615UL) / 2 || hex_maxlen <= bin_len * 2U) {
        abort();
    }
    ...
    return hex;                     /* the only non-error return */
}
```

So there is exactly ONE rejection statement (`abort()`), reached from a
two-term short-circuiting `||`. Each term is a distinct trigger, and the
`<=` / `>=` boundaries give the distinct one-step-past-the-range rows.
`bin2hex` returns `char *` but never returns `NULL` and never sets `errno`:
the only observable rejection is death by `SIGABRT` (signal 6). All rows are
therefore asserted by running the call in a forked child and comparing the
child's wait-status (signalled-by-6 vs. exited-0) between C and Rust.

`hex` / `bin` are NOT null-checked by the C code, and no `assert` is present.

| #  | function  | trigger (exact invalid input/condition)                                                                        | expected C result |
|----|-----------|----------------------------------------------------------------------------------------------------------------|-------------------|
| 1  | `bin2hex` | `bin_len >= SIZE_MAX/2`, i.e. `bin_len == 0x7FFFFFFFFFFFFFFF` (== `SIZE_MAX/2`, first term true, exact boundary) | `abort()` — SIGABRT |
| 2  | `bin2hex` | `bin_len == SIZE_MAX/2 + 1 == 0x8000000000000000` (first term true, one past boundary)                          | `abort()` — SIGABRT |
| 3  | `bin2hex` | `bin_len == SIZE_MAX == 0xFFFFFFFFFFFFFFFF` (first term true, maximal)                                          | `abort()` — SIGABRT |
| 4  | `bin2hex` | `bin_len == SIZE_MAX/2 - 1 == 0x7FFFFFFFFFFFFFFE` (first term FALSE — largest `bin_len` that passes term 1 — combined with a `hex_maxlen` that fails term 2) | `abort()` — SIGABRT (via term 2) |
| 5  | `bin2hex` | `hex_maxlen == bin_len * 2` exactly (second term true via `<=`, no room for the NUL terminator), `bin_len > 0`   | `abort()` — SIGABRT |
| 6  | `bin2hex` | `hex_maxlen < bin_len * 2` (second term true, e.g. `hex_maxlen = 0`, `bin_len = 4`)                             | `abort()` — SIGABRT |
| 7  | `bin2hex` | `bin_len == 0` and `hex_maxlen == 0` (second term: `0 <= 0` is TRUE — the empty input still aborts when `hex_maxlen` is 0) | `abort()` — SIGABRT |
| 8  | `bin2hex` | `bin_len == 0`, `hex_maxlen == 1` (one step INTO the valid range; must NOT abort, writes `hex[0] = 0`)          | returns `hex`, no abort |
| 9  | `bin2hex` | `hex_maxlen == bin_len * 2 + 1` exactly, `bin_len > 0` (minimum accepted `hex_maxlen`; must NOT abort)          | returns `hex`, no abort |
| 10 | `bin2hex` | `bin_len == SIZE_MAX/2 - 1` with `hex_maxlen` large enough to pass term 2 — term-2 product `bin_len*2` must NOT overflow, so the check passes and the loop is entered (then segfaults on the unmapped write). Establishes that neither term rejects it. | no abort from the checks (SIGSEGV from the huge write) |
| 11 | `bin2hex` | `hex == NULL` with a valid-looking `hex_maxlen`/`bin_len` (no null check in C)                                  | no abort; SIGSEGV on the store |
| 12 | `bin2hex` | `bin == NULL`, `bin_len > 0` (no null check in C)                                                              | no abort; SIGSEGV on the load |
| 13 | `bin2hex` | `bin == NULL`, `bin_len == 0`, `hex_maxlen >= 1` (loop body never runs, so the null `bin` is never dereferenced) | returns `hex`, no abort, `hex[0] = 0` |
| 14 | `bin2hex` | out-of-range "enum"-style ints across FFI: N/A — the C signature has no enum/flag parameter (`char*`, `size_t`, `const uint8_t*`, `size_t`); all `size_t` bit patterns are covered by rows 1-10 and all `uint8_t` byte values 0..=255 by `CONFIGS.md` row 4 | n/a (documented, no test) |

## Divergence found and fixed

Rows **11** and **12** (null `hex` / null `bin`) initially FAILED in the
`debug` profile: rustc injects a `debug_assertions`-gated
"null pointer dereference occurred" language-UB precondition check in front of
every raw-pointer deref, which kills the process with `SIGABRT` (6), whereas
the C — which performs no null check at all — dies from `SIGSEGV` (11).

```
[err11] status divergence: C=Signalled(11) Rust=Signalled(6)
[err12] status divergence: C=Signalled(11) Rust=Signalled(6)
```

Fixed in `src/lib.rs` by routing the byte load/store through libc `memcpy`
(`load_u8` / `store_u8`) and using `pointer::wrapping_add` instead of
`pointer::add` for the address arithmetic. Neither carries a rustc-side
precondition check, so the fault behaviour now matches C in the `release`
profile, the `debug` profile, and with
`RUSTFLAGS="-C debug-assertions=on -C overflow-checks=on"`.
