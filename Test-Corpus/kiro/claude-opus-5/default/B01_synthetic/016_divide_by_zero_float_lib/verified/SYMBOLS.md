# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Dynamic symbols DEFINED by the C `.so`

| # | symbol | C decl | C linkage | exported by Rust `.so`? |
|---|--------|--------|-----------|-------------------------|
| 1 | `printLine`    | `void printLine(const char *line)`            | `T` (global) | YES |
| 2 | `printIntLine` | `void printIntLine(int intNumber)`            | `T` (global) | YES |
| 3 | `bad`          | `void bad(float data)`                        | `T` (global) | YES |
| 4 | `good`         | `void good(float data)`                       | `T` (global) | YES |
| 5 | `driver`       | `void driver(float goodData, float badData)`  | `T` (global) | YES |

Missing from Rust `.so`: **NONE**. Symbol diff is empty.

## C symbols deliberately NOT exported

These are `static` in `c_src/src/driver.c`, therefore absent from the C `.so`'s
dynamic symbol table. The Rust translation keeps them private too, so parity is
preserved in both directions (no extra exports either).

| symbol | C decl | why not exported |
|--------|--------|------------------|
| `goodG2B` | `static void goodG2B(void)`        | `static` → internal linkage (local `t` at `0x11d8`) |
| `goodB2G` | `static void goodB2G(float data)`  | `static` → internal linkage (local `t` at `0x1216`) |

They are still covered by the differential tests transitively, because `good()`
calls `goodG2B()` then `goodB2G(data)`, and `driver()` calls `good()`.

## Extra symbols exported by the Rust `.so` but not the C `.so`

None in the `T`/`W`/`D` set beyond the five above (Rust `cdylib` exports only
`#[no_mangle] pub extern "C"` items; the crate has no other public exports).

## UNDEFINED symbols

The C `.so` imports only `printf`, `puts` (GCC rewrites
`printf("%s\n", s)` → `puts(s)`), plus the standard weak ELF/`__cxa_finalize`
set.

The Rust `.so` imports the same `printf`/`puts` plus libc/`libgcc` runtime
symbols pulled in by Rust's `std` (`malloc`, `memcpy`, `_Unwind_*`,
`pthread_key_*`, `dl_iterate_phdr`, …). Checked with:

```sh
nm -D -u translation/target/release/libdriver.so
```

Every undefined symbol resolves against `libc`/`libgcc_s`, i.e. **0
missing/undefined non-libc symbols**. Verified concretely by `ldd -r`, which
reports no unresolved relocations.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so `default` is the
one and only configuration. `cargo check --no-default-features` and
`cargo check` are therefore the complete cross-product, and both are exercised.

## Verification commands and results

```sh
cd translation && ./run_verification.sh
```

which does, and reports:

| check | result |
|-------|--------|
| `nm -D` diff, C `.so` vs Rust `.so` (default features) | **empty** |
| `nm -D` diff, C `.so` vs Rust `.so` (`--no-default-features`) | **empty** |
| Rust `.so` exports no symbols the C `.so` lacks | confirmed |
| `ldd -r` unresolved symbols in the Rust `.so` | **none** |
| differential suite, default features | 58/58 pass |
| differential suite, `--no-default-features` | 58/58 pass |

Symbol parity is also asserted from inside the suite
(`phase_d_symbol_parity`), which shells out to `nm -D` on both objects and
fails on any C symbol missing from the Rust side, on any Rust symbol absent
from the C side, and if the C `.so`'s exported set ever changes — so this table
cannot silently go stale.

Mutation-tested: un-exporting `printIntLine` (removing its `#[no_mangle]`)
makes `phase_d_symbol_parity` and every differential case fail, confirming the
check is live rather than vacuous.

## Completeness note

No C source file was left untranslated. `c_src/` contains exactly one
implementation file (`src/driver.c`, 86 lines including the 22-line licence
header) and one header (`include/driver.h`), and every function in it —
`printLine`, `printIntLine`, `bad`, `goodG2B`, `goodB2G`, `good`, `driver` —
has a counterpart in `translation/src/lib.rs`. Nothing is stubbed and there is
no `unimplemented!()`/`todo!()` anywhere in the crate.
