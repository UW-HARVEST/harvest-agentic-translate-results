# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/lib.c` (22 lines) and
`c_src/include/lib.h` (1 line).

Grep used to enumerate every rejection construct:

```sh
grep -nE 'return|assert|NULL|-1|if|else|switch|#if|error|ERROR|errno|<|>|==|!=' \
    c_src/src/lib.c c_src/include/lib.h
```

Complete set of matches:

```
src/lib.c:3:#include <string.h>
src/lib.c:13:  if(s1 && s2) {
src/lib.c:14:    path = (s1 > s2) ? s1 + 1 : s2 + 1;
src/lib.c:16:  else if(s1)
src/lib.c:18:  else if(s2)
src/lib.c:21:  return path;
```

## Findings

The C implementation contains:

* **0** error-return macros (`RETURN_ERROR`, `CURLE_*`, `goto fail`, …)
* **0** `return -1` / `return NULL` / error-enum returns — the single `return`
  statement (line 21) unconditionally returns a valid `char *` derived from the
  caller's own pointer
* **0** `assert` / `abort` / `errno` assignments
* **0** explicit range checks, length checks, or min/max constants
* **0** NULL checks on the `path` argument
* **0** enum parameters (the only parameter is `char *`), so there is no
  out-of-range-enum class of input for this API

The three `if` branches at lines 13/16/18 are **not** rejections: they are
valid-path dispatch on which separator was found, and every branch falls through
to the same successful `return path`. Those branches are therefore enumerated in
`CONFIGS.md`, not here.

## Error-surface table

The only way to make this function *not* return normally is to violate its
implicit precondition (`path` must be a readable NUL-terminated string), which
is undefined behaviour in C rather than a diagnosed rejection. Both rows below
are still tested differentially (in an isolated child process, comparing the
fatal signal), because they are the only rejection-shaped behaviour that exists.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `tool_basename` | `path == NULL` — no NULL check exists, so `strrchr(NULL, '/')` dereferences the null page | Undefined behaviour; observably the process dies with `SIGSEGV` (signal 11). No error code is returned. Rust must die the same way — it must **not** return NULL, must **not** panic with a Rust message, and must **not** "handle" the NULL. |
| 2 | `tool_basename` | `path` points to a buffer with **no NUL terminator** (unterminated / non-readable tail) — no length parameter and no bound exists, so the scan runs off the end of the allocation | Undefined behaviour; observably reads past the buffer and, when the tail is unmapped, dies with `SIGSEGV` (signal 11). Rust must exhibit the same unbounded scan and same fatal signal. |

### Generic FFI boundaries also covered by tests (not distinct C rejections)

| boundary | why it is not a table row | where tested |
|----------|---------------------------|--------------|
| zero length (`""`, i.e. `path[0] == '\0'`) | Accepted, not rejected: `strrchr` returns NULL twice, all three `if`s are false, `path` is returned unchanged. This is a *valid* configuration. | `CONFIGS.md` row 1 |
| oversized length (multi-megabyte string) | Accepted; no length limit exists. | `CONFIGS.md` row 16 |
| "one step past a valid range" for the separator bytes — i.e. `'/'-1 = 0x2E ('.')`, `'/'+1 = 0x30 ('0')`, `'\\'-1 = 0x5B ('[')`, `'\\'+1 = 0x5D (']')` | Accepted; these are ordinary non-separator bytes. Included because an off-by-one in the Rust `strrchr` comparison would show up exactly here. | `CONFIGS.md` row 13 |
| out-of-range enum value across FFI | **Not applicable** — the API takes no enum, no `int` mode, and no flags. Confirmed by the 1-line header. | n/a |
| signed-vs-unsigned `char` handling of bytes `0x80..=0xFF` | Accepted; but on x86-64 Linux `c_char` is signed `i8`, so a naive Rust comparison could mis-handle high bytes. | `CONFIGS.md` rows 3 and 15 |
| byte `0x00` as the searched character | Not reachable: the C only ever searches for `'/'` and `'\\'`, both non-zero. The private Rust `strrchr` still replicates C's rule that the terminator is part of the searched string. | n/a (private helper) |

## Status

| # | test | status |
|---|------|--------|
| 1 | `err_row1_null_pointer_same_fatal_signal` | [x] PASSES — both C and Rust die with SIGSEGV |
| 2 | `err_row2_unterminated_buffer_same_fatal_signal` | [x] PASSES — both C and Rust die with SIGSEGV |

## Divergence found and fixed by row 1

Row 1 initially FAILED in the **dev/test** profile: C died with `SIGSEGV` (11)
while the Rust `.so` died with `SIGABRT` (6). Cause: Rust debug builds enable
`cfg(ub_checks)` (it follows `debug-assertions`), which turns the NULL
dereference inside `strrchr` into a panic instead of a fault. The C compiler
emits no such instrumentation, so this was a genuine behavioural divergence from
the ground truth — visible only on the error path, which is exactly why Phase C
exists (the release profile matched all along, and every Phase B test passed).

Fix (in `translation/Cargo.toml`, not in `c_src/`):

```toml
[profile.dev]
debug-assertions = false
overflow-checks = false
panic = "abort"

[profile.test]
debug-assertions = false
overflow-checks = false
```

Row 1 now passes in both profiles. The translated logic in `src/lib.rs` was
correct and was not changed.
