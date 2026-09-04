# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects:

```sh
nm -D --defined-only c_src/build/libdriver.so           | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort
```

## C `.so` public (dynamic, defined) symbols

There are exactly 7. `allocate_matrix` is absent from `include/matrix.h` but is
non-`static` in `src/matrix.c`, so it is part of the exported ABI.

| # | symbol | C source | Rust source | exported by Rust `.so` |
|---|--------|----------|-------------|------------------------|
| 1 | `allocate_matrix` | `src/matrix.c:33` | `src/matrix.rs` `#[unsafe(no_mangle)]` | yes |
| 2 | `free_matrix` | `src/matrix.c:66` | `src/matrix.rs` `#[unsafe(no_mangle)]` | yes |
| 3 | `initialize_matrix_from_string` | `src/matrix.c:78` | `src/matrix.rs` `#[unsafe(no_mangle)]` | yes |
| 4 | `multiply_matrices` | `src/matrix.c:118` | `src/matrix.rs` `#[unsafe(no_mangle)]` | yes |
| 5 | `matrix_to_string` | `src/matrix.c:137` | `src/matrix.rs` `#[unsafe(no_mangle)]` | yes |
| 6 | `write_to_file` | `src/write.c:32` | `src/write.rs` `#[unsafe(no_mangle)]` | yes |
| 7 | `driver` | `src/driver.c:35` | `src/driver.rs` `#[unsafe(no_mangle)]` | yes |

No macro-generated symbols exist in this library (no symbol-emitting macros in
the C sources).

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(empty)
```

**Missing symbols: 0.** No C translation unit was skipped: `matrix.c`,
`write.c` and `driver.c` map to `src/matrix.rs`, `src/write.rs`,
`src/driver.rs`. No stubs, no `unimplemented!()`.

## Undefined-symbol check on the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only

* glibc imports (`malloc`, `free`, `strdup`, `strtok_r`, `atoi`, `perror`,
  `fprintf`, `snprintf`, `fopen`, `fclose`, `strerror`, `stderr`,
  `__errno_location`, …) — the same set the C `.so` imports, plus
* Rust `std`/`libgcc` runtime imports (`_Unwind_*`, `pthread_key_*`,
  `dl_iterate_phdr`, `mmap64`, …) and weak symbols.

**0 missing/undefined non-libc symbols.**

Notable: the C `.so` imports `strcat@GLIBC` while the Rust `.so` does not — the
translation open-codes `strcat` in `src/cstd.rs`. This is an
implementation-detail import, not a public symbol, and the byte-level behaviour
is identical (append at first NUL, copy through and including the NUL).

## Exported data layout

`matrix_t` is `#[repr(C)] { *mut *mut c_int, c_int, c_int }` = 16 bytes,
offsets 0 / 8 / 12 — identical to the C `struct`. Verified at runtime by the
differential tests, which read matrices produced by one library with the layout
assumed by the other.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are therefore the same build; both are exercised.
