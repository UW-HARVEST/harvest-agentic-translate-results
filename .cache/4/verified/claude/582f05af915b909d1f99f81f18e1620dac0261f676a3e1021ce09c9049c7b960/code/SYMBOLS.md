# SYMBOLS.md — Public ABI surface parity (Phase A / Phase D)

Derived mechanically from `nm -D` on the built shared objects.

* C  `.so`: `c_src/build/libdriver.so` (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/debug/libdriver.so` (`crate-type = ["cdylib"]`)

Reproduce with `./check_symbols.sh`.

## Raw `nm -D` output

### C library

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001173 T driver
                 U printf@GLIBC_2.2.5
                 U putchar@GLIBC_2.2.5
```

### Rust library (defined text symbols)

```
00000000000120c0 T driver
```

## Defined-symbol table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T` (defined, global text) | `T` (defined, global text) | `void driver(int)`. Rust: `#[unsafe(no_mangle)] pub extern "C" fn driver(floors: c_int)` in `src/lib.rs`. |

**Total C defined dynamic symbols: 1. Total present in Rust: 1. Missing: 0.**

## Symbols intentionally NOT required

These are *not* part of the translated surface and are correctly absent from the
comparison:

| symbol | `nm` type | why excluded |
|--------|-----------|--------------|
| `_ITM_deregisterTMCloneTable` | `w` (weak undefined) | GCC transactional-memory link stub, emitted by the C toolchain's crt glue, not library API. |
| `_ITM_registerTMCloneTable` | `w` (weak undefined) | same |
| `__cxa_finalize@GLIBC_2.2.5` | `w` (weak undefined) | glibc destructor registration, toolchain glue. |
| `__gmon_start__` | `w` (weak undefined) | profiling hook, toolchain glue. |
| `printf@GLIBC_2.2.5` | `U` (undefined) | libc import, satisfied at load time. Rust `.so` imports it too. |
| `putchar@GLIBC_2.2.5` | `U` (undefined) | libc import. Present only because GCC rewrote the C source's `printf("\n")` into `putchar('\n')` as an optimization. The Rust translation calls `printf("\n")` directly, which is byte-for-byte equivalent on stdout; see Phase B row 17/18/19 which assert exact output bytes including the trailing newline. |

`static void print_hex(unsigned char *, int)` is `static` in `c_src/src/driver.c`
and therefore has **no** dynamic symbol; it is not part of the ABI and is
reachable only through `driver`. The Rust translation likewise keeps
`fn print_hex` private. Phase B row 20 exercises it indirectly at the only `len`
value it can ever receive (`sizeof(house_t)` == 16).

## Undefined non-libc symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` yields only libc / libgcc / libm /
ld.so imports (`printf`, `memcpy`, `__libc_start_main`-family, unwinder, TLS and
`pthread` symbols). **0 missing/undefined non-libc symbols.**

## Completeness of the translation

Every C translation unit named in `c_src/CMakeLists.txt` is accounted for:

| C source file | translated to | status |
|---|---|---|
| `c_src/src/driver.c` | `src/lib.rs` | fully translated (`house_t` → `HouseT`, `print_hex` → `print_hex`, `driver` → `driver`) |
| `c_src/include/driver.h` | declaration only (`void driver(int)`) | exported |

No C module was skipped, so no additional translation work was required and no
symbol is stubbed.

## Verification status

`./check_symbols.sh [debug|release]` reduces the C-vs-Rust defined-symbol diff to
**empty** in both profiles:

```
=== C .so   : c_src/build/libdriver.so (1 defined symbols) ===
driver
=== Rust .so: target/debug/libdriver.so (1 defined symbols) ===
driver
== Undefined non-libc symbols in the Rust .so ==
(none)
== PASS: symbol diff is EMPTY -- 0 missing, 0 unresolved non-libc ==
```

This is also enforced as tests (`tests/phase_d_symbols.rs`, 4 tests) so it cannot
regress: `d01` diffs `nm -D`, `d02` checks every C symbol is `dlsym`-resolvable in
both libraries, `d03` checks the Rust `.so` has no unresolved non-libc imports,
and `d04` pins the C surface so drift in the C build is noticed. Deleting the
`#[unsafe(no_mangle)]` attribute makes 26 tests fail, confirming the check is live.
