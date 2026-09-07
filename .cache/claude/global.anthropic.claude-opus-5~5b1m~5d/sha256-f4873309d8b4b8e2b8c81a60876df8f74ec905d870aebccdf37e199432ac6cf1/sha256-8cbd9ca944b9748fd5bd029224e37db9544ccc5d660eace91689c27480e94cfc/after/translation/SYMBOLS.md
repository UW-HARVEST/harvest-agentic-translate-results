# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

* C  `.so`: `c_src/build/libdriver.so`
* Rust `.so`: `translation/target/release/libdriver.so`

## C source inventory (completeness check)

`c_src` contains exactly two source files, both fully translated:

| C file | translated to | status |
|--------|---------------|--------|
| `c_src/include/driver.h` | `translation/src/lib.rs` (`driver` decl) | complete |
| `c_src/src/driver.c`     | `translation/src/lib.rs`                | complete |

Every function defined in `driver.c` has a Rust counterpart:

| C function | linkage in C | Rust counterpart | exported from Rust `.so` |
|------------|--------------|------------------|--------------------------|
| `printLine`    | external | `printLine`    | yes (`#[unsafe(no_mangle)] extern "C"`) |
| `printIntLine` | external | `printIntLine` | yes (`#[unsafe(no_mangle)] extern "C"`) |
| `bad`          | external | `bad`          | yes (`#[unsafe(no_mangle)] extern "C"`) |
| `goodG2B`      | `static` (internal) | `goodG2B` (private `fn`) | no — matches C, `static` is not exported |
| `goodB2G`      | `static` (internal) | `goodB2G` (private `fn`) | no — matches C, `static` is not exported |
| `good`         | external | `good`         | yes (`#[unsafe(no_mangle)] extern "C"`) |
| `driver`       | external | `driver`       | yes (`#[unsafe(no_mangle)] extern "C"`) |

No C module was skipped; there is nothing left to translate.

## Dynamic symbol table (`nm -D --defined-only`)

| # | symbol | C `.so` | Rust `.so` | note |
|---|--------|---------|-----------|------|
| 1 | `bad`          | `T` | `T` | `void bad(float)` |
| 2 | `driver`       | `T` | `T` | `void driver(float, float)` |
| 3 | `good`         | `T` | `T` | `void good(float)` |
| 4 | `printIntLine` | `T` | `T` | `void printIntLine(int)` |
| 5 | `printLine`    | `T` | `T` | `void printLine(const char *)` |

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(no output)
```

**Missing from Rust `.so`: 0.** No stubs, no `unimplemented!()`.

## Undefined (imported) symbols

The Rust `.so` imports only libc symbols, exactly like the C `.so`:

| symbol | present in C `.so` imports | present in Rust `.so` imports |
|--------|----------------------------|-------------------------------|
| `printf`      | yes | yes |
| `fabs`        | yes (may be inlined by the compiler) | not needed — `f64::abs` is a bit-mask intrinsic, semantically identical |
| `__cxa_finalize` / `_ITM_*` / `__gmon_start__` (weak) | yes | n/a |

0 missing / unresolved non-libc symbols in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table at all**, therefore the
only build configuration is the default one:

```
$ cargo test                                  # default (== all features == no features)
$ cargo test --no-default-features            # identical build, no features exist
```

Both were run; there is no further cross-product to enumerate.
