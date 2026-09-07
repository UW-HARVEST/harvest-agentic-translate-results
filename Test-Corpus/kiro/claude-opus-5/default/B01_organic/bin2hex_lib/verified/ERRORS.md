# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. The complete set of rejection
constructs in the C source is:

```
$ grep -n 'return\|abort\|assert\|NULL\|if (\|switch\|enum' c_src/src/lib.c
12:    if (bin_len >= (18446744073709551615UL) / 2 || hex_maxlen <= bin_len * 2U) {
13:        abort();
26:    return hex;
```

So there is exactly **one** rejection statement (`abort()` at line 13), reached
by **two distinct short-circuit conditions**, plus the generic FFI boundary
conditions every C API has (null pointers, zero/oversized lengths, one step
past a documented range). There are no `assert`s, no error enums, no
`return -1` / `return NULL` paths, and no `#ifdef` branches.

`LIMIT` below denotes `18446744073709551615UL / 2` = `9223372036854775807`
(= `0x7FFF_FFFF_FFFF_FFFF`).

`abort()` terminates the process with `SIGABRT` (signal 6). The differential
tests fork a child per row and compare `WIFSIGNALED`/`WTERMSIG` (or
`WEXITSTATUS`) between the C `.so` and the Rust `.so`, so "same rejection"
means the same termination signal, not merely "both died".

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| E1 | `bin2hex` | `bin_len == LIMIT` (first condition, exactly at boundary), `hex_maxlen = SIZE_MAX`, `bin`/`hex` non-null | `abort()` → SIGABRT | [x] |
| E2 | `bin2hex` | `bin_len == LIMIT + 1` (`0x8000_0000_0000_0000`), `hex_maxlen = SIZE_MAX` | `abort()` → SIGABRT | [x] |
| E3 | `bin2hex` | `bin_len == SIZE_MAX`, `hex_maxlen = SIZE_MAX` | `abort()` → SIGABRT | [x] |
| E4 | `bin2hex` | first condition dominates even when second would pass: `bin_len == LIMIT`, `hex_maxlen == 0` | `abort()` → SIGABRT | [x] |
| E5 | `bin2hex` | second condition, exact equality: `hex_maxlen == bin_len * 2` (`bin_len = 4`, `hex_maxlen = 8`) | `abort()` → SIGABRT | [x] |
| E6 | `bin2hex` | second condition, one below: `hex_maxlen == bin_len * 2 - 1` (`bin_len = 4`, `hex_maxlen = 7`) | `abort()` → SIGABRT | [x] |
| E7 | `bin2hex` | second condition, `hex_maxlen == 0` with `bin_len == 0` (degenerate: `0 <= 0`) | `abort()` → SIGABRT | [x] |
| E8 | `bin2hex` | second condition, `hex_maxlen == 0` with `bin_len == 1` | `abort()` → SIGABRT | [x] |
| E9 | `bin2hex` | second condition, `hex_maxlen == 1` with `bin_len == 1` (`1 <= 2`) | `abort()` → SIGABRT | [x] |
| E10 | `bin2hex` | second condition, `hex_maxlen == 2` with `bin_len == 1` (`2 <= 2`) | `abort()` → SIGABRT | [x] |
| E11 | `bin2hex` | second condition swept: for `bin_len` in `0..=32`, every `hex_maxlen` in `0..=bin_len*2` | `abort()` → SIGABRT for all | [x] |
| E12 | `bin2hex` | negative control / one step past guard into the valid range: `hex_maxlen == bin_len * 2 + 1` | NO abort; normal return of `hex` | [x] |
| E13 | `bin2hex` | negative control on first guard: `bin_len == LIMIT - 1` (guard 1 passes), `hex_maxlen = SIZE_MAX`, `hex`/`bin` = NULL → loop is entered and dereferences NULL | fatal memory fault (SIGSEGV), **not** SIGABRT | [x] |
| E14 | `bin2hex` | `hex == NULL` with `bin_len == 0`, `hex_maxlen == 1` (guard passes, then `hex[0] = 0`) | fatal memory fault (SIGSEGV) | [x] |
| E15 | `bin2hex` | `hex == NULL` with `bin_len == 8`, `hex_maxlen == 17` | fatal memory fault (SIGSEGV) | [x] |
| E16 | `bin2hex` | `bin == NULL` with `bin_len == 8`, valid `hex` (loop reads `bin[0]`) | fatal memory fault (SIGSEGV) | [x] |
| E17 | `bin2hex` | `bin == NULL` with `bin_len == 0`, valid `hex`, `hex_maxlen == 1` (loop never runs, `bin` never read) | NO fault; returns `hex`, writes `"\0"` | [x] |
| E18 | `bin2hex` | both `hex == NULL` and `bin == NULL`, `bin_len == 0`, `hex_maxlen == 1` | fatal memory fault (SIGSEGV, from `hex[0]`) | [x] |
| E19 | `bin2hex` | oversized `hex_maxlen` that lies about the buffer: `hex_maxlen = SIZE_MAX` but real buffer only `bin_len*2+1` bytes | NO abort; writes exactly `bin_len*2+1` bytes | [x] |
| E20 | `bin2hex` | out-of-range enum across FFI: **N/A** — the C API has no `enum` parameter (`grep enum` finds none); all four parameters are pointers / `size_t`, whose entire value range is covered by E1–E19. | — | [x] |

Notes on rows deliberately marked N/A / not directly executable:

- `bin_len == LIMIT - 1` with a *real* buffer (E13's positive counterpart) is
  not executable: it would require a ~2^64-byte output buffer and 2^63 loop
  iterations. E13 exercises the same guard boundary observably instead, by
  showing that at `LIMIT - 1` the guard does **not** fire (the process dies
  from the subsequent null dereference, i.e. SIGSEGV, not SIGABRT) while at
  `LIMIT` it does (E1: SIGABRT). This pins the constant `LIMIT` exactly.
- `bin_len * 2U` cannot wrap in the second condition, because the first
  condition already rejected everything `>= LIMIT`; `(LIMIT-1) * 2` =
  `0xFFFF_FFFF_FFFF_FFFC` fits in `size_t`. No wrap-around row is therefore
  reachable.
