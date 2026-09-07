# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

* C   `.so`: `c_src/build/libdriver.so`
* Rust `.so`: `translation/target/release/libdriver.so`

## C public headers

`c_src/include/driver.h` declares exactly one entry point:

```c
void driver(int x, int y, int z);
```

## Defined (exported) symbols

`nm -D --defined-only` output:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T driver` | `T driver` | public entry point; Rust: `#[unsafe(no_mangle)] pub extern "C" fn driver` |

Weak, compiler/linker-synthesised symbols present in BOTH objects and not part of
the library surface (ignored for parity, they come from the C runtime / crt glue):

* `_ITM_deregisterTMCloneTable` (w)
* `_ITM_registerTMCloneTable` (w)
* `__cxa_finalize@GLIBC_2.2.5` (w)
* `__gmon_start__` (w)

## Symbols present in the C source but deliberately NOT exported

| C symbol | C linkage | Rust counterpart | exported? |
|----------|-----------|------------------|-----------|
| `multi_stage` | `static int multi_stage(int x, int z)` — internal | private `fn multi_stage(x: c_int, z: c_int) -> c_int` | NO (correct — `static` in C has internal linkage, must not appear in `nm -D`) |
| `y` | `static int y = 123;` — internal file-scope mutable global | `static Y: AtomicI32 = AtomicI32::new(123);` | NO (correct — internal linkage) |

Verified: neither `multi_stage` nor `y` appears in `nm -D` for either object.

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(empty)
```

**Result: 0 symbols missing from the Rust `.so`.** No module of the C source was
skipped: `c_src` contains only `include/driver.h` + `src/driver.c`, and both are
fully translated in `translation/src/lib.rs`.

## Undefined (imported) symbols

The C object imports `printf@GLIBC_2.2.5` and `puts@GLIBC_2.2.5` (`puts` is
GCC's automatic rewrite of `printf("literal\n")`).

The Rust object imports `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, and the usual
Rust `std` runtime set (libc allocator/IO/TLS entries such as `malloc`, `free`,
`memcpy`, `write`, `pthread_key_create`, plus the `_Unwind_*` family from
libgcc). **Every Rust undefined symbol resolves to libc / libgcc_s — there are 0
missing or unresolved non-libc symbols.** Confirmed by loading the Rust `.so`
with `libloading` (an unresolved non-libc symbol would make `dlopen` fail):

```
$ ldd -r translation/target/release/libdriver.so   # no "undefined symbol" lines
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default one (`--no-default-features` is equivalent). The
Phase D "repeat for every feature combination" requirement therefore collapses to
the single default configuration. This is checked explicitly by the
`only_one_build_configuration_exists` test in `tests/phase_d_symbols.rs` (which
fails if a `[features]` section is ever added) and by the feature-power-set loop
in `scripts/run_tests.sh`.

## Verification (Phase D, all passing)

Run with `RUST_TEST_THREADS=1 cargo test --release --test phase_d_symbols`.

| check | test in `tests/phase_d_symbols.rs` |
|-------|------------------------------------|
| defined-symbol sets identical, diff empty, and the C surface is still exactly `{driver}` (so the check cannot pass vacuously) | `defined_symbol_sets_are_identical` |
| the C `static` internals (`multi_stage`, `y`) leak out of neither `.so` | `c_internal_static_symbols_are_not_exported_by_either_so` |
| 0 unresolved non-libc imports in the Rust `.so` (`nm -D --undefined-only` classified, plus a live `dlopen`) | `rust_so_has_no_unresolved_non_libc_imports` |
| `ldd -r` reports no undefined symbols in either `.so` | `ldd_reports_no_undefined_symbols_for_either_so` |
| no cargo features and no `#ifdef` in the C ⇒ the default build is the whole matrix | `only_one_build_configuration_exists` |
| the exported `driver` is real, input-dependent behaviour, not a stub | `exported_driver_is_callable_and_not_a_stub` |

Result: **6 passed, 0 failed.** Symbol diff: **empty**.

## Negative control (mutation testing)

To prove the differential suite is not vacuous, nine mutations were injected into
`src/lib.rs` one at a time (each reverted afterwards; `src/lib.rs` is
byte-identical to its original):

| mutation | caught? |
|----------|---------|
| `z != 3` → `z != 4` | yes |
| swap the `y` and `z` check order | yes |
| delete the `"Operation failed"` epilogue | yes |
| `result = 1` → `result = 4` on the `x` failure | yes |
| `"Ok!\n"` → `"OK!\n"` | yes |
| `x != 1` → `x != 0` | yes |
| drop the `\n` from `"Result: %d\n"` | yes |
| remove the `Y.store(local_y, ...)` (stale static) | yes |
| `AtomicI32::new(123)` → `AtomicI32::new(0)` | **no — correctly so**: `driver` writes the static before any read, so the C's `= 123` initialiser is provably dead and unobservable through the public API (CONFIGS.md axis A6) |

8/8 behaviour-changing mutations detected; the single unobservable one correctly
not flagged.
