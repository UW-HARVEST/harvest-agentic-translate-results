# SYMBOLS.md — exported-symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
(`c_src/build/libharvest-work-f6KvIi.so`) versus the Rust cdylib
(`translation/target/release/libhsl_to_rgb_lib.so`).

## C `.so` exported (non-libc, defined) symbols

| # | symbol | type | declared in | defined in | exported by Rust `.so`? |
|---|--------|------|-------------|------------|--------------------------|
| 1 | `hsl_to_rgb` | `T` (global text) | `c_src/include/lib.h:1` | `c_src/src/lib.c:5` | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn hsl_to_rgb` (`translation/src/lib.rs:104`) |

## Rust `.so` extra symbols

None. `nm -D --defined-only` on the Rust cdylib prints exactly `hsl_to_rgb`
(plus the usual dynamic-loader/libc-supplied undefined entries, which are
filtered out).

## Undefined (imported) symbols

The C object imports `fmodf` from libm (`fabsf` is inlined by the compiler to an
`andps`). The Rust cdylib imports `fmodf` from the system libm as well
(`unsafe extern "C" { safe fn fmodf(..) }`), so the dynamic-symbol requirement
set is the same. No non-libc symbol is left undefined in the Rust `.so`.

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/*.so        | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/libhsl_to_rgb_lib.so \
                                                    | awk '{print $NF}' | sort)
```

Result: **empty diff — 0 missing symbols, 0 extra symbols.**
No C source file was left untranslated: `c_src/src/lib.c` is the only
translation unit in `c_src/CMakeLists.txt`, and its single function is
translated in `translation/src/lib.rs`.
