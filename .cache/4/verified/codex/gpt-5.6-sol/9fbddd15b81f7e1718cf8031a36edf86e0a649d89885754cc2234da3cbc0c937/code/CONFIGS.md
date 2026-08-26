# Configuration Surface

The only public and lowest-level entry point is `slice`. Its runtime axes are:

- `start_ptr`: null (default start `0`) or present.
- `stop_ptr`: null (default stop `strlen(mystr)`) or present.
- String shape: logical length zero, one, or many bytes. `strlen` makes the
  first NUL the logical end; bytes after it are ignored.
- Present index shape: zero, strictly interior, or exactly the logical end.
- Selected span: empty (possible only with a null stop), full, prefix, suffix,
  or interior.

All nonempty rows are exercised with randomized byte strings, including
non-UTF-8 bytes, and with bytes after the first NUL. Rows allowing multiple
length/span classes exercise each class.

`Cargo.toml` has no `[features]` section. `c_src/CMakeLists.txt` has no options,
conditional definitions, or conditional sources. Therefore the full
build-time matrix has exactly one member: no Rust features / the default CMake
configuration.

| # | entry point(s) | configuration (options set + input shape) | Test |
|---|----------------|--------------------------------------------|------|
| 1 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`; logical length `0`; full empty string | [x] |
| 2 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`; logical length `1` or many; full string | [x] |
| 3 | `slice` | `start_ptr=&0`, `stop_ptr=NULL`; logical length `0`; full empty string | [x] |
| 4 | `slice` | `start_ptr=&0`, `stop_ptr=NULL`; logical length `1` or many; full string | [x] |
| 5 | `slice` | `start_ptr=&start`, `stop_ptr=NULL`; many-byte string and `0 < start < len`; suffix | [x] |
| 6 | `slice` | `start_ptr=&len`, `stop_ptr=NULL`; logical length `1` or many; empty span at end | [x] |
| 7 | `slice` | `start_ptr=NULL`, `stop_ptr=&len`; logical length `1` or many; full string | [x] |
| 8 | `slice` | `start_ptr=NULL`, `stop_ptr=&stop`; many-byte string and `0 < stop < len`; prefix | [x] |
| 9 | `slice` | `start_ptr=&0`, `stop_ptr=&len`; logical length `1` or many; full string | [x] |
| 10 | `slice` | `start_ptr=&0`, `stop_ptr=&stop`; many-byte string and `0 < stop < len`; prefix | [x] |
| 11 | `slice` | `start_ptr=&start`, `stop_ptr=&len`; many-byte string and `0 < start < len`; suffix | [x] |
| 12 | `slice` | Both pointers present; many-byte string and `0 < start < stop < len`; interior span of one or many bytes | [x] |
