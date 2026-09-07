# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C:    `c_src/build/libpow.so`      (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust: `translation/target/release/libpow.so`

## Defined (exported) symbols in the C `.so`

`nm -D c_src/build/libpow.so | grep -E ' [TtWwBbDdRr] '`

| # | symbol | C type | present in Rust `.so`? | notes |
|---|--------|--------|------------------------|-------|
| 1 | `my_pow` | `T` (global text) | YES — `T my_pow` | the only real public symbol; declared in `c_src/include/pow.h` |
| 2 | `_ITM_deregisterTMCloneTable` | `w` (weak undefined) | YES (`w`) | toolchain-generated crt stub, not library API |
| 3 | `_ITM_registerTMCloneTable` | `w` | YES (`w`) | toolchain-generated |
| 4 | `__cxa_finalize@GLIBC_2.2.5` | `w` | YES (`w`) | toolchain-generated |
| 5 | `__gmon_start__` | `w` | YES (`w`) | toolchain-generated |

There are no macro-generated exports, no versioned exports, and no exported
data symbols. `c_src/src/pow.c` is the only translation unit and
`c_src/include/pow.h` declares exactly one function.

## Undefined (imported) symbols in the C `.so`

| # | symbol | present as `U` in Rust `.so`? |
|---|--------|-------------------------------|
| 1 | `__errno_location@GLIBC_2.2.5` | YES |
| 2 | `fprintf@GLIBC_2.2.5` | YES |
| 3 | `pow@GLIBC_2.29` | YES |
| 4 | `stderr@GLIBC_2.2.5` | YES |

The Rust `.so` imports additional libc symbols (`malloc`, `memcpy`,
`_Unwind_*`, `dl_iterate_phdr`, …). These come from the Rust runtime /
panic-unwind + backtrace machinery, are all resolved by libc/libgcc at load
time, and are not part of the library's API surface. They are permitted extra
imports, not missing exports.

## Result

Missing-from-Rust exported symbols: **0**
Missing-from-Rust non-libc undefined symbols: **0**

Verification command (must print nothing):

```sh
diff <(nm -D c_src/build/libpow.so           | awk '$2 ~ /^[TtWwBbDdRr]$/ {print $3}' | sort) \
     <(nm -D translation/target/release/libpow.so | awk '$2 ~ /^[TtWwBbDdRr]$/ {print $3}' | sort)
```

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
build configuration is the default one. The tip about looping over feature
combinations degenerates to a single combination here; this is confirmed by
`grep -n '\[features\]' Cargo.toml` returning nothing.

## Verified

```
$ comm -23 <(nm -D c_src/build/libpow.so           | awk '$0 !~ / U /{print $NF}' | sort -u) \
           <(nm -D translation/target/release/libpow.so | awk '$0 !~ / U /{print $NF}' | sort -u)
                                    # (no output)
$ ldd -r translation/target/release/libpow.so | grep -i 'undefined\|not found'
                                    # (no output — every import resolves)
```

* Exported symbols missing from the Rust `.so`: **0**
* Unresolved (non-libc or otherwise) symbols in the Rust `.so`: **0**
* Nothing was stubbed: `grep -rn 'unimplemented!\|todo!\|unreachable!' src/` finds nothing.
  `c_src/src/pow.c` is the only C translation unit and its single function is
  fully translated in `src/pow.rs`, so no module was skipped.

This is re-checked mechanically on every run by `verify.sh` (for each profile
and feature combination) and from inside the test suite by
`tests/phase_d_sweeps.rs::d8_symbol_parity`, so the parity claim cannot silently
rot.

Note that the whole test suite is *itself* additional evidence of export
correctness: every call in every test goes through `dlopen` + `dlsym("my_pow")`
on both `.so` files, so the `#[unsafe(no_mangle)] extern "C"` wrapper is what is
being exercised — the Rust function is never called directly.
