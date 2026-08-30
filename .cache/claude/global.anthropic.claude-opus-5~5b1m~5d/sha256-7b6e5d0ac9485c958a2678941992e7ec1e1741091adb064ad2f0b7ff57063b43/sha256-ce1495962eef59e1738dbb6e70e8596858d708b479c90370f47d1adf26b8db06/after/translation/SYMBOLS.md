# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on the built shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C   : `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## C `.so` exported (defined) dynamic symbols

```
$ nm -D --defined-only c_src/build/libdriver.so
00000000000011c9 T call_fma
00000000000013b4 T driver
0000000000001139 T fma_array
```

Note: `driver.h` only declares `driver()`, but `fma_array()` and `call_fma()` are
non-`static` in `src/driver.c` and therefore part of the exported ABI. They are
also *lower-level* public entry points and are exercised directly by the
differential tests (not only through the `driver()` convenience wrapper).

## Rust `.so` exported (defined) dynamic symbols (function symbols)

```
$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011820 T call_fma
0000000000011a50 T driver
0000000000011b00 T fma_array
```

## Parity table

| # | symbol      | C `.so` | Rust `.so` | source of Rust definition                | status |
|---|-------------|---------|------------|------------------------------------------|--------|
| 1 | `fma_array` | T       | T          | `src/lib.rs` `#[unsafe(no_mangle)] fma_array` | OK |
| 2 | `call_fma`  | T       | T          | `src/lib.rs` `#[unsafe(no_mangle)] call_fma`  | OK |
| 3 | `driver`    | T       | T          | `src/lib.rs` `#[unsafe(no_mangle)] driver`    | OK |

**Missing symbols: 0.** No C translation unit was skipped — `c_src` contains
exactly one source file (`src/driver.c`) and one header (`include/driver.h`),
and every non-static function in it is implemented and exported by the Rust
crate. No stubs / `unimplemented!()` are present.

## Undefined (imported) symbols

The C library imports only libc:

```
$ nm -D -u c_src/build/libdriver.so
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
U __isoc99_sscanf@GLIBC_2.7
U printf@GLIBC_2.2.5
```

The Rust library imports the same two libc entry points
(`__isoc99_sscanf`, `printf`) plus the usual Rust `std`/`libgcc` runtime
imports (`malloc`, `memcpy`, `_Unwind_*`, `abort`, …). All are libc /
compiler-runtime symbols resolved by the dynamic loader; **0 missing or
undefined non-libc symbols.**

To keep number parsing byte-identical, the Rust translation binds the very same
glibc C99 entry point the C object links against — `__isoc99_sscanf` — on
`target_env = "gnu"`, falling back to plain `sscanf` elsewhere. (glibc's legacy
`sscanf` and `__isoc99_sscanf` differ for `%a`/positional specifiers; the format
used here is `"%d%zn"`, but binding the identical symbol removes the class of
divergence entirely.)

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one:

```
$ cargo read-manifest | python3 -c 'import json,sys; print(json.load(sys.stdin)["features"])'
{}
```

Therefore `--no-default-features` and the default build are the same code, and
Phase D's "every feature combination" requirement collapses to the single
default configuration (verified explicitly by `scripts/check_features.sh`).
