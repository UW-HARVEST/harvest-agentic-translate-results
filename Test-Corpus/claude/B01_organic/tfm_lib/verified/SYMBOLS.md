# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libtranslated_rust.so`
  (built by `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/{debug,release}/libtfm_lib.so`
  (`[lib] crate-type = ["cdylib"]`, `name = "tfm_lib"`)

Reproduce with:

```sh
./verify.sh            # builds both, diffs symbols, runs every feature combo
```

or by hand:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > c.txt
nm -D --defined-only target/debug/libtfm_lib.so        | awk '{print $3}' | sort > r.txt
comm -23 c.txt r.txt   # MUST be empty
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | C declaration | notes |
|---|--------|---------|------------|---------------|-------|
| 1 | `tfm` | `T` (defined, global) | `T` (defined, global) | `void tfm(float *dest, const float *src, int count);` | `include/lib.h`; the entire public API of the library |

`comm -23 c.txt r.txt` → **empty**. 0 symbols missing from the Rust `.so`.

The C translation unit is a single file (`c_src/src/lib.c`, 32 lines) containing
exactly one function; `c_src/include/lib.h` declares exactly one prototype.
There is no macro-generated symbol, no `static` helper promoted to a global, no
exported data object, and no second translation unit — `CMakeLists.txt` lists
only `src/lib.c`. So no C source was left untranslated and no export wrapper is
missing.

## Undefined (imported) symbols

The C `.so` imports one non-weak, non-libc-startup symbol:

| symbol | C `.so` | Rust `.so` | how the Rust side satisfies it |
|--------|---------|------------|--------------------------------|
| `sqrtf@GLIBC_2.2.5` | `U` | not imported | implemented inline by `fsqrt()`, which models `sqrtss` + glibc's negative-operand path on raw bits (`f32::sqrt` lowers to `sqrtss`) |

Both objects additionally reference only weak toolchain/startup symbols
(`_ITM_*`, `__gmon_start__`, `__cxa_finalize`). The Rust `.so` imports the
usual `libstd` set (`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`, …); all
of these are libc / libgcc runtime symbols, not library symbols the C `.so`
was expected to provide.

**Non-libc undefined symbols in the Rust `.so`: 0.**

## Feature combinations

`Cargo.toml` declares `[features] default = []` and no other feature; there are
no `#[cfg(feature = ...)]` sites in `src/`. `c_src/CMakeLists.txt` defines no
`option()`, no `target_compile_definitions`, and no `#ifdef` build switch, and
`c_src/src/lib.c` contains no `#if`/`#ifdef`. The complete set of valid
build-time configurations is therefore the single empty combination:

| # | cargo invocation | C counterpart |
|---|------------------|---------------|
| 1 | `cargo check/test --no-default-features` | default CMake build (no `CMAKE_BUILD_TYPE`, no extra flags) |
| 2 | `cargo check/test` (i.e. `--features default`, which is empty) | identical to #1 |

Both are verified by `verify.sh`.
