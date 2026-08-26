# Configuration Surface

## Build-Time Matrix

| # | Rust features | CMake configuration | [ ] |
|---|---------------|----------------------|-----|
| 1 | no features (`--no-default-features --features ""`) | default `driver` shared-library target; no CMake options or source `#if` branches | [x] |

## Runtime Axes

The exported ABI has no option, mode, flag, enum, length, element-type, format,
or byte-order parameter. The C branches distinguish negative/nonnegative
integers, successful/failed opens, zero/one/many successful `fgets` iterations,
and clean/error EOF. Valid integer rows also retain the zero and signed
multiplication-wrap boundaries because the compiled C artifact treats those
value classes differently at the returned-byte level.

File shapes used below:

- **empty**: readable file; zero successful `fgets` calls.
- **one chunk**: readable nonempty file of at most 99 bytes with no earlier
  newline; one successful `fgets` call.
- **many chunks**: readable file causing two or more successful `fgets` calls
  through multiple lines and/or data longer than the 99-byte payload buffer.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `forward_goto_example` | no options; `x == 0` boundary | [x] |
| 2 | `forward_goto_example` | no options; `1 <= x <= INT_MAX / 2`, doubled result is representable | [x] |
| 3 | `forward_goto_example` | no options; `INT_MAX / 2 < x <= INT_MAX`, compiled C doubled result wraps | [x] |
| 4 | `open_with_cleanup` | no options; readable empty file | [x] |
| 5 | `open_with_cleanup` | no options; readable one-chunk file | [x] |
| 6 | `open_with_cleanup` | no options; readable many-chunk file | [x] |
| 7 | `driver` | `num == 0`; readable empty file | [x] |
| 8 | `driver` | representable positive doubled result; readable empty file | [x] |
| 9 | `driver` | wrapping positive doubled result; readable empty file | [x] |
| 10 | `driver` | `num == 0`; readable one-chunk file | [x] |
| 11 | `driver` | representable positive doubled result; readable one-chunk file | [x] |
| 12 | `driver` | wrapping positive doubled result; readable one-chunk file | [x] |
| 13 | `driver` | `num == 0`; readable many-chunk file | [x] |
| 14 | `driver` | representable positive doubled result; readable many-chunk file | [x] |
| 15 | `driver` | wrapping positive doubled result; readable many-chunk file | [x] |
