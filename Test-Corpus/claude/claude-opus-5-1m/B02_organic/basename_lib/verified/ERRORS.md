# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Derived **mechanically** from the complete C source. The entire library is:

```c
char *tool_basename(char *path)
{
  char *s1;
  char *s2;

  s1 = strrchr(path, '/');
  s2 = strrchr(path, '\\');

  if(s1 && s2) {
    path = (s1 > s2) ? s1 + 1 : s2 + 1;
  }
  else if(s1)
    path = s1 + 1;
  else if(s2)
    path = s2 + 1;

  return path;
}
```

Mechanical grep of the C source for every rejection mechanism:

```sh
grep -nE "RETURN_ERROR|return +-?[0-9]|return +NULL|assert|errno|enum|EINVAL" c_src/src/lib.c
# -> no matches
grep -nE "return|if *\(|else" c_src/src/lib.c
# -> 13:if(s1 && s2)  14:(s1 > s2) ? ...  16:else if(s1)  18:else if(s2)  21:return path
```

**Findings:** the C function has *no* error return, *no* sentinel `NULL` return,
*no* `assert`, *no* `errno` use, *no* range check, *no* min/max constant, *no*
enum parameter and *no* length parameter. `tool_basename` is total over all
valid NUL-terminated strings: every input either returns `path` unchanged or a
pointer strictly inside the same buffer. The rejection surface is therefore made
up entirely of the *contract violations* the C code does **not** check, plus the
generic FFI boundaries required by the task. Those are enumerated below, one row
per distinct condition, and each row has a differential test.

| # | function | trigger (exact invalid input / condition) | expected C result | test |
|---|----------|-------------------------------------------|-------------------|------|
| E1 | `tool_basename` | `path == NULL` — C does not null-check; `strrchr(NULL, '/')` dereferences address 0 | no error code: process faults, terminated by `SIGSEGV` (11). Never returns. | `tests/errors.rs::e1_null_pointer` (forked child, compares wait-status of C vs Rust) |
| E2 | `tool_basename` | `path` points at a buffer that is **not NUL-terminated** and is followed by unmapped memory — C scans past the end | no error code: process faults reading the guard page, terminated by `SIGSEGV` (11). Never returns. | `tests/errors.rs::e2_unterminated_buffer_guard_page` (forked child, `mmap` + `PROT_NONE` guard page) |
| E3 | `tool_basename` | zero-length input: `path` points at `""` (immediate NUL) — degenerate/empty length | no error: `strrchr` returns `NULL` twice, both `if` arms fail, returns `path` **unchanged** (same pointer, offset 0) | `tests/errors.rs::e3_zero_length` |
| E4 | `tool_basename` | separator is the **last** byte, e.g. `"abc/"`, `"abc\\"`, `"/"`, `"\\"` — the "one step past the valid range" case: the returned pointer is `s+1` == the NUL terminator | no error: returns a pointer **to the NUL terminator**, i.e. the empty string, offset == `strlen(path)`. Pointer is still in-bounds (one past last char is legal). | `tests/errors.rs::e4_separator_is_last_byte` |
| E5 | `tool_basename` | input consisting *only* of separators, e.g. `"/"`, `"////"`, `"\\\\\\\\"`, `"/\\/\\"` | no error: returns pointer to the NUL terminator (empty basename) | `tests/errors.rs::e5_only_separators` |
| E6 | `tool_basename` | oversized input: 1 MiB and 4 MiB strings, with and without separators (no length parameter exists, so "too long" is unrepresentable) | no error: same rule applies; must not truncate or overflow | `tests/errors.rs::e6_oversized_input` |
| E7 | `tool_basename` | bytes with the high bit set (`0x80`–`0xFF`) in the path — `char` is *signed* on x86-64, so a naive `c_char` comparison could mis-handle them | no error: such bytes are never `'/'` (0x2F) or `'\\'` (0x5C); they are ordinary path characters | `tests/errors.rs::e7_high_bit_bytes` |
| E8 | `tool_basename` | data *after* the NUL terminator contains separators (`"ab\0/x"`) — must not be scanned | no error: scan stops at the NUL; trailing separators are invisible; returns `path` unchanged | `tests/errors.rs::e8_separator_after_nul` |
| E9 | `tool_basename` | `path` is a read-only / aliased buffer, result is fed back into `tool_basename` (idempotence of a returned interior pointer, incl. the empty-string result from E4) | no error: `tool_basename(tool_basename(p)) == tool_basename(p)` | `tests/errors.rs::e9_idempotent_on_result` |
| E10 | `tool_basename` | out-of-range enum value across the FFI boundary | **N/A** — verified by grep: the public header declares one function with a single `char *` parameter; there is no enum, no flag word and no `int` mode anywhere in the library, so no out-of-range enum input exists. Documented for completeness; no test possible. | — (n/a, no enum in ABI) |

## Findings (divergences found and fixed in the Rust, never in the C)

* **E1 — FAILED initially, now fixed.** The original translation re-implemented
  `strrchr` as a Rust byte loop (`let ch = *p;`). With `debug-assertions` on
  (any `dev`-profile build of the cdylib) rustc inserts a null-pointer-deref
  assertion, so `tool_basename(NULL)` printed
  `panicked ... null pointer dereference occurred` and terminated the process
  with **SIGABRT (6)**, whereas the C library faults with **SIGSEGV (11)** and
  prints nothing. Fix: `src/lib.rs` now calls the *same* libc `strrchr` the C
  source calls (declared in an `unsafe extern "C"` block), so the fault
  semantics, and therefore the observable behaviour for every input including
  the UB ones, are identical in **both** the `dev` and `release` profiles.
  After the fix E1 passes for `target/debug/libdriver.so` and
  `target/release/libdriver.so`.
* E2–E10 passed as first written; no other divergence was found.

Notes on E1/E2: these are the only inputs on which the C library's behaviour is
"rejection". Because the C code performs no check, the observable contract is
*crash*, not an error value. The tests therefore compare the **exact
termination status** (signal number) of a forked child that calls the C symbol
against one that calls the Rust symbol, so "both failed somehow" is not
accepted — the signal must be identical.
