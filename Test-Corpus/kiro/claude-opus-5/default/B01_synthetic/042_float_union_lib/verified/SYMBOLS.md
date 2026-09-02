# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` lists exactly one translation unit:

```
add_library(driver SHARED
    src/driver.c)
```

`c_src/include/driver.h` declares exactly one public entity:

```c
void driver(double f);
```

`c_src/src/driver.c` defines exactly one function (`driver`) plus one file-local
`typedef union { uint64_t x; double f; } raw_double_t` (a type, not a symbol).
There is no second module, no macro-generated symbol family, no global data.
So the complete expected public surface is the single symbol `driver`.

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `driver` | `T` (global text) | `T` (global text) | `#[unsafe(no_mangle)] pub extern "C" fn driver(f: c_double)` |

Symbol diff (C-defined minus Rust-defined): **empty**.

## Undefined (imported) symbols

The C `.so` imports `printf` plus the usual weak CRT hooks
(`__cxa_finalize`, `__gmon_start__`, `_ITM_*TMCloneTable`).

The Rust `.so` imports a larger but strictly libc/libm/libgcc-only set. The
deliberate ones are:

| import | why the translation needs it |
|--------|------------------------------|
| `fwrite`, `stdout` | write through the *same* glibc `FILE` that C `printf` uses, so buffering and interleaving with other C output match |
| `localeconv` | `%a` and `%.4f` take their radix character from `LC_NUMERIC`, so the locale's `decimal_point` must be read on every call |
| `fegetround` | `__printf_fp` rounds `%.4f` according to the current FP direction, so it has to be read on every call |

The rest (`memcpy`, `malloc`, `free`, `_Unwind_*`, `dl_iterate_phdr`, …) come
from `core`/`std`'s formatting, allocator and panic machinery.

There are **0 missing or undefined non-libc symbols** in the Rust `.so`:
every `U`/`w` entry resolves against `libc.so.6` / `libm.so.6` /
`libgcc_s.so.1`, which are already loaded in any process that loaded the C
`.so`. Verified with `ldd -r` (no `undefined symbol` lines) in
`tests/symbols.rs::phase_d_rust_so_has_no_unresolved_non_libc_imports`.

Verification command used:

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so     | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configurations are the default one and
`--no-default-features` (identical, since there are no default features).
Both are checked/tested by `run_all_features.sh`.
