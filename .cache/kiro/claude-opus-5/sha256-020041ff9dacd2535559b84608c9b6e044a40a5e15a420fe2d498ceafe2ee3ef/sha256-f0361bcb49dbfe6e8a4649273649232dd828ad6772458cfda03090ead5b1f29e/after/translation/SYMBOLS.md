# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

C `.so`:    `c_src/build/libharvest-work-7Ma4DJ.so`
Rust `.so`: `translation/target/release/libhsv_to_rgb_lib.so`

## C exported (defined) dynamic symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-7Ma4DJ.so
0000000000001109 T hsv_to_rgb
```

That is the complete list. The C translation unit is a single file
(`c_src/src/lib.c`, 59 lines) declaring a single public prototype in
`c_src/include/lib.h`:

```c
void hsv_to_rgb(float *dest, const float *src);
```

There are no macro-generated symbols, no versioned aliases, no `static`
functions promoted to external linkage, and no additional C source files in
`CMakeLists.txt` (`add_library(... SHARED src/lib.c)` only).

## Symbol parity table

| # | C symbol | type | exported by Rust `.so`? | Rust item |
|---|----------|------|-------------------------|-----------|
| 1 | `hsv_to_rgb` | `T` (global text) | YES — `T hsv_to_rgb` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn hsv_to_rgb` in `src/lib.rs` |

## Missing-symbol analysis

**None.** The C→Rust symbol diff is empty; no exports needed to be added and no
C module was left untranslated. `c_src/src/lib.c` is fully translated in
`translation/src/lib.rs`.

```
$ comm -3 <(nm -D --defined-only <c.so>  | awk '{print $NF}' | sort) \
          <(nm -D --defined-only <rust.so> | awk '{print $NF}' | sort) \
  | grep -v '^\s*_\?_\?rust\|^\s*_ITM_\|^\s*__cxa\|^\s*__gmon'
  <empty>
```

## Undefined (imported) symbols

C imports `floorf@GLIBC_2.2.5` plus the usual weak CRT hooks
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`). The Rust `.so` resolves `floorf`
internally via `f32::floor` (an LLVM intrinsic lowering to
`roundss $0x9` / `frintm`), so it has no extra non-libc undefined symbols.
0 missing / 0 undefined non-libc symbols on the Rust side.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configuration is the default one. `--no-default-features` and
`--all-features` are equivalent to the default here (verified in Phase D).
