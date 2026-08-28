# ERRORS.md — Phase A error-surface table

## Mechanical derivation

Every rejection path in the library was found by grepping the complete C source
(22 lines of `src/lib.c`, 1 line of `include/lib.h`) for every construct that
could reject input:

```sh
grep -nE 'return|assert|NULL|ERROR|errno|if|else|\?|<|>|==|!=|#if|#define|enum|switch' \
     c_src/src/lib.c c_src/include/lib.h
```

Result:

```
src/lib.c:3:#include <string.h>
src/lib.c:13:  if(s1 && s2) {
src/lib.c:14:    path = (s1 > s2) ? s1 + 1 : s2 + 1;
src/lib.c:16:  else if(s1)
src/lib.c:18:  else if(s2)
src/lib.c:21:  return path;
```

Findings, stated exactly as the source shows them:

* **`return` statements: 1** (`return path;`). There is no `return -1`, no
  `return NULL`, no `RETURN_ERROR`-style macro.
* **`assert`: 0.**
* **explicit range checks / min-max constants: 0.**
* **NULL checks on the `path` argument: 0.** `path` goes straight into
  `strrchr()`.
* **error enums / error codes / `errno` use: 0.**
* **`#if` / `#ifdef` / `switch`: 0.** There are no enum parameters anywhere in
  the public API (the only parameter is `char *`), so there is no
  out-of-range-enum-value case to construct.

**`tool_basename` therefore has no error return at all.** It is total over every
valid NUL-terminated string: it always returns a non-NULL pointer into the
caller's buffer. The only conditions that can be called "rejections" are (a) the
degenerate/no-separator inputs that fall through the `if` chain to the untouched
`return path;`, and (b) the one input the C cannot handle — `NULL` — which it
does not check and therefore faults on.

The table below has one row per distinct such condition. The three `if` branches
on lines 13/16/18 are *success* paths and belong to `CONFIGS.md`; the row for
the implicit `else` (no separator at all) is here because it is the fall-through
of that rejection-shaped chain.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `tool_basename` | `path == NULL`. No NULL check exists; `strrchr(NULL, '/')` dereferences it. | Undefined behaviour → process dies on `SIGSEGV` (11). Not an error *return*: no value comes back. | `phase_c_null_ptr.rs::null_pointer_differential` (both `.so`s called in forked child processes; compared termination signals) | ✅ |
| E2 | `tool_basename` | `path` = `""` (empty string, length 0). Both `strrchr` calls return `NULL`, so the `if`/`else if`/`else if` chain is skipped entirely. | Returns `path` unchanged (offset 0); result is `""`. Never `NULL`. | `phase_c_errors.rs::e2_empty_string` | ✅ |
| E3 | `tool_basename` | `path` contains **no** `'/'` and **no** `'\\'` (e.g. `"filename.txt"`). `s1 == NULL && s2 == NULL`; all three branches fail. | Returns `path` unchanged (offset 0). Never `NULL`. | `phase_c_errors.rs::e3_no_separator_at_all` | ✅ |
| E4 | `tool_basename` | Separator is the **last** byte, so the "basename" is empty (`"dir/"`, `"dir\\"`). `s1`/`s2` points at the final byte; `s1 + 1` is the NUL terminator. | Returns a pointer to the NUL terminator = `""` at offset `len-1+1 == len`. This is a valid in-bounds pointer, not an error. | `phase_c_errors.rs::e4_trailing_separator_yields_empty` | ✅ |
| E5 | `tool_basename` | Separator is the **only** byte (`"/"`, `"\\"`). Degenerate form of E4 with `len == 1`. | Returns pointer to the NUL terminator = `""` at offset 1. | `phase_c_errors.rs::e5_separator_only` | ✅ |
| E6 | `tool_basename` | A byte one step **outside** the separator values is passed where a separator would be: `'.'`(0x2E) / `'0'`(0x30) around `'/'`(0x2F), and `'['`(0x5B) / `']'`(0x5D) around `'\\'`(0x5C). These are the "one past a valid range" inputs for the two byte comparisons. | Not treated as separators → behaves like E3 (returns offset 0). | `phase_c_errors.rs::e6_bytes_adjacent_to_separators` | ✅ |
| E7 | `tool_basename` | High-bit bytes `0x80..=0xFF` in the buffer. `char` is **signed** on x86-64, so a naive signed comparison could sign-extend and mis-compare; `strrchr` compares as `unsigned char`. Includes `0xAF` (= `0x2F \| 0x80`) and `0xDC` (= `0x5C \| 0x80`). | Never matched as separators. Result is decided only by real `'/'`/`'\\'` bytes. | `phase_c_errors.rs::e7_high_bit_bytes_are_not_separators` | ✅ |
| E8 | `tool_basename` | Oversized input: buffer of 1 MiB with the separator at the very end, and a 1 MiB buffer with no separator. There is no length cap in the C. | No truncation, no cap, no error: returns the correct interior pointer at offset ~1 MiB. | `phase_c_errors.rs::e8_oversized_input` | ✅ |
| E9 | `tool_basename` | Buffer whose bytes are **not valid UTF-8** (e.g. lone `0xFF`, truncated multi-byte sequences), with and without separators. C is byte-oriented and has no encoding validation; a Rust port that went through `str`/`from_utf8` would reject or panic here. | Byte-wise result exactly as for any other bytes; no validation, no error. | `phase_c_errors.rs::e9_invalid_utf8_bytes` | ✅ |
| E10 | `tool_basename` | Buffer containing **only** the NUL terminator at a nonzero index cannot happen, but the mirror case can: bytes appearing *after* the NUL terminator (separators hidden in the tail of an oversized allocation). C stops at the NUL. | Bytes past the NUL are invisible; result must ignore them (identical to E3/E2 for the visible prefix). | `phase_c_errors.rs::e10_bytes_after_nul_are_invisible` | ✅ |

## Notes on generic boundaries required by Phase C

| generic boundary | how it is covered |
|---|---|
| NULL pointer | E1 (subprocess-based signal comparison — the only way to observe UB differentially without killing the harness) |
| zero length | E2 (`""`) |
| oversized length | E8 (1 MiB buffers) |
| one step past a documented valid range | E6 (bytes `0x2E`/`0x30` and `0x5B`/`0x5D`, i.e. the neighbours of the two separator byte values) and E7 (high-bit / sign-extension neighbours) |
| out-of-range enum value across the FFI boundary | **not applicable, and this is verified rather than assumed**: `grep -E 'enum|switch' c_src/src/lib.c c_src/include/lib.h` finds nothing, and the sole public prototype is `char *tool_basename(char *path)` — there is no integer or enum parameter in the entire ABI, so no invalid discriminant can be constructed. |
| return-value sentinel | The C never returns `NULL`; tests assert the Rust never does either, and compare the exact byte **offset** of the returned interior pointer, not just "both non-NULL". |
| output-buffer mutation | The C never writes through `path`; tests assert the input buffer is byte-identical after both calls (E2–E10 and all Phase B rows). |

**All 10 rows have a passing differential test. Phase C gate satisfied.**
