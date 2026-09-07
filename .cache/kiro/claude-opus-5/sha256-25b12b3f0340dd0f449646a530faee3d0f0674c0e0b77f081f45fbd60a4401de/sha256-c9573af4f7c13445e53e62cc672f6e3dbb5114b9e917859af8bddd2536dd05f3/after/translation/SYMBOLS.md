# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-1J8YN1.so

cd translation && cargo build --release
# -> translation/target/release/libldexp_q2_lib.so
```

## C source surface

`c_src` contains exactly one translation unit and one public header:

| file | contents |
|------|----------|
| `c_src/include/lib.h` | `float ldexp_q2(float y, int exp_q2);` (1 line, 1 declaration) |
| `c_src/src/lib.c`     | the single definition of `ldexp_q2` (12 lines) |

There are no other `.c`/`.h` files, no macro-generated symbol families, no
`#ifdef`-gated alternate entry points. So the whole library is one function; no
C module was skipped by the translation.

## `nm -D` comparison

C `.so` defined dynamic symbols (non-libc, excluding the standard
`_init`/`_fini`/`__*` link-editor artifacts, which `nm -D --defined-only`
does not list here):

```
00000000000010f9 T ldexp_q2
```

Rust `.so` defined dynamic symbols:

```
0000000000011690 T ldexp_q2
```

| # | C symbol | type | present in Rust `.so`? | Rust definition |
|---|----------|------|------------------------|-----------------|
| 1 | `ldexp_q2` | `T` (global text) | YES, exact name | `#[unsafe(no_mangle)] pub extern "C" fn ldexp_q2` in `src/lib.rs` |

### Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libharvest-work-1J8YN1.so | awk '{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libldexp_q2_lib.so | awk '{print $3}' | sort)
(empty)
```

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C
module had to be translated — the one and only C symbol was already exported.

## Undefined (imported) symbols

The Rust `.so` must not depend on anything the C one does not provide from
libc. `nm -D --undefined-only` on the Rust `.so` yields only the usual
`libc`/`libgcc_s` runtime imports pulled in by `std` (`memcpy`, `__errno_location`,
`pthread_*`, `_Unwind_*`, …). None of them are project symbols, so there are
**0 missing/undefined non-libc symbols**.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
complete set of feature combinations is the single default (empty) combination.
`--no-default-features` and the default build are the same configuration; both
are exercised (see `run_all.sh`). Phase D's "every feature combination"
requirement is satisfied by that one combination.

## Cross-build robustness checks (beyond the required matrix)

The C code's `1 << 30 >> (e >> 2)` has an out-of-range shift count for negative
`exp_q2`, which is undefined behaviour in C, so the C `.so`'s behaviour could in
principle depend on the optimisation level. It does not: both the default
(`-O0`) and `-DCMAKE_BUILD_TYPE=Release` (`-O2`) builds emit `sar edx, cl`, and
x86 masks a 32-bit shift count to its low 5 bits. The Rust translation
reproduces that with an explicit `& 31`.

The whole suite was therefore also run against:

| variation | how | result |
|-----------|-----|--------|
| C built at `-O2` (`CMAKE_BUILD_TYPE=Release`) | `C_SO_PATH=... cargo test --release` | all 55 tests pass |
| Rust `.so` from the `dev` profile (overflow checks ON) | `RUST_SO_PATH=target/debug/libldexp_q2_lib.so cargo test --release` | all 55 tests pass, no debug-assertion panic |

`C_SO_PATH` / `RUST_SO_PATH` environment variables let the harness point at any
pair of shared objects.
