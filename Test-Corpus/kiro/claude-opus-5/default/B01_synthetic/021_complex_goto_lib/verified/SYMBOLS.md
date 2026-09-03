# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so
```

## Translation-unit inventory

| C source file | translated in Rust? | where |
|---|---|---|
| `c_src/src/driver.c` | yes | `translation/src/lib.rs` |
| `c_src/include/driver.h` (decls only) | n/a — declares `driver` only | — |

There is exactly **one** C translation unit, so there is no skipped-module class
of completeness failure here.

## Public symbols of the C `.so`

`nm -D --defined-only` on `libdriver.so` (C), filtered to non-libc,
non-toolchain symbols:

| # | symbol | type | present in Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `driver` | `T` (global text) | **yes** (`T driver`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_int, y: c_int)` |

No macro-generated symbols exist: `grep -nE '#(if|ifdef|ifndef|define|else|elif)'`
over `src/driver.c` and `include/driver.h` finds only the `DRIVER_H_` include
guard, which emits no symbols.

## Symbol diff

```
comm -23 <(c defined symbols) <(rust defined symbols)   ->  (empty)
```

* Symbols in C but missing from Rust: **0**
* Undefined (`U`) non-libc symbols in the Rust `.so`: **0** — the only
  undefined symbols are libc imports (`printf`, plus the usual
  `__cxa_*`/unwind/`memcpy`-class runtime imports pulled in by `libstd`),
  which the C `.so` also imports from libc.

Verified by `translation/check_symbols.sh`, which fails with a non-zero exit
status if the diff is ever non-empty.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only build
configuration is the default one. `translation/check_features.sh` re-derives the
feature list from `Cargo.toml` and would loop over every combination; it
currently confirms the single default configuration.
