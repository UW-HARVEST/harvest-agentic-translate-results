# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Defined (exported) symbols

| # | symbol | C source | C `.so` | Rust `.so` | Rust impl site |
|---|--------|----------|---------|------------|----------------|
| 1 | `allocate_matrix` | `src/matrix.c:33` (non-static, absent from `matrix.h` but exported) | T | T | `src/matrix.rs` |
| 2 | `free_matrix` | `src/matrix.c:66` | T | T | `src/matrix.rs` |
| 3 | `initialize_matrix_from_string` | `src/matrix.c:78` | T | T | `src/matrix.rs` |
| 4 | `multiply_matrices` | `src/matrix.c:118` | T | T | `src/matrix.rs` |
| 5 | `matrix_to_string` | `src/matrix.c:137` | T | T | `src/matrix.rs` |
| 6 | `write_to_file` | `src/write.c:32` | T | T | `src/write.rs` |
| 7 | `driver` | `src/driver.c:35` | T | T | `src/driver.rs` |

C exports 7 symbols. Rust exports the same 7. **Symbol diff: EMPTY.**

There are no macro-generated symbols in this library (the only `#define` in the
C sources is `OUT_FILE "matrix.txt"`, a string literal, not a symbol factory).

## Translated-module coverage

Every C translation unit in `CMakeLists.txt` has a Rust counterpart, so no
module was skipped:

| C file | Rust file |
|--------|-----------|
| `src/matrix.c` | `src/matrix.rs` |
| `src/write.c` | `src/write.rs` |
| `src/driver.c` | `src/driver.rs` |
| (libc usage) | `src/cstd.rs` — `extern "C"` declarations, calls straight through to glibc |

`src/cutil.rs` exists on disk but is not declared as a module in `src/lib.rs`,
so it is not compiled and contributes no symbols. Harmless dead file.

## Undefined symbols

`nm -D --undefined-only` on the Rust `.so` lists only glibc
(`malloc`, `free`, `strdup`, `strtok_r`, `atoi`, `perror`, `fprintf`,
`snprintf`, `fopen`, `fclose`, `strerror`, `strlen`, `strcat`/`memcpy`,
`stderr`, `__errno_location`, …) and the libgcc unwinder (`_Unwind_*`)
plus Rust-runtime libc usage (`mmap64`, `dl_iterate_phdr`, `pthread_key_*`, …).

**0 missing / undefined non-libc symbols.**

Checklist:

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every C-exported symbol is exported by Rust with the exact same name.
