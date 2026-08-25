# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` declares no
options or conditional targets. There is exactly one valid feature
combination:

| # | Cargo feature set | CMake configuration | [ ] |
|---|-------------------|---------------------|-----|
| BT1 | Empty set: `--no-default-features --features ""` | Default shared `driver` target, POSIX separator branch on this host | [x] |

The C source also contains a compiler-platform branch for `_MSC_VER`,
`__MINGW32__`, or `__MSVCRT__`. None is defined by the configured Linux C
compiler, and it is not a Cargo feature or CMake option.

## Runtime Configurations

Rows E1-E5 exercise the lowest-level exported entry point directly. Rows
F1-F18 exercise the composed allocation/copy entry point. Each row is tested
with many deterministic randomized byte strings.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| E1 | `extractFilename` | Separator absent; empty, one-byte, and many-byte paths | [x] |
| E2 | `extractFilename` | Exactly one separator in the path, not trailing | [x] |
| E3 | `extractFilename` | Multiple separators; result starts after the last one | [x] |
| E4 | `extractFilename` | Separator is the final path byte; result is the terminating empty string | [x] |
| E5 | `extractFilename` | Separator value is NUL; `strrchr` finds the terminator and returns one-past it | [x] |
| F1 | `FIO_createFilename_fromOutDir` | Path has no `/`; output directory has no trailing `/`; `suffixLen == 0` | [x] |
| F2 | `FIO_createFilename_fromOutDir` | Path has no `/`; output directory has no trailing `/`; positive `suffixLen` | [x] |
| F3 | `FIO_createFilename_fromOutDir` | Path has no `/`; output directory ends in `/`; `suffixLen == 0` | [x] |
| F4 | `FIO_createFilename_fromOutDir` | Path has no `/`; output directory ends in `/`; positive `suffixLen` | [x] |
| F5 | `FIO_createFilename_fromOutDir` | Path has one interior `/`; output directory has no trailing `/`; `suffixLen == 0` | [x] |
| F6 | `FIO_createFilename_fromOutDir` | Path has one interior `/`; output directory has no trailing `/`; positive `suffixLen` | [x] |
| F7 | `FIO_createFilename_fromOutDir` | Path has one interior `/`; output directory ends in `/`; `suffixLen == 0` | [x] |
| F8 | `FIO_createFilename_fromOutDir` | Path has one interior `/`; output directory ends in `/`; positive `suffixLen` | [x] |
| F9 | `FIO_createFilename_fromOutDir` | Path has multiple `/` bytes; output directory has no trailing `/`; `suffixLen == 0` | [x] |
| F10 | `FIO_createFilename_fromOutDir` | Path has multiple `/` bytes; output directory has no trailing `/`; positive `suffixLen` | [x] |
| F11 | `FIO_createFilename_fromOutDir` | Path has multiple `/` bytes; output directory ends in `/`; `suffixLen == 0` | [x] |
| F12 | `FIO_createFilename_fromOutDir` | Path has multiple `/` bytes; output directory ends in `/`; positive `suffixLen` | [x] |
| F13 | `FIO_createFilename_fromOutDir` | Path ends in `/`; output directory has no trailing `/`; `suffixLen == 0` | [x] |
| F14 | `FIO_createFilename_fromOutDir` | Path ends in `/`; output directory has no trailing `/`; positive `suffixLen` | [x] |
| F15 | `FIO_createFilename_fromOutDir` | Path ends in `/`; output directory ends in `/`; `suffixLen == 0` | [x] |
| F16 | `FIO_createFilename_fromOutDir` | Path ends in `/`; output directory ends in `/`; positive `suffixLen` | [x] |
| F17 | `FIO_createFilename_fromOutDir` | Empty output-directory string backed by a preceding `/` guard byte | [x] |
| F18 | `FIO_createFilename_fromOutDir` | Empty output-directory string backed by a preceding non-`/` guard byte | [x] |
