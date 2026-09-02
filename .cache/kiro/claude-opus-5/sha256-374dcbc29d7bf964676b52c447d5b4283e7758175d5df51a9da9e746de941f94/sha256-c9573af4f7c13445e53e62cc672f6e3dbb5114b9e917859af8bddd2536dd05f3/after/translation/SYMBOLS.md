# SYMBOLS.md — Public symbol surface (Phase A / Phase D)

Derived mechanically from:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release

nm -D --defined-only c_src/build/libStaticLoop.so
nm -D --defined-only translation/target/release/libStaticLoop.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` builds exactly one translation unit:

| C source file | translated to | status |
|---|---|---|
| `c_src/src/staticloop.c` | `translation/src/lib.rs` | translated (both functions) |

`c_src/include/staticloop.h` declares exactly two entry points
(`int static_sum(int update);`, `void driver(int update);`). There is no second
module, no macro-generated symbol set, and no exported global data. Nothing was
skipped, so no additional C had to be translated for Phase A/D.

## Defined (exported) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|:-------:|:----------:|-------|
| 1 | `static_sum` | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn static_sum` |
| 2 | `driver`     | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

Symbol diff (C defined − Rust defined): **EMPTY**.

Note: the C `static int sum` inside `static_sum` has *no* linkage (function-local
static), so it is intentionally not an exported symbol on either side. The Rust
`static mut SUM` is likewise not exported (`nm -D` shows no `SUM`), which matches.

## Undefined (imported) dynamic symbols

| `.so` | non-libc / non-runtime undefined symbols |
|---|---|
| C | none (`printf@GLIBC_2.2.5` + `_ITM_*`/`__gmon_start__`/`__cxa_finalize` weak stubs only) |
| Rust | none (`printf@GLIBC_2.2.5`, other glibc symbols, and `_Unwind_*` from the Rust runtime only) |

The Rust `.so` imports a larger set of glibc symbols (`malloc`, `memcpy`,
`mmap64`, `pthread_key_create`, `_Unwind_*`, …) purely because the Rust standard
library / panic-unwind runtime is statically linked into the cdylib. All of them
are libc or compiler-runtime symbols; **0 missing/undefined non-libc symbols**.

## Verification result

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with the
      exact same name.
- [x] The Rust `.so` exports no extra public symbols beyond those two.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.

Re-verified by `translation/tests/symbol_parity.rs`, which shells out to `nm -D`
on both `.so` files and asserts the defined-symbol diff is empty.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the only
build configuration is the default one (`--no-default-features` is equivalent).
Phase D's "repeat for every feature combination" reduces to the single default
configuration; this is confirmed mechanically by
`translation/check_all_features.sh`.
