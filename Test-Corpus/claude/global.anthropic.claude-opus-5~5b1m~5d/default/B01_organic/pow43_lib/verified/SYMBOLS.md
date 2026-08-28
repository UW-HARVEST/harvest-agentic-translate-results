# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-NqWXYC.so   (name comes from the parent dir name,
#                                             see CMakeLists.txt cmake_path GET FILENAME)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libpow43_lib.so   (crate-type = ["cdylib"])
```

## C source inventory (completeness check)

The whole library is a single translation unit. `CMakeLists.txt` lists exactly
one source file, so there is no module that could have been skipped:

| C file | contents | translated in Rust? |
|--------|----------|---------------------|
| `c_src/src/lib.c` | `static const float g_pow43[129 + 16]`, `float pow43(int)` | yes — `translation/src/lib.rs` (`g_pow43`, `pow43`) |
| `c_src/include/lib.h` | `float pow43(int x);` (1 line, the only public declaration) | yes |

There are no other `.c` files, no `#ifdef`-selected alternative
implementations, and no name-mangling / namespacing macros in the public
header, so the exported linker symbol is plainly `pow43`.

## Defined dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `pow43` | `T` (text, global) | `T` (text, global) | `#[unsafe(no_mangle)] pub extern "C" fn pow43(x: c_int) -> f32` |

`g_pow43` is `static` in C, therefore *not* a dynamic symbol; the Rust `static`
is likewise private. Correctly absent from both.

### Symbol diff

```
$ comm -3 <(nm -D --defined-only c_src/build/libharvest-work-NqWXYC.so   | awk '{print $NF}' | sort) \
          <(nm -D --defined-only translation/target/release/libpow43_lib.so | awk '{print $NF}' | sort)
<empty>
```

**0 symbols missing from the Rust `.so`.** No `#[no_mangle]` wrapper had to be
added and no C module had to be translated after the fact; nothing is stubbed
or `unimplemented!()`.

## Undefined symbols (imports)

| `.so` | undefined non-libc symbols |
|-------|----------------------------|
| C | none (only weak CRT hooks: `_ITM_*`, `__cxa_finalize`, `__gmon_start__`) |
| Rust | none — every `U` entry is glibc (`malloc`, `memcpy`, `open64`, `pthread_key_create`, …) or the platform unwinder (`_Unwind_*`), pulled in by `libstd`; all are satisfied by `libc`/`libgcc_s` at load time |

`dlopen(RTLD_NOW)` of both objects succeeds, which confirms every import
resolves.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration that exists is the default one. `scripts/check_all_features.sh`
enumerates the feature powerset from `Cargo.toml` and runs
`cargo check`/`cargo test` for each; with no features declared it degenerates
to the four canonical variants (default, `--no-default-features`,
`--all-features`, and `--no-default-features --all-features`), all of which
are exercised.
