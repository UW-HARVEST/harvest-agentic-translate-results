# SYMBOLS.md — Symbol parity: C `.so` vs Rust `.so`

Derived mechanically from `nm -D` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-lEcaeQ.so`
- Rust `.so`: `translation/target/release/libfallcalc_lib.so`

Regenerate / re-verify with `./check_symbols.sh` in the crate root.

## C source inventory (`c_src/src/lib.c`)

Every non-`static` function definition in the single C translation unit. There
is exactly one C source file (`src/lib.c` per `c_src/CMakeLists.txt`), so no
module was skipped by the translation.

| C definition | external linkage | declared in `include/lib.h` | translated in `src/lib.rs` |
|---|---|---|---|
| `int safe_double_to_int(double)` | yes | no | yes |
| `int process_array_reverse(int *, int)` | yes | no | yes |
| `int switch_fallthrough_calculator(int, int)` | yes | no | yes |
| `int allocate_and_compute(int, double)` | yes | no | yes |
| `int foreach_sum(int *, int)` | yes | no | yes |
| `int fallcalc(int, int, int, int)` | yes | yes | yes |

Non-function C constructs, for completeness — none of these produce a symbol:

| construct | kind | notes |
|---|---|---|
| `FOREACH(item, array, count)` | function-like macro | expanded inline inside `foreach_sum`; emits no symbol |
| `OCTAL_MASK_1` = `0777` = 511 | object-like macro | no symbol |
| `OCTAL_MASK_2` = `0100` = 64 | object-like macro | no symbol |
| `OCTAL_FLAG` = `0200` = 128 | object-like macro | no symbol |
| `OCTAL_BASE` = `010` = 8 | object-like macro | no symbol |
| `DataPoint` = `{ int value; double coefficient; }` | typedef struct | no symbol; `#[repr(C)]` in Rust |

There are no global/static variables, so no `B`/`D`/`R` data symbols exist on
either side.

## Exported (defined) symbols

`nm -D --defined-only`, Rust side filtered of Rust-internal symbols
(`_ZN…` mangled items, `__rust_*`, `__rdl_*`, `__rg_*`).

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `safe_double_to_int` | `T` | `T` | OK |
| 2 | `process_array_reverse` | `T` | `T` | OK |
| 3 | `switch_fallthrough_calculator` | `T` | `T` | OK |
| 4 | `allocate_and_compute` | `T` | `T` | OK |
| 5 | `foreach_sum` | `T` | `T` | OK |
| 6 | `fallcalc` | `T` | `T` | OK |

**Symbols exported by C but MISSING from Rust: 0.**

No `#[no_mangle]` wrapper had to be added and no C module had to be translated
from scratch: `c_src/src/lib.c` is the only C source file and all six of its
external-linkage functions were already present and exported.

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` | notes |
|--------|---------|------------|-------|
| `malloc@GLIBC_2.2.5` | `U` | `U` | Rust binds libc `malloc` directly, not Rust's global allocator, so allocation-failure behaviour is identical |
| `free@GLIBC_2.2.5` | `U` | `U` | same |
| `_ITM_deregisterTMCloneTable` | `w` | — | toolchain-emitted weak stub |
| `_ITM_registerTMCloneTable` | `w` | — | toolchain-emitted weak stub |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` | toolchain-emitted weak stub |
| `__gmon_start__` | `w` | — | toolchain-emitted weak stub |

**Undefined non-libc symbols in the Rust `.so`: 0.** The Rust `.so` imports
only libc (`malloc`, `free`, plus the usual libc/`ld.so` runtime entries) and
loads with `RTLD_NOW` without unresolved references — verified by the
`libloading` tests, which open it eagerly and would fail on any missing symbol.

## Rust `math.h` mapping

The C code includes `math.h` for `isnan`/`isinf`; both are compiler builtins at
`-O0` and generate no imported symbol. Rust uses `f64::is_nan` /
`f64::is_infinite`, which are equivalent bit-level predicates.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one (`--no-default-features` and the default build
are the same build). Verified mechanically by `./check_features.sh`. There are
also no `#[cfg(...)]` attributes in `src/lib.rs` and no `#ifdef` conditionals on
behaviour in `c_src/src/lib.c`.
