# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-jXqYKA.so`
- Rust `.so`: `translation/target/release/libsynth_pair_lib.so`

## C exported (defined) dynamic symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-jXqYKA.so
0000000000001160 T synth_pair
```

| # | C symbol | type | exported by Rust `.so`? | notes |
|---|----------|------|-------------------------|-------|
| 1 | `synth_pair` | `T` (global text) | YES — `T synth_pair` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn synth_pair` in `src/lib.rs` |

## Non-exported C symbols (must NOT be required)

| C symbol | linkage | reason not in `nm -D` |
|----------|---------|------------------------|
| `mp3d_scale_pcm` | `static` (file-local, `t`) | `static` in `src/lib.c:3`; never exported by either side. Rust keeps it a private `fn`. Behaviour is verified indirectly through `synth_pair` (it is the sole consumer). |

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-jXqYKA.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libsynth_pair_lib.so \
            | awk '$2 ~ /^[TDBRW]$/ {print $NF}' | sort)
(empty)
```

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C module
was untranslated: `src/lib.c` is the only C translation unit in
`CMakeLists.txt` and both of its functions are present in `src/lib.rs`.

## Undefined symbols

- C `.so` undefined: only the 4 standard weak ELF/glibc hooks
  (`_ITM_*TMCloneTable`, `__cxa_finalize`, `__gmon_start__`).
- Rust `.so` undefined: the same 4 hooks plus **libc / libgcc-unwind only**
  (`malloc`, `free`, `memcpy`, `__errno_location`, `_Unwind_*`, `pthread_key_*`,
  …). These come from the Rust `std` runtime linked into the `cdylib`, not from
  the translated code.

**0 missing/undefined non-libc symbols in the Rust `.so`.** ✔

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, so the only build
configuration is the default one (`cargo test` == `cargo test
--no-default-features`). Phase D's "repeat for every feature combo" therefore
collapses to a single combo, which is verified explicitly by
`tests/feature_matrix.sh`.
