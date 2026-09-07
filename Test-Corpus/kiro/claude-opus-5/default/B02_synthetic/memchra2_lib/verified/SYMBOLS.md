# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

- C  `.so`: `c_src/build/libharvest-work-iGmUF0.so`
- Rust `.so`: `translation/target/release/libmemchra2_lib.so`

## C exported (dynamic, defined) symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-iGmUF0.so
00000000000013e1 T memchra2
```

## Rust exported (dynamic, defined) symbols, Rust-internal noise filtered

```
$ nm -D --defined-only translation/target/release/libmemchra2_lib.so \
    | grep -v -E '_ZN|rust_|__rust|GCC_except|DW\.ref'
0000000000011db0 T memchra2
```

## Parity table

| # | C symbol | type | present in Rust `.so` | notes |
|---|----------|------|-----------------------|-------|
| 1 | `memchra2` | `T` (global text) | YES — `T memchra2` | `#[unsafe(no_mangle)] pub extern "C" fn memchra2(a,b,c,d) -> c_int` |

**Missing symbols: 0.** The symbol diff is empty in the C → Rust direction.

## Symbols intentionally NOT exported

All other functions in `c_src/src/lib.c` are declared `static` and therefore
have internal linkage; they do not appear in `nm -D` for the C `.so` and must
not be exported by Rust either. For the record, the full static set is:

`memchra`, `process_buffer`, `int_to_float_bits`, `process_strings`,
`safe_sum_array`, `interpret_as_int`, `count_occurrences`, `complex_iteration`

These are private helpers in the Rust translation as well, so parity holds in
both directions.

## Undefined (imported) symbols

The Rust `.so` imports only libc/`ld-linux` symbols (`memcpy`, `__libc_start*`
family, unwinder/allocator shims). No non-libc symbol is left undefined.

```
$ nm -D --undefined-only translation/target/release/libmemchra2_lib.so
```
→ all entries are glibc (`GLIBC_2.*`) or `__cxa_*`/unwind runtime symbols.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. `--no-default-features` and any
`--features <combo>` are therefore equivalent to the default build. This is
verified mechanically in `check_features.sh`.

## Binary executable

Neither build produces an executable: `c_src/CMakeLists.txt` contains only
`add_library(... SHARED src/lib.c)`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]`. The "compare C and Rust binary stdout" gate is
therefore **N/A** for this project.

## Automated verification

`./check_features.sh` re-runs the whole Phase D gate mechanically:

```
### 2. symbol parity (nm -D)
C exports:    1
Rust exports: 1
symbol diff (C -> Rust): EMPTY  [OK]
undefined non-libc symbols in Rust .so:
  (empty above means none)

### 3. feature combinations
declared features: 0 (none)
  default                                    OK
  --no-default-features                      OK

PHASE D: PASS
```

The script extracts feature names from `Cargo.toml` with `awk` and enumerates
all 2ⁿ subsets, so it stays correct if features are added later.
