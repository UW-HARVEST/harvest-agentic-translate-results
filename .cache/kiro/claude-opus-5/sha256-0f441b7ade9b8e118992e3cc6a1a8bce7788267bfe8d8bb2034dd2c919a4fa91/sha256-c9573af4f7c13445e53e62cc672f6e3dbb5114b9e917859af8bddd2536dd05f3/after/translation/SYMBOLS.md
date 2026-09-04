# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release

nm -D c_src/build/liblong.so
nm -D translation/target/release/liblong.so
```

## C source inventory

The whole library is one translation unit, `c_src/src/long.c`, with one public
header, `c_src/include/long.h`. There are exactly three file-scope definitions
with external linkage, so there is no module that could have been skipped by
the translation:

| C definition | kind | declared in header? |
|---|---|---|
| `int array[256 * 1024]` | object, 0x100000 bytes, `.bss` | no (implicitly external) |
| `void perform_expensive_operations()` | function | no (implicitly external) |
| `void long_exec(unsigned int seed)` | function | yes |

`ARRAY_SIZE`, `ITERATIONS` are object-like macros and emit no symbols. There
are no macro-generated symbol families, no `static` functions, no additional
`.c` files, and no conditional compilation.

## Defined-symbol table (the parity requirement)

| # | symbol | C type / size | Rust `.so` | match |
|---|--------|---------------|------------|-------|
| 1 | `array` | `B` (bss object), size `0x100000` | `B`, size `0x100000` | yes |
| 2 | `long_exec` | `T` (text) | `T` | yes |
| 3 | `perform_expensive_operations` | `T` (text) | `T` | yes |

Verified sizes (`nm -S -D`):

```
C:    0000000000004060 0000000000100000 B array
Rust: 0000000000050000 0000000000100000 B array
```

Symbol diff of defined symbols (C minus Rust): **empty**.

Nothing had to be added or translated: every C definition already had a
`#[no_mangle]` export in the Rust crate (`array` as `#[used] #[no_mangle] pub
static mut`, both functions as `#[no_mangle] pub extern "C"`). No stubs and no
`unimplemented!()` exist in the crate (`grep -rn 'unimplemented\|todo!' src/`
returns nothing).

## Undefined (imported) symbols

The C `.so` imports only `printf`, `srand`, `rand` from glibc, plus the four
weak toolchain symbols (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same three glibc symbols and additionally the
libc/`libgcc` symbols that the Rust standard library and its unwinder pull in
(`malloc`, `free`, `memcpy`, `mmap64`, `_Unwind_*`, `pthread_key_*`, ...). These
are all resolved by the platform C library / unwinder at load time.

Requirement check: **0 missing defined symbols and 0 unresolved non-libc
symbols in the Rust `.so`.** Confirmed by:

```sh
# defined symbols only, name-compared
diff <(nm -D --defined-only c_src/build/liblong.so       | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/liblong.so | awk '{print $NF}' | sort)

# every Rust import resolves
ldd -r translation/target/release/liblong.so   # no "undefined symbol" lines
```

## Feature combinations

`Cargo.toml` declares one optional feature, `debug-stats` (stderr diagnostics
only). The complete set of combinations is therefore:

1. `--no-default-features` (== default; there are no default features)
2. `--no-default-features --features debug-stats`

Both must export the same three symbols; `debug-stats` must not change any
exported symbol, stdout byte, or `array` byte.

## Completion gate — recorded results

| gate | evidence | result |
|---|---|---|
| `nm -D`: 0 missing defined symbols, 0 unresolved non-libc symbols in the Rust `.so` | `diff` of `nm -D --defined-only` is empty; `ldd -r` reports no undefined symbol, for **both** feature combinations | PASS |
| Phase B: every `CONFIGS.md` row passes across randomised inputs | `cargo test --test configs`: 40 passed, 0 failed | PASS |
| Phase C: every `ERRORS.md` row has a passing differential test | `cargo test --test errors`: 18 tests covering rows 1-21, 0 failed | PASS |
| Holds under every feature combination | `./verify.sh`: `### ALL FEATURE COMBINATIONS PASSED` for `<default>` and `debug-stats` | PASS |
| Exhaustive low-level equivalence | `./exhaustive.sh` on both builds: `total chunks=16384 ... total mismatches=0` | PASS |
| Live end-to-end C vs Rust (no recorded files) | `cargo test --test slow_live_c -- --ignored`: 2 passed in 1880 s; identical stdout **and** identical 1 MiB `array` after the real C `long_exec` | PASS |
| `c_src/` unmodified | `md5sum c_src/src/long.c` = `a72ea774c56c759d0fd985577aa13318`, `c_src/include/long.h` = `4be21022d629fd28b89f7a447ca247f7`; only `c_src/build/` (cmake output) added | PASS |
| No stubs | `grep -rn 'unimplemented!\|todo!' src/` returns nothing | PASS |

Full default-feature suite, final state:

```
unittests src/lib.rs   3 passed
tests/configs.rs      40 passed
tests/errors.rs       18 passed
tests/slow_live_c.rs   2 ignored (opt-in; ~31 min, run and passed separately)
tests/smoke.rs         2 passed
```

The two `#[ignore]`d tests are the *slow live* variants of rows that are already
covered non-ignored through recorded C ground truth; they are opt-in only
because each C `long_exec` call takes ~8 minutes, not to hide a failure. They
were executed and passed:

```
test live_full_pipeline_and_extra_pass ... ok
test live_repeated_and_alternating_seeds ... ok
test result: ok. 2 passed; 0 failed; ... finished in 1880.70s
```
