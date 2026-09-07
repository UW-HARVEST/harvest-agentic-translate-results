# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

* C `.so`:    `c_src/build/libdriver.so`
* Rust `.so`: `translation/target/release/libdriver.so`

## Complete public (exported / defined dynamic) symbol list of the C `.so`

`nm -D --defined-only c_src/build/libdriver.so`

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `driver` | `T` (global text) | YES — `T driver` |

That is the *entire* exported surface. `c_src/src/driver.c` is the only
translation unit in `c_src/CMakeLists.txt` (`add_library(driver SHARED src/driver.c)`),
it defines exactly one non-static function, and `c_src/include/driver.h`
declares exactly one prototype (`void driver(int x);`). There are no
macro-generated symbols, no exported data objects, and no additional C source
files, so nothing was skipped by the translation.

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort) \
         <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
```

Result: **empty** — 0 symbols missing from the Rust `.so`.

No `#[no_mangle]` wrapper had to be added and no C module had to be translated:
the single symbol `driver` was already implemented and exported (via
`#[unsafe(no_mangle)] pub extern "C" fn driver`).

## Undefined (imported) symbols

Both objects import `printf@GLIBC_2.2.5` plus the usual weak
`_ITM_*` / `__gmon_start__` / `__cxa_finalize` entries. The Rust `.so`
additionally imports only libc and `libgcc`/unwind runtime symbols pulled in by
the Rust standard library (`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`,
…). There are **0 missing/undefined non-libc symbols** in the Rust `.so`:
every non-libc/non-unwind undefined entry is absent, i.e. nothing references a
Rust-side function that was never translated.

## Extra symbols exported by Rust but not C

None (`comm -13` on the same lists is also empty). The Rust `.so` exports
exactly `driver`, matching the C `.so` one-for-one.

## Verification result (Phase D)

`tests/symbols.rs` re-runs this comparison as an executable check:

| test | asserts |
|------|---------|
| `every_c_symbol_is_exported_by_rust` | `nm -D --defined-only` set difference C → Rust is empty |
| `c_and_rust_symbols_are_distinct_implementations` | the two `dlsym`-resolved `driver` addresses differ, so the differential tests are not comparing one library with itself |
| `rust_symbol_is_callable_and_matches_c_signature_shape` | `driver(7)` produces `314\n` from both `.so`s |
| `rust_so_has_no_unresolved_non_libc_symbols` | every `nm -D -u` entry belongs to libc / the unwinder / the loader — nothing untranslated is referenced |

`test result: ok. 4 passed; 0 failed`, under the default feature set and under
`--no-default-features`, in both the `dev` and `release` profiles.

Nothing was stubbed and no `unimplemented!()` exists anywhere in `src/`: the C
library's whole implementation is 4 statements in one function, all of which are
present in `src/lib.rs`.
