# Configuration Surface

Mechanically derived from the public declaration in
`../c_src/include/slicing.h` and every branch in
`../c_src/src/slicing.c`.

There is one public entry point and no compile-time or Cargo feature modes. Its
runtime axes are whether each optional index pointer is null, string length
(empty, one byte, or many bytes), and valid index position (zero, interior, or
exactly the string length). Rows prune combinations that cannot be valid, such
as a present stop for an empty string.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; empty string (`len = 0`) | [x] |
| 2 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; one-byte string | [x] |
| 3 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`; many-byte string | [x] |
| 4 | `slice` | present start, `stop_ptr = NULL`; empty string with `start = 0 = len` | [x] |
| 5 | `slice` | present start, `stop_ptr = NULL`; nonempty string with `start = 0` | [x] |
| 6 | `slice` | present start, `stop_ptr = NULL`; many-byte string with `0 < start < len` | [x] |
| 7 | `slice` | present start, `stop_ptr = NULL`; nonempty string with `start = len` | [x] |
| 8 | `slice` | `start_ptr = NULL`, present stop; nonempty string with `1 <= stop <= len`, including `stop = 1` and `stop = len` | [x] |
| 9 | `slice` | present start and stop; one-byte string with `start = 0`, `stop = 1` | [x] |
| 10 | `slice` | present start and stop; many-byte string with `0 <= start < stop <= len`, including zero/interior/end boundaries | [x] |

## Public entry-point coverage

- `slice`: rows 1-10.
- Lower-level public entry points: none.
- Convenience wrappers: none.
- Binary executable: none is built by either `Cargo.toml` or
  `CMakeLists.txt`.
