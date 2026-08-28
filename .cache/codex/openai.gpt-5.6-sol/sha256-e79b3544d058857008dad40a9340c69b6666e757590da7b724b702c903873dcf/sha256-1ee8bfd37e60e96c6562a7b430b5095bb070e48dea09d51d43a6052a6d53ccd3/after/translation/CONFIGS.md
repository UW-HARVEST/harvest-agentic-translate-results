# Configuration Surface

The public surface has one entry point and no Cargo features, C preprocessor
feature branches, enums, modes, or flags. The runtime axes are whether each
optional index pointer is null and the effective C-string shape determined by
the first NUL byte. Invalid combinations are enumerated in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `slice` | empty C string; `start_ptr = NULL`; `stop_ptr = NULL` | [x] |
| 2 | `slice` | empty C string; `*start_ptr = 0`; `stop_ptr = NULL` | [x] |
| 3 | `slice` | one-byte C string; `start_ptr = NULL`; `stop_ptr = NULL` | [x] |
| 4 | `slice` | one-byte C string; `*start_ptr = 0`; `stop_ptr = NULL` | [x] |
| 5 | `slice` | one-byte C string; `*start_ptr = len`; `stop_ptr = NULL` | [x] |
| 6 | `slice` | one-byte C string; `start_ptr = NULL`; `*stop_ptr = len` | [x] |
| 7 | `slice` | one-byte C string; `*start_ptr = 0`; `*stop_ptr = len` | [x] |
| 8 | `slice` | multi-byte C string; `start_ptr = NULL`; `stop_ptr = NULL` | [x] |
| 9 | `slice` | multi-byte C string; `*start_ptr = 0`; `stop_ptr = NULL` | [x] |
| 10 | `slice` | multi-byte C string; `*start_ptr` is interior; `stop_ptr = NULL` | [x] |
| 11 | `slice` | multi-byte C string; `*start_ptr = len`; `stop_ptr = NULL` | [x] |
| 12 | `slice` | multi-byte C string; `start_ptr = NULL`; `*stop_ptr` is interior | [x] |
| 13 | `slice` | multi-byte C string; `start_ptr = NULL`; `*stop_ptr = len` | [x] |
| 14 | `slice` | multi-byte C string; `*start_ptr = 0`; `*stop_ptr = len` | [x] |
| 15 | `slice` | multi-byte C string; both indices interior and `start < stop` | [x] |
| 16 | `slice` | embedded NUL with trailing bytes; both pointers null; effective length ends at first NUL | [x] |
| 17 | `slice` | embedded NUL with trailing bytes; valid non-null start and null stop, bounded by effective length | [x] |
| 18 | `slice` | embedded NUL with trailing bytes; null start and valid non-null stop, bounded by effective length | [x] |
| 19 | `slice` | embedded NUL with trailing bytes; valid non-null start and stop, both bounded by effective length | [x] |

Build configuration cross-product: one configuration only. `Cargo.toml`
declares no features, so default and `--no-default-features` select identical
code.
