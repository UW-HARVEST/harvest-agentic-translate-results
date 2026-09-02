# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands:

```sh
nm -D --defined-only c_src/build/libharvest-work-dvbeFO.so
nm -D --defined-only translation/target/release/libtfm_lib.so
nm -D --undefined-only translation/target/release/libtfm_lib.so
```

## C source inventory (completeness check)

The whole C library is two files, and both are accounted for:

| C file | translated in | notes |
|---|---|---|
| `c_src/include/lib.h` | `translation/src/lib.rs` | single declaration: `void tfm(float*, const float*, int)` |
| `c_src/src/lib.c` | `translation/src/lib.rs` | single definition: `tfm` |

No C module was skipped, so no Phase A "translate the missing source" work applies.

## Exported symbols

| # | C symbol (`nm -D` C `.so`) | type | exported by Rust `.so`? | Rust item |
|---|---|---|---|---|
| 1 | `tfm` | `T` (global text) | YES — `T tfm` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn tfm` |

The C `.so` exports exactly **one** non-libc, non-linker-synthetic symbol. There
are no macro-generated symbols, no exported data objects, and no versioned
aliases.

`nm -D --defined-only` on the C `.so`, restricted to real symbols:

```
0000000000001109 T tfm
```

`nm -D --defined-only` on the Rust `.so`, restricted to real symbols:

```
T tfm
```

### Symbol diff

```
C-only symbols missing from Rust : (none)
```

The diff is **empty**. See `tests/symbol_parity.rs`, which recomputes this diff
at test time and fails if it is ever non-empty.

## Undefined symbols in the Rust `.so`

All undefined (`U`/`w`) entries in the Rust `.so` are libc / libgcc-unwind /
linker-synthetic imports pulled in by the Rust standard library
(`malloc`, `memcpy`, `pthread_key_create`, `_Unwind_*`, `__gmon_start__`, …).
There are **0 missing/undefined non-libc symbols**.

Note the C `.so` imports `sqrtf@GLIBC_2.2.5` (via PLT); the Rust `.so` has no
such import because `f32::sqrt` lowers to an inline `sqrtss`. That is an import
difference, not an export difference, and the two are bit-identical on this
target (see `src/lib.rs` `sqrtf` docs and Phase B row 22).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default (empty) feature set. `cargo test`,
`cargo test --no-default-features` and `cargo test --all-features` all resolve
to that same single configuration. Verified by `scripts/verify_all.sh`.
