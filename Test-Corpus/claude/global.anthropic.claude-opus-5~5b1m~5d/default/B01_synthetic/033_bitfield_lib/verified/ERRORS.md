# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep audit of the whole C source

| pattern searched | hits in `c_src/` |
|------------------|------------------|
| `return` (any)   | 0 — both functions are `void` and have no `return` statement |
| `return -1` / `return NULL` / `return 0` | 0 |
| `RETURN_ERROR` / `goto` / error macro | 0 |
| `assert` / `static_assert` / `abort` / `exit` | 0 |
| `errno` | 0 |
| `if` / `switch` / `?:` / any conditional | 0 |
| `enum` | 0 |
| min/max constants, `#define`d limits | 0 |
| null checks (`== NULL`, `!ptr`) | 0 |
| `malloc` / `free` / allocation failure paths | 0 |

**Conclusion: the C library has NO error-return surface at all.** Both
`driver` and `print_foo` return `void`, are branch-free, and validate nothing.
The "error surface" is therefore entirely made of *implicit* rejections: values
silently truncated by the bit-fields, and undefined behaviour on a bad pointer.
Every such implicit rejection is enumerated below — one row each — and each has
a differential test that asserts C and Rust produce the *same* observable
result (identical stdout bytes, or identical fatal signal).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `driver` | `x` out of range of a 2-bit field (`x > 3`, e.g. `4`, `5`, `0xFFFFFFFF`) | no error; `x` silently truncated to `x & 0x3`, printed via `%u` | `err_x_out_of_range` | [x] |
| 2 | `driver` | `y` out of range of a 3-bit field (`y > 7`, e.g. `8`, `9`, `0xFFFFFFFF`) | no error; `y` silently truncated to `y & 0x7`, printed via `%u` | `err_y_out_of_range` | [x] |
| 3 | `driver` | `b` is a non-canonical `_Bool` byte (any value except 0/1, e.g. `2`, `0x80`, `0xFF`) — a real FFI input, since C `_Bool` accepts any byte across the ABI | no error; gcc emits `movzbl; and $0x1`, so only **bit 0** survives: prints `1` iff `b & 1`, else `0` | `err_b_non_canonical_byte` | [x] |
| 4 | `driver` | `x` exactly one step past the valid 2-bit range (`x == 4`, i.e. boundary+1) | prints `0` for the `x` column | `err_boundary_one_past_x` | [x] |
| 5 | `driver` | `y` exactly one step past the valid 3-bit range (`y == 8`, i.e. boundary+1) | prints `0` for the `y` column | `err_boundary_one_past_y` | [x] |
| 6 | `driver` | `x`/`y` at the maximum of their C parameter type (`UINT_MAX`), far past the field range | prints `3` and `7` respectively | `err_uint_max_args` | [x] |
| 7 | `driver` | `z == INT_MIN` (extreme of the signed range; `%d` of the most negative int) | prints `-2147483648` | `err_z_int_min` | [x] |
| 8 | `driver` | `z == INT_MAX` | prints `2147483647` | `err_z_int_max` | [x] |
| 9 | `driver` | `z` negative in general (the `int` field is signed, unlike `x`/`y`) | prints the negative value verbatim via `%d`, no truncation (`z` is a full `int`, not a bit-field) | `err_z_negative` | [x] |
| 10 | `print_foo` | struct storage byte has the **padding bits 6..7 set** (an "invalid"/garbage storage unit reachable only through the raw-pointer entry point; C `driver` itself leaves those bits uninitialised) | no error; bits 6..7 are ignored — output depends only on bits 0..5 and on `z` | `err_padding_bits_ignored` | [x] |
| 11 | `print_foo` | struct storage byte `0xFF` (every bit-field saturated, incl. padding) | prints `3 7 1 <z>` | `err_all_bits_set` | [x] |
| 12 | `print_foo` | the 3 bytes of inter-field padding at offsets 1..3 hold garbage | no error; ignored, output unaffected | `err_inter_field_padding_ignored` | [x] |
| 13 | `print_foo` | `foo == NULL` — the C code dereferences unconditionally with **no null check** | undefined behaviour: in practice `SIGSEGV` (fatal signal 11), no error code | `err_null_pointer_both_crash` (runs both in forked children, compares exit status) | [x] |
| 14 | `print_foo` | `foo` is a wild / unmapped non-null pointer | undefined behaviour: in practice `SIGSEGV` | `err_wild_pointer_both_crash` | [x] |
| 15 | `driver` / `print_foo` | "out-of-range enum value across the FFI boundary" — **N/A**: the C source declares no `enum`. The nearest equivalent is the `_Bool` parameter, whose only two valid variants are 0/1; every other byte is covered by row 3 (exhaustively, all 256 byte values). | — | `err_b_all_256_bytes_exhaustive` | [x] |
| 16 | `driver` | "zero and oversized lengths" — **N/A**: no pointer+length parameter pairs, no buffers, no counts exist anywhere in the API. Closest analogue is the all-zero input and the all-max input. | prints `0 0 0 0` / `3 7 1 2147483647` | `err_all_zero_and_all_max` | [x] |
