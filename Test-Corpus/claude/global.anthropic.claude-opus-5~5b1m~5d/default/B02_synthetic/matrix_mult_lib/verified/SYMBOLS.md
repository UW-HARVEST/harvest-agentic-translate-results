# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

```
C   : c_src/build/libdriver.so
Rust: translation/target/release/libdriver.so
```

## C source → symbol inventory

| C file | function | `static`? | exported by C `.so` | Rust impl | Rust `#[no_mangle]` export |
|---|---|---|---|---|---|
| `src/matrix.c` | `allocate_matrix`      | no (not in header, but non-static → exported) | yes | `src/matrix.rs` | yes |
| `src/matrix.c` | `free_matrix`          | no | yes | `src/matrix.rs` | yes |
| `src/matrix.c` | `initialize_matrix_from_string` | no | yes | `src/matrix.rs` | yes |
| `src/matrix.c` | `multiply_matrices`    | no | yes | `src/matrix.rs` | yes |
| `src/matrix.c` | `matrix_to_string`     | no | yes | `src/matrix.rs` | yes |
| `src/write.c`  | `write_to_file`        | no | yes | `src/write.rs`  | yes |
| `src/driver.c` | `driver`               | no | yes | `src/driver.rs` | yes |

There are no macro-generated / renamed symbols: neither header contains any
namespacing or symbol-renaming macros, so the linker names equal the
source-level names.

## `nm -D --defined-only` — C

```
allocate_matrix                 T
free_matrix                     T
initialize_matrix_from_string   T
multiply_matrices               T
matrix_to_string                T
write_to_file                   T
driver                          T
```

(7 defined `T` symbols; everything else in the C `.so` is `U` — libc imports:
`malloc`, `free`, `strdup`, `strtok_r`, `strcat`, `snprintf`, `atoi`,
`perror`, `fprintf`, `fopen`, `fclose`, `strerror`, `__errno_location`,
`stderr`.)

## `nm -D --defined-only` — Rust (C-ABI subset)

```
allocate_matrix                 T
driver                          T
free_matrix                     T
initialize_matrix_from_string   T
matrix_to_string                T
multiply_matrices               T
write_to_file                   T
```

The Rust `.so` additionally exports Rust-internal / std symbols
(`_ZN…`, `rust_*`, `__rust_*`) which have no C counterpart and are not part of
the comparison surface.

## Diff

| symbol | in C | in Rust | action |
|---|---|---|---|
| `allocate_matrix` | ✔ | ✔ | — |
| `free_matrix` | ✔ | ✔ | — |
| `initialize_matrix_from_string` | ✔ | ✔ | — |
| `multiply_matrices` | ✔ | ✔ | — |
| `matrix_to_string` | ✔ | ✔ | — |
| `write_to_file` | ✔ | ✔ | — |
| `driver` | ✔ | ✔ | — |

**Missing from Rust: 0. Extra C-ABI symbols in Rust: 0.**

## Undefined (imported) non-libc symbols in the Rust `.so`

`nm -D -u` on the Rust `.so` resolves entirely against `libc`/`libgcc`
(`malloc`, `free`, `strdup`, `strtok_r`, `strcat`, `snprintf`, `atoi`,
`perror`, `fprintf`, `fopen`, `fclose`, `strerror`, `__errno_location`,
`stderr`, plus std/unwind runtime imports). **0 missing non-libc symbols.**

Reproduce with:

```sh
nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort > /tmp/c.txt
nm -D --defined-only translation/target/release/libdriver.so \
  | awk '{print $3}' | grep -v '^_ZN\|^rust_\|^__rust' | sort > /tmp/r.txt
comm -23 /tmp/c.txt /tmp/r.txt   # must be empty
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default (empty) feature set. Phase D's "every feature
combination" therefore collapses to a single combination, which is verified by
`--no-default-features` and the default build (both checked in
`check_features.sh`).

## Result

`./check_symbols.sh` → `SYMBOL PARITY: OK (0 missing)`, and `EXTRA C-ABI
symbols in the Rust .so: (none)`. Verified for both the default and the
`--no-default-features` build by `./check_features.sh`.

No C source file was left untranslated: `c_src/CMakeLists.txt` compiles exactly
`src/matrix.c`, `src/write.c`, `src/driver.c`, and each has a corresponding
Rust module (`src/matrix.rs`, `src/write.rs`, `src/driver.rs`) with a
`#[no_mangle] extern "C"` wrapper per non-`static` C function. No symbol is
stubbed or `unimplemented!()`.
