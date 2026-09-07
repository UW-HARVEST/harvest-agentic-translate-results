# ERRORS.md — Error-surface table (Phase C gate)

Derived mechanically from the C source, not from documentation.

```
$ grep -n "return\|assert\|NULL\|ERROR\|errno\|exit\|if\|switch\|#ifdef\|#if \|enum\|MAX\|MIN" \
      c_src/src/driver.c c_src/include/driver.h
c_src/src/driver.c:7:// modify, merge, publish, distribute, sublicense,      <- licence comment
c_src/include/driver.h:7:// modify, merge, publish, distribute, sublicense,  <- licence comment
c_src/include/driver.h:24:#ifndef DRIVER_H_                                  <- include guard
c_src/include/driver.h:31:#endif //DRIVER_H_                                 <- include guard
```

Mechanical finding: **the C library contains no error-return statement, no
`return` at all, no `assert`, no range check, no null check, no error enum,
and no min/max constant.** Both public functions return `void`. There is
therefore no error *code* surface to compare.

What remains are the implicit rejection/normalisation behaviours the C code
actually performs on invalid or out-of-range input. Each is one row.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `driver` | `x` has bits above bit 1 set (any value `> 3`, up to `UINT_MAX`) | no error; silently truncated by `and $0x3` — prints `x & 3` | `err_e1_x_out_of_range` |
| E2 | `driver` | `y` has bits above bit 2 set (any value `> 7`, up to `UINT_MAX`) | no error; silently truncated by `and $0x7` — prints `y & 7` | `err_e2_y_out_of_range` |
| E3 | `driver` | `b` byte is neither 0 nor 1 (an out-of-range `_Bool`, e.g. `2`, `0xFE`, `0xFF`) — the FFI analogue of an out-of-range enum value | no error; `movzbl` + `and $0x1` — prints `b & 1`. `0xFE` prints `0`, `0xFF` prints `1` | `err_e3_bool_out_of_range` |
| E4 | `driver` | `z == INT_MIN` / `z == INT_MAX` / `z < 0` (one step past, and at, the `int` range ends) | no error; `z` is a full-width `int` field, printed verbatim with `%d` | `err_e4_z_extremes` |
| E5 | `print_foo` | `foo` byte 0 has bits 6–7 set (padding bits inside the bit-field allocation unit, unreachable through `driver`) | no error; those bits belong to no member and are not read — output depends only on bits 0–5 | `err_e5_padding_bits_ignored` |
| E6 | `print_foo` | `foo` bytes 1–3 hold arbitrary garbage (struct padding, left uninitialised by `driver`) | no error; padding is never read — output unaffected | `err_e6_struct_padding_ignored` |
| E7 | `print_foo` | `foo == NULL` | unconditional dereference → `SIGSEGV`. Not a recoverable error; the C returns no code | `err_e7_null_pointer_segv` (both `.so`s crash with the same signal, checked in forked child processes) |

Notes on completeness of the generic boundaries required by Phase C:

* **null pointers** — row E7 (`print_foo`; `driver` takes no pointer).
* **zero lengths / oversized lengths** — not applicable: the API has no
  length, size, or count parameter and no buffers.
* **one step past a documented valid range** — rows E1, E2, E4.
* **out-of-range enum values across the FFI boundary** — row E3. C `_Bool`
  is the only enum-like parameter; the ABI passes it as a byte, so values
  with no valid variant (2…255) are real inputs. Covered exhaustively
  (all 256 byte values) by `err_e3_bool_out_of_range`.

All rows: **[x] passing** (see `translation/tests/differential.rs`).
