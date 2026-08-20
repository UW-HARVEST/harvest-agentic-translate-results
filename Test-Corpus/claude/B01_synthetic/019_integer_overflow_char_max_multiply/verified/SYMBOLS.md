# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on the C shared object and the Rust `cdylib`.

## Build commands

```sh
# C: executable (as CMakeLists.txt declares) + shared object for symbol diffing
cd translated_rust/c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .      # -> ./driver
gcc -shared -fPIC -o libdriver_c.so ../src/main.c                     # -> ./libdriver_c.so

# Rust: binary + cdylib from one shared implementation (src/imp.rs)
cd translated_rust && cargo build --release
#   -> target/release/driver
#   -> target/release/libdriver.so
```

`c_src/CMakeLists.txt` declares no options, no `option()`/`add_definitions()`,
and no `#ifdef` exists anywhere in `c_src/src/main.c`. `Cargo.toml` declares no
`[features]`. **There is therefore exactly one build configuration** (see
`CONFIGS.md` §0).

## Exported (dynamic, defined) symbols

`nm -D --defined-only` on each `.so`:

| # | symbol | C signature | C `.so` | Rust `.so` | notes |
|---|--------|-------------|---------|------------|-------|
| 1 | `printLine`        | `void printLine(const char *line)` | T | T | `src/lib.rs` `#[no_mangle] unsafe extern "C"` |
| 2 | `printHexCharLine` | `void printHexCharLine(char charHex)` | T | T | `src/lib.rs` `#[no_mangle] extern "C"` |
| 3 | `bad`              | `void bad(void)` | T | T | `src/lib.rs` `#[no_mangle] extern "C"` |
| 4 | `good`             | `void good(void)` | T | T | `src/lib.rs` `#[no_mangle] extern "C"` |
| 5 | `main`             | `int main(void)` | T | T | `src/lib.rs` `#[no_mangle] extern "C"` |

### Deliberately NOT exported

`goodG2B` and `goodB2G` are `static` in `main.c`, so they are absent from the C
`.so`'s dynamic symbol table. They are private `fn`s in `src/imp.rs` and are
likewise absent from the Rust `.so`. They are still fully covered because
`good()` is their only caller in C and in Rust.

There are no macro-generated symbols in this translation unit (no function-like
macros expand to definitions; `CHAR_MAX` is an object-like constant).

## Symbol diff result

```sh
comm -23 <(nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort) \
         <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort)
```

Output: **empty** — 0 symbols missing from the Rust `.so`.

The Rust `.so` exports exactly these 5 symbols and nothing else (std internals
are hidden), so the surface matches in both directions.

## Undefined symbols

* C `.so` undefined: `__isoc99_scanf`, `printf`, `puts` (GCC lowers
  `printf("%s\n", line)` to `puts(line)`), plus the usual weak
  `_ITM_*` / `__cxa_finalize` / `__gmon_start__`.
* Rust `.so` undefined: only libc (`malloc`, `write`, `writev`, `read`, `memcpy`,
  `__errno_location`, `pthread_*`, `mmap64`, …) and libgcc unwinder
  (`_Unwind_*`) symbols.

**0 missing/undefined non-libc symbols in the Rust `.so`.** ✅
