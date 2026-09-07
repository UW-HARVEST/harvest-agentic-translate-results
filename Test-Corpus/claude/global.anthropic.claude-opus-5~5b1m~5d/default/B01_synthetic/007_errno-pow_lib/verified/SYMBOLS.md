# SYMBOLS.md — Phase A: Public symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libpow.so
nm -D --defined-only translation/target/release/libpow.so
```

## C `.so` exported (defined, dynamic) symbols

| # | symbol | type | exported by Rust `.so`? |
|---|--------|------|-------------------------|
| 1 | `my_pow` | `T` (text/global) | YES — `#[unsafe(no_mangle)] pub extern "C" fn my_pow` |

The entire public surface of `c_src` is the single function declared in
`c_src/include/pow.h`:

```c
double my_pow(double base, double exponent);
```

There are no other translation units in `c_src/CMakeLists.txt` (`src/pow.c`
only), no macro-generated symbols, no exported globals, and no exported data.

## Missing symbols

**NONE.** The symbol diff (C defined-set minus Rust defined-set) is EMPTY.

## Undefined (imported) symbols

The C `.so` imports: `__errno_location`, `fprintf`, `pow`, `stderr`
(plus the standard weak CRT symbols `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the SAME four functional symbols —
`__errno_location@GLIBC_2.2.5`, `fprintf@GLIBC_2.2.5`, `pow@GLIBC_2.29`,
`stderr@GLIBC_2.2.5` — which is required for behavioural identity: the Rust
translation deliberately calls glibc's `pow` (not Rust's `f64::powf`) and reads
the same thread-local `errno` slot, so the `errno` side-effects that drive the
error branches are bit-identical.

All remaining undefined symbols in the Rust `.so` are libc / libgcc-unwind /
Rust-std runtime imports (`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`,
`pthread_key_*`, …). There are **0 missing or undefined non-libc symbols**.

### Checklist

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name.
- [x] No stubs / `unimplemented!()` / faked exports.
- [x] Symbol diff reaches empty.
- [x] 0 missing/undefined non-libc symbols in the Rust `.so`.
