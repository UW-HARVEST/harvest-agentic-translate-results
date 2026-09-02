# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-a0erkJ.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libgaussian_kernel_lib.so
```

## C source inventory (completeness check)

The whole library is one translation unit; `CMakeLists.txt` compiles exactly
`src/lib.c`, and `include/lib.h` is a single line. There is no second module
that could have been skipped by the translate step.

| C file | lines | functions defined | translated in Rust? |
|--------|-------|-------------------|---------------------|
| `c_src/include/lib.h` | 1 | (declaration only) `gaussian_kernel` | yes |
| `c_src/src/lib.c` | 28 | `gaussian_kernel` | yes — `translation/src/lib.rs` |

No file-static/internal helper functions exist in the C source, so there is no
hidden implementation that needs translating.

## Exported (defined, dynamic) symbols

| # | symbol | in C `.so` | in Rust `.so` | note |
|---|--------|-----------|---------------|------|
| 1 | `gaussian_kernel` | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C"` wrapper in `src/lib.rs` |

**Symbol diff (C defined − Rust defined): EMPTY.**
**Symbol diff (Rust defined − C defined): EMPTY.**

No macro-generated symbols exist in the C source (the only macro-like construct
is the inline `((v) > (0)) ? (v) : (0)` clamp, which generates no symbol).

## Undefined (imported) symbols

C imports exactly one non-libc-startup symbol:

| symbol | C | Rust |
|--------|---|------|
| `expf@GLIBC_2.27` | `U` | `U` |

The Rust `.so` imports the *same* versioned `expf@GLIBC_2.27` from the platform
libm, which is what makes the transcendental results bit-identical rather than
merely close. Its remaining undefined symbols are the Rust standard-library
runtime's own libc/libgcc dependencies (`malloc`, `memcpy`, `_Unwind_*`,
`dl_iterate_phdr`, …). **0 missing/undefined non-libc symbols.**

Verified by `translation/tests/symbols.rs::c_exports_are_a_subset_of_rust_exports`,
which shells out to `nm -D` at test time and asserts the diff is empty.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `cargo check`/`cargo test` were nevertheless
run with `--no-default-features` and with `--all-features` to confirm both are
identical to the default build (see `run_all.sh`).
