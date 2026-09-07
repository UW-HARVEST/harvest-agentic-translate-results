# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

```
C:    c_src/build/libdriver.so
Rust: translation/target/release/libdriver.so
```

## Public (dynamic, defined) symbols exported by the C `.so`

| # | symbol | type | exported by Rust `.so`? | notes |
|---|--------|------|-------------------------|-------|
| 1 | `driver` | `T` (global text) | YES (`T driver`) | `#[no_mangle] pub unsafe extern "C" fn driver` in `src/lib.rs` |

Total C symbols: **1**. Total Rust symbols: **1**. Missing from Rust: **0**.

## Full C source inventory (completeness check)

The whole library is two files; nothing was skipped by the translation.

| C file | contents | translated? |
|--------|----------|-------------|
| `c_src/include/driver.h` | single declaration `void driver(const char *s1, const char *s2);` | yes (signature matches: `*const c_char, *const c_char -> ()`) |
| `c_src/src/driver.c` | single definition of `driver`, body `printf("%zu\n", strcspn(s1, s2));` | yes |

There are no other translation units, no `static`/internal helpers, no
macro-generated symbol families, no global/`D`/`B` data symbols, and no
`#ifdef`-gated alternate definitions in the C source.

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $2, $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $2, $3}' | sort)
(empty)
```

Verified empty — see `tests/differential.rs::phase_d_symbol_parity`, which
re-derives both symbol lists at test time and asserts set equality, so the
parity claim cannot silently rot.

## Undefined (imported) symbols

Neither object may import a non-libc symbol.

| object | undefined non-libc symbols |
|--------|----------------------------|
| C | none (`printf`, `strcspn` from `libc.so.6`) |
| Rust | none (`printf`, `strcspn` from `libc.so.6`) |

Rust deliberately imports libc's `strcspn` and `printf` rather than
reimplementing them, so that the exact behaviour of the C library — including
its argument-evaluation order on invalid pointers — is reproduced. See
`ERRORS.md` row 2 for the divergence that a hand-rolled `strcspn` caused.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the
only build configuration is the default one. `cargo check --no-default-features`
and `cargo check --all-features` are both equivalent to `cargo check` here;
both are exercised by `scripts/check_all_features.sh` for completeness.
