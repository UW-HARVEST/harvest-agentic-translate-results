# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-cEVbts.so
#    (CMakeLists derives the project name from the PARENT directory name,
#     so the file name is environment-dependent; tests glob for lib*.so.)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libtritanopia_lib.so
```

## Exported (defined) symbols

`nm -D --defined-only` on each library:

| # | C symbol (`nm -D`) | type | Rust `.so` exports it? | notes |
|---|--------------------|------|------------------------|-------|
| 1 | `tritanopia`       | `T`  | YES — `T tritanopia`   | the one and only public entry point |

**C total: 1 defined symbol. Rust total: 1 matching defined symbol. Symbol diff: EMPTY.**

```
$ nm -D --defined-only c_src/build/libharvest-work-cEVbts.so
0000000000001670 T tritanopia

$ nm -D --defined-only translation/target/release/libtritanopia_lib.so
0000000000011da0 T tritanopia
```

### Why there is only one symbol

`c_src/src/lib.c` is the single translation unit. Every other function in it is
declared `static` (internal linkage) and therefore is **not** part of the ABI:

| C function | linkage | in Rust `.so`? | reason |
|------------|---------|----------------|--------|
| `cbRemoveGammaRGB` | `static` | private `fn` | internal linkage — not an ABI symbol |
| `cbNorm`           | `static` | private `fn` | internal linkage — not an ABI symbol |
| `cbDenorm`         | `static` | private `fn` | internal linkage — not an ABI symbol |
| `cbApplyGammaRGB`  | `static` | private `fn` | internal linkage — not an ABI symbol |
| `Tritanopia`       | `static` | private `fn` | internal linkage — not an ABI symbol (note: capital `T`, distinct from the exported lowercase `tritanopia`) |
| `tritanopia`       | extern  | `#[no_mangle] pub extern "C"` | the exported symbol |

`include/lib.h` contains no namespacing/renaming macros, so no macro-generated
symbol names exist and the final linker name is plainly `tritanopia`.

**Consequence for Phase B:** the five low-level functions are *not reachable*
through either `.so`. The lowest-level externally reachable entry point is
`tritanopia` itself. They are therefore exercised **as the composed pipeline**,
by choosing inputs that drive each internal branch — see `CONFIGS.md`, whose rows
are the enumerated reachable *internal* branch signatures rather than a list of
callable functions.

## Undefined (imported) symbols

No missing non-libc symbols on either side.

| library | non-libc / non-runtime undefined symbols |
|---------|------------------------------------------|
| C `.so` | none (only `pow@GLIBC_2.29`, `__cxa_finalize`, `__gmon_start__`, `_ITM_*`) |
| Rust `.so` | none (`pow@GLIBC_2.29` + libc allocator/IO/`truncf` + `libgcc_s` `_Unwind_*` from the panic runtime) |

Both libraries import **the same** `pow@GLIBC_2.29`, which is required for
bit-identical transcendental results: the Rust crate binds libm's `pow` directly
via `#[link(name = "m")] extern "C" { fn pow(f64, f64) -> f64; }` instead of
using `f64::powf`.

`truncf` is imported by the Rust `.so` because `f32_to_u8_c_cast` calls
`f32::trunc`; it is libm, not a missing translation unit.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration (`--no-default-features` and the default build are the
same code). Phase D's feature-combination sweep is therefore a single cell, but
it is still executed by script over both the `dev` and `release` profiles, since
those produce two different `.so` artifacts (`panic = "abort"` and optimisation
apply only to `release`).

## Result

Symbol diff is **empty** in all four configurations (`dev`/`release` x
default/`--no-default-features`), checked mechanically by `../verify.sh`:

```
OK: all 1 C symbol(s) present in the Rust .so (Rust exports 1)
    C symbols: tritanopia
```

Nothing needed to be translated or re-exported: the C library has exactly one
public symbol and the Rust crate already exported it. No stubs were introduced.

### A note on the parity check itself

The first version of the check wrote its `nm` output to `/tmp`, which is
read-only in this sandbox; `comm` then compared two empty files and printed a
false "OK". It now writes to a scratch dir under `target/` and **fails loudly if
`nm` reports zero exported symbols**, so a broken check can no longer masquerade
as a passing one.
