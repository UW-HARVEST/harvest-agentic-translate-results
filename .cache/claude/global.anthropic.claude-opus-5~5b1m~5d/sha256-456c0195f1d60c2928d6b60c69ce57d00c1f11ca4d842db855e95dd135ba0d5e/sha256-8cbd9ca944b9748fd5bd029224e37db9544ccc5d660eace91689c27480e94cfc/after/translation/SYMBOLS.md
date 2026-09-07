# SYMBOLS.md — exported symbol parity (Phase A / Phase D)

Source of truth: `nm -D --defined-only c_src/build/libharvest-work-0mZzK9.so`

The C library is a single translation unit (`c_src/src/lib.c`) with a single
public header (`c_src/include/lib.h`). `cbLuminance` and `cbContrastRatio` are
`static`, so they are **not** part of the exported ABI (confirmed: they do not
appear in `nm -D`, and they are absent from `nm --defined-only` as local `t`
symbols only). There are no macro-generated symbols, no global data, and no
`#ifdef`-gated alternative entry points.

## C `.so` exported symbols (non-libc, non-toolchain)

| # | symbol | type | signature (from `include/lib.h`) | present in Rust `.so`? |
|---|--------|------|----------------------------------|------------------------|
| 1 | `contrast_ratio` | `T` (text, global) | `float contrast_ratio(cb_rgb_255 A, cb_rgb_255 B)` | ✅ yes (`#[unsafe(no_mangle)] pub extern "C"`) |

Toolchain-generated symbols present in either object (`_init`, `_fini`,
`__bss_start`, `_edata`, `_end`, `_ITM_*`, `__cxa_finalize`,
`__gmon_start__`, `rust_eh_personality`) are not part of the library API and
are excluded from the diff by the parity test.

## C static (internal, NOT exported — must NOT be exported by Rust either)

| symbol | why not exported |
|--------|------------------|
| `cbLuminance` | `static` in `lib.c` |
| `cbContrastRatio` | `static` in `lib.c` |

## Undefined (imported) symbols

| library | undefined non-libc symbols |
|---------|----------------------------|
| C `.so` | `pow` (libm) — that is libc/libm, allowed |
| Rust `.so` | `pow` (libm) — the Rust translation binds the *platform* `pow` via `extern "C"`, so both libraries call the identical implementation |

## Result

Symbol diff (C exports − Rust exports) = **∅**. 0 missing symbols,
0 undefined non-libc symbols. No C source file was left untranslated:
`c_src/src/lib.c` is the only `.c` file and all four of its functions
(`cbLuminance`, `cbContrastRatio`, `contrast_ratio`, plus the inlined sRGB
transfer expression) are translated in `translation/src/lib.rs`.

Verified mechanically by `tests/differential.rs::phase_d_symbol_parity`.
