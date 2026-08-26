# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Regenerate with `scripts/symbol_diff.sh` (builds both shared objects, diffs
`nm -D --defined-only`).

* C shared object: `target/cdiff/libc_driver.so`
  (`cc -shared -fPIC c_src/src/main.c`; `c_src/CMakeLists.txt` itself only
  declares the `driver` **executable**, which is built to
  `target/cdiff/c_driver` and used for the end-to-end `main` tests)
* Rust shared object: `target/debug/libstb_perlin_cli.so`
  (`[lib] crate-type = ["cdylib", "rlib"]`)

The C translation unit is `main.c` with `STB_PERLIN_IMPLEMENTATION` defined, so
everything non-`static` in `stb_perlin.h` plus `inner`/`main` is exported.

## Defined dynamic symbols

| # | C symbol (`nm -D` in libc_driver.so) | C prototype | exported by Rust `.so` | Rust implementation |
|---|--------------------------------------|-------------|------------------------|---------------------|
| 1 | `stb_perlin_noise3_internal` | `float(float,float,float,int,int,int,unsigned char)` | yes | `src/lib.rs` wrapper → `stb_perlin::stb_perlin_noise3_internal` |
| 2 | `stb_perlin_noise3` | `float(float,float,float,int,int,int)` | yes | `src/lib.rs` wrapper → `stb_perlin::stb_perlin_noise3` |
| 3 | `stb_perlin_noise3_seed` | `float(float,float,float,int,int,int,int)` | yes | `src/lib.rs` wrapper → `stb_perlin::stb_perlin_noise3_seed` |
| 4 | `stb_perlin_ridge_noise3` | `float(float,float,float,float,float,float,int)` | yes | `src/lib.rs` wrapper → `stb_perlin::stb_perlin_ridge_noise3` |
| 5 | `stb_perlin_fbm_noise3` | `float(float,float,float,float,float,int)` | yes | `src/lib.rs` wrapper → `stb_perlin::stb_perlin_fbm_noise3` |
| 6 | `stb_perlin_turbulence_noise3` | `float(float,float,float,float,float,int)` | yes | `src/lib.rs` wrapper → `stb_perlin::stb_perlin_turbulence_noise3` |
| 7 | `stb_perlin_noise3_wrap_nonpow2` | `float(float,float,float,int,int,int,unsigned char)` | yes | `src/lib.rs` wrapper → `stb_perlin::stb_perlin_noise3_wrap_nonpow2` |
| 8 | `inner` | `float(int,float,float,float,int,int,int,int,float,float,float,int)` | yes | `src/lib.rs` wrapper → `driver::inner` |
| 9 | `main` | `int(void)` | yes | `src/lib.rs` wrapper → `driver::c_main` (same body as the `driver` binary) |

`comm -23 c_syms rust_syms` → **empty**: nothing the C `.so` exports is missing
from the Rust `.so`.

## `static` (file-local) C functions and data

These are *not* dynamic symbols, so symbol parity does not require exporting
them, but they are all translated (they are reachable through the nine symbols
above):

| C `static` symbol | Rust |
|---|---|
| `stb__perlin_lerp` | `stb_perlin::stb_perlin_lerp` |
| `stb__perlin_fastfloor` | `stb_perlin::stb_perlin_fastfloor` (+ `f32_to_i32`) |
| `stb__perlin_grad` | `stb_perlin::stb_perlin_grad` |
| `stb__perlin_ease` (macro) | `stb_perlin::stb_perlin_ease` |
| `stb__perlin_randtab[512]` | `tables::RANDTAB` |
| `stb__perlin_randtab_grad_idx[512]` | `tables::RANDTAB_GRAD_IDX` |
| `basis[12][4]` (function-local static) | `stb_perlin::BASIS` |

Observed `.data` layout of the C build (identical in the executable and in the
shared object, `nm --numeric-sort`):

```
stb__perlin_randtab           +0     (512 bytes)
stb__perlin_randtab_grad_idx  +512   (512 bytes)
basis.0                       +1024  (192 bytes)  <- end of .data
```

`src/stb_perlin.rs::read_table_mem` models exactly this contiguous window, which
is what makes the out-of-bounds table reads of
`stb_perlin_noise3_wrap_nonpow2` (reachable with wrap arguments outside
`1..=256`, see `ERRORS.md` rows E22–E25 and E29) reproducible instead of undefined;
reads beyond the window are documented as irreproducible in rows E26, E27 and E30.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` lists only libc/libgcc imports (`memcpy`, `write`,
`_Unwind_*`, …); there are **0** missing/undefined non-libc symbols.

## Symbols the Rust `.so` exports in addition

None beyond the nine above (Rust's `cdylib` exports only `#[no_mangle]` items).

## Build-time configurations

`Cargo.toml` has **no `[features]` table**, and `c_src/CMakeLists.txt` defines
no options (`STB_PERLIN_IMPLEMENTATION` is unconditionally defined by `main.c`;
`__cplusplus` is never defined in a C build). The only valid feature
combination is therefore the empty/default one; `scripts/check_features.sh`
enumerates the table and runs `--no-default-features`, `--all-features` and the
plain default build.

## How the artefacts are produced

* `scripts/build_c_so.sh` — builds the cmake executable (`target/cdiff/c_driver`)
  and the shared object (`target/cdiff/libc_driver.so`) from the untouched
  `c_src/src/main.c`. Nothing is written inside `c_src/` except cmake's own
  `c_src/build/` tree.
* `scripts/symbol_diff.sh [debug|release]` — rebuilds both libraries and prints
  the `nm -D` diff (exits non-zero if the Rust side misses a symbol).
* `src/bin/so_main_runner.rs` — test helper that `dlopen`s a library and calls
  its exported `main`, so the `main` symbol of *both* shared objects is exercised
  through the FFI boundary (`tests/driver_cli.rs::c48_so_main_export`).

## Implementation notes for the exports

* `src/lib.rs` holds one `#[no_mangle] extern "C"` wrapper per exported C
  function; the bodies live in `src/stb_perlin.rs` (library) and
  `src/driver.rs` (`inner`, `main`).
* The `main` export carries `#[cfg(not(test))]`: when cargo compiles the library
  target *as a test harness* it generates its own entry point, and two `main`
  symbols cannot be linked. The `cdylib` — the artefact the differential tests
  load — is always compiled without `test`, so it exports `main` (verified by
  `scripts/symbol_diff.sh`, which lists `main` on both sides).
* `src/main.rs` (the `driver` binary) deliberately declares the modules itself
  instead of depending on the library target, for the same reason.
