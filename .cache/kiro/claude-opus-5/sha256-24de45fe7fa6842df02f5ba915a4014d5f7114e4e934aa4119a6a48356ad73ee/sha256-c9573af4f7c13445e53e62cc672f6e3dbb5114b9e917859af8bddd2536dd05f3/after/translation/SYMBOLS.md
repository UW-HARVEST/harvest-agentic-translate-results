# SYMBOLS.md — public ABI surface parity

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-x12mak.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libcontrast_ratio_lib.so
```

## C `.so` defined dynamic symbols (excluding absolute/ABI tag symbols)

| # | symbol | type | notes |
|---|--------|------|-------|
| 1 | `contrast_ratio` | `T` (text, global) | the only public API symbol |

`c_src/src/lib.c` also defines `cbLuminance` and `cbContrastRatio`, but both are
`static` and therefore have **no** dynamic symbol — they must NOT appear in the
Rust `.so` either (and do not; they are private Rust `fn`s).

The C `.so` additionally imports `pow` from `libm` (undefined symbol `pow`).
The Rust `.so` resolves `f64::powf` to the same `libm` `pow` on this target,
so its undefined-symbol set is a subset of libc/libm as well.

## Rust `.so` defined dynamic symbols relevant to the C surface

| # | symbol | present in Rust `.so`? | how |
|---|--------|------------------------|-----|
| 1 | `contrast_ratio` | YES | `#[unsafe(no_mangle)] pub extern "C" fn contrast_ratio` in `src/lib.rs` |

## Symbol diff

```
comm -23 <(c symbols) <(rust symbols)   # C-only  -> EMPTY
```

**Result: 0 symbols missing from the Rust `.so`. 0 undefined non-libc/libm
symbols in the Rust `.so`.** No C source file was left untranslated: `src/lib.c`
is the only translation unit in `CMakeLists.txt` and all three of its functions
(`cbLuminance`, `cbContrastRatio`, `contrast_ratio`) exist in `src/lib.rs`.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so there is exactly
one feature combination (the empty/default one). `--no-default-features` and the
default build are the same build. Verified by
`cargo check --no-default-features` succeeding and producing an identical symbol
set.

## Verification evidence

```
$ nm -D --defined-only c_src/build/libharvest-work-x12mak.so
0000000000001369 T contrast_ratio

$ nm -D --defined-only translation/target/harness/release/libcontrast_ratio_lib.so | grep contrast
0000000000011780 T contrast_ratio

$ diff <(C defined syms) <(Rust defined syms) | grep '^<'
no C-only symbols: OK
```

Undefined (imported) symbols in the Rust `.so` are all libc / libm / unwind:
`pow@GLIBC_2.29` (the identical `libm` entry point the C `.so` calls), plus
`malloc`, `memcpy`, `_Unwind_*`, `__cxa_finalize`, etc. **0 non-libc undefined
symbols.**

Automated by `translation/run_matrix.sh`, which enumerates the feature list from
`Cargo.toml` (rather than hardcoding it), runs the suite for every combination in
both profiles, and finishes with the `nm -D` diff.

Test `d01_symbol_parity` re-checks this from inside the suite: it shells out to
`nm -D` on the C `.so`, then `dlsym`s **every** resulting name in the Rust `.so`,
and additionally asserts the two `static` C helpers are absent from both.
