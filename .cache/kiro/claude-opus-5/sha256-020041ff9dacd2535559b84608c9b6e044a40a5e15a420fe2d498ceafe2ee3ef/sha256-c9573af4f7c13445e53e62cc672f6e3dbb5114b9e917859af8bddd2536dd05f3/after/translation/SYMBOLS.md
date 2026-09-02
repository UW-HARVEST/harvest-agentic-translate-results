# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

## Commands used

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-uyduuJ.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libhsv_to_rgb_lib.so
```

## C source inventory

`c_src/CMakeLists.txt` compiles exactly one translation unit into the shared
library:

* `src/lib.c`  (links against `m` for `floorf`)

`c_src/include/lib.h` declares exactly one prototype:

```c
void hsv_to_rgb(float *dest, const float *src);
```

There are no additional C source files, so there is no un-translated module.

## Symbol table

| # | C symbol (`nm -D`) | type | exported by Rust `.so` | notes |
|---|--------------------|------|------------------------|-------|
| 1 | `hsv_to_rgb`       | `T`  | YES (`T hsv_to_rgb`)   | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

No macro-generated symbols exist in the C source (no function-defining macros
are used at all).

## Diff

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-uyduuJ.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libhsv_to_rgb_lib.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust: 0.  Extra in Rust: 0.  Symbol diff is EMPTY.**

## Undefined (imported) symbols

| library | undefined non-libc symbols |
|---------|----------------------------|
| C `.so` | none (`floorf` is libm) |
| Rust `.so` | none (only libc/`ld-linux` interfaces from the Rust runtime) |

**Result: `nm -D` shows 0 missing and 0 undefined non-libc symbols in Rust. ✅**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configuration is the default one. `--no-default-features` and the
default build are identical. Verified by an automated loop
(`scripts/check_all_features.sh`).
