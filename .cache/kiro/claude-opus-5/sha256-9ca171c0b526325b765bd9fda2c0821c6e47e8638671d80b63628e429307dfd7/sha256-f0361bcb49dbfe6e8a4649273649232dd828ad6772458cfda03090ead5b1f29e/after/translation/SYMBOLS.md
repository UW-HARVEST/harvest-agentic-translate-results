# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory

The whole library is one translation unit; there are no untranslated modules.

| C file | public declarations | translated in Rust |
|---|---|---|
| `c_src/include/driver.h` | `void driver(int x, int y);` | yes |
| `c_src/src/driver.c` | definition of `driver` (no other functions, no file-static helpers, no globals) | yes — `translation/src/lib.rs` |

## `nm -D --defined-only` — C `.so`

```
0000000000001109 T driver
```

Non-libc defined symbols: **1** (`driver`).

## `nm -D --defined-only` — Rust `.so`

```
0000000000011700 T driver
```

Non-libc defined symbols: **1** (`driver`).

## Symbol diff

| symbol | C `.so` | Rust `.so` | status |
|--------|---------|------------|--------|
| `driver` | `T` (global text) | `T` (global text) | present in both, exact name |

**Missing from Rust `.so`: none. Extra in Rust `.so`: none.**

`driver` is exported from Rust via `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_int, y: c_int)`.
No macro-generated symbol names exist in the C source (no function-generating
macros; the only preprocessor directives are the `DRIVER_H_` include guard and
`#include`s).

## Undefined-symbol audit of the Rust `.so`

`nm -D -u translation/target/release/libdriver.so` reports only glibc
(`GLIBC_*`) and unwinder (`_Unwind_*`, `GCC_*`) imports plus weak
`__gmon_start__` / `_ITM_*` / `statx` / `gettid` stubs. There are **0 missing or
undefined non-libc symbols**.

Note: the Rust `.so` imports `puts` rather than `printf`. This is LLVM's
standard `printf("literal\n")` → `puts("literal")` simplification of the
translated `printf` calls. Both write the same bytes through the same glibc
`stdout` `FILE` stream, so observable output is unchanged; this is verified
byte-for-byte in Phase B rather than assumed.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. `--no-default-features` is therefore
equivalent to the default build. Verified mechanically:

```
$ grep -n '\[features\]' translation/Cargo.toml   # -> no match
```

## Completion

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

## Verified in both build profiles

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(empty)
```

`target/debug/libdriver.so` likewise exports `T driver` and nothing else
non-libc (see `run_all_configs.sh` output).
