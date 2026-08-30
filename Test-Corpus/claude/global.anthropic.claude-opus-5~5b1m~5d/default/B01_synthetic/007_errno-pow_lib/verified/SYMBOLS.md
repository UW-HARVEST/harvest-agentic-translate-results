# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C:    `c_src/build/libpow.so`
* Rust: `translation/target/release/libpow.so`

## C source inventory (completeness check)

The whole library is two files; `CMakeLists.txt` compiles exactly one
translation unit, so there is no untranslated module.

| C file | contents | translated in Rust? |
|--------|----------|---------------------|
| `c_src/include/pow.h` | declares `double my_pow(double, double)` — the only public API | yes |
| `c_src/src/pow.c` | defines `my_pow` (the only function in the file) | yes — `translation/src/lib.rs` |

`CMakeLists.txt` `add_library(pow SHARED src/pow.c)` → single TU, `target_link_libraries(pow m)`.

## Defined (exported) dynamic symbols

`nm -D --defined-only`

| symbol | C `.so` | Rust `.so` | notes |
|--------|---------|------------|-------|
| `my_pow` | `T` | `T` | `#[unsafe(no_mangle)] pub extern "C" fn my_pow` |

No macro-generated or otherwise hidden exports exist: `pow.h` declares exactly
one function and `pow.c` defines exactly one non-static function.

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libpow.so        | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libpow.so | awk '{print $NF}' | sort)
(empty)
```

**Result: 0 symbols missing from the Rust `.so`.** Nothing was stubbed; the one
exported symbol is a real translation of the C body.

## Undefined (imported) symbols

The C `.so` imports these non-weak symbols, all of which the Rust `.so` also
imports — i.e. the Rust translation resolves the *same* libc/libm entry points
rather than reimplementing them, which is what keeps the numeric and `errno`
behaviour bit-identical:

| imported symbol | C | Rust | why |
|-----------------|---|------|-----|
| `pow@GLIBC_2.29` | U | U | Rust binds it as `#[link_name = "pow"] libm_pow` — same libm version, so identical results *and* identical `errno` side effects |
| `__errno_location@GLIBC_2.2.5` | U | U | thread-local `errno` access (C `errno` macro) |
| `fprintf@GLIBC_2.2.5` | U | U | the two diagnostic messages |
| `stderr@GLIBC_2.2.5` | U | U | target stream for the diagnostics |

The Rust `.so` additionally imports the usual Rust runtime / libgcc set
(`_Unwind_*`, `malloc`, `memcpy`, `dl_iterate_phdr`, …). All of these are
libc / libgcc symbols satisfied by `libc.so.6`, `libm.so.6` and
`libgcc_s.so.1`, as confirmed by `ldd`:

```
libgcc_s.so.1 => /lib64/libgcc_s.so.1
libm.so.6     => /lib64/libm.so.6
libc.so.6     => /lib64/libc.so.6
```

**0 missing / unresolved non-libc undefined symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section and no optional
dependencies, so the only build configuration is the default one. There is
exactly one feature combination to verify (see `CONFIGS.md`).

## Status — verified

```
$ diff <(nm -D --defined-only c_src/build/libpow.so                  | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libpow.so    | awk '{print $NF}' | sort)
(empty - symbol parity achieved)
```

Enforced continuously by `tests/phase_d_symbols.rs`:

* `d_exported_symbol_diff_is_empty` — the `nm -D` set difference must be empty,
  and asserts the C surface is still exactly one symbol, so a newly added C
  function forces these artifacts to be re-derived instead of being missed.
* `d_no_unresolved_non_libc_symbols` — every undefined symbol in the Rust `.so`
  is a libc / libm / libgcc / loader import.
* `d_both_import_the_same_libm_pow_and_errno` — both `.so`s import `pow`,
  `__errno_location`, `fprintf` and `stderr`, so numeric results and `errno`
  side effects are identical *by construction* rather than by luck.
* `d_exported_symbol_is_callable_via_dlsym` — the symbol is reachable and
  callable through `dlsym`, i.e. the `#[no_mangle] extern "C"` wrapper really is
  the ABI entry point an external C caller binds to.

Nothing was stubbed and no C module was left untranslated: `pow.c` contains one
function and it is fully translated.
