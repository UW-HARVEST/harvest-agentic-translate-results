# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

Generated mechanically with `nm -D` on both shared libraries.

* C library:    `c_src/build/libSieve.so`   (cmake, default config, `gcc -O0`, PIC)
* Rust library: `target/debug/libSieve.so`  (`crate-type = ["cdylib"]`, `lib.name = "Sieve"`)

Reproduce with:

```sh
nm -D --defined-only c_src/build/libSieve.so | awk '{print $2, $3}' | sort   > /tmp/c.syms
nm -D --defined-only target/debug/libSieve.so | awk '{print $2, $3}' | sort  > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # must be EMPTY
```

## Defined (exported) dynamic symbols

| # | symbol | C `.so` type | Rust `.so` type | present in Rust? |
|---|--------|--------------|-----------------|------------------|
| 1 | `sieve` | `T` (global text) | `T` (global text) | YES |

The C library exports exactly one public symbol. `c_src/include/sieve.h` declares
exactly one entry point (`void sieve(int start);`), and `c_src/src/sieve.c`
contains exactly one function definition. There are no macro-generated symbols,
no exported data objects, no static/hidden helpers promoted to the dynamic
table, and no additional translation units in `c_src/CMakeLists.txt`
(`add_library(Sieve SHARED src/sieve.c)` — one file).

**Missing-symbol diff: EMPTY.** No symbol required a new `#[no_mangle]` wrapper
and no C source file was left untranslated (the C library is a single 40-line
file, fully translated in `src/lib.rs`).

## Undefined (imported) symbols

The C `.so` imports one non-weak libc symbol:

| symbol | kind |
|--------|------|
| `printf@GLIBC_2.2.5` | libc |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*registerTMCloneTable` | weak, toolchain glue |

The Rust `.so` imports `printf@GLIBC_2.2.5` (the same libc entry point used by
the translation, so formatting/buffering is byte-identical) plus the standard
Rust runtime imports: libc allocator/IO/TLS/`stat`/`mmap` symbols and the
`_Unwind_*` family from `libgcc_s`. Every one of these is a libc / platform
runtime symbol resolved by the loader.

**Undefined non-libc symbols in the Rust `.so`: 0.** Verified by
`ldd -r target/debug/libSieve.so` reporting no unresolved symbols.

## Build configurations

`Cargo.toml` has **no `[features]` section**, therefore the complete set of
valid feature combinations is a single one: *no features*
(`cargo check/test --no-default-features`). `c_src/CMakeLists.txt` defines no
`option()`, no `target_compile_definitions`, and `c_src/src/sieve.c` contains no
`#ifdef` other than the header include guard, so the C side likewise has exactly
one configuration. Symbol parity and all tests below are therefore verified
under the one and only configuration that exists — see `CONFIGS.md` for the
*runtime* configuration surface, which is where this library's real variability
lives.

## Verified diff (actual command output)

```
$ nm -D --defined-only c_src/build/libSieve.so
0000000000001109 T sieve

$ nm -D --defined-only target/debug/libSieve.so
0000000000011e90 T sieve

$ nm -D --defined-only target/release/libSieve.so
0000000000011c90 T sieve          (address varies per build; name/type do not)

$ comm -23 <(nm -D --defined-only c_src/build/libSieve.so | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only target/debug/libSieve.so | awk '{print $NF}' | sort -u)
            (no output — diff is EMPTY)

$ ldd -r target/debug/libSieve.so | grep "undefined symbol"
            (no output)
```

Both profiles were checked (`./verify_all.sh` step 5): the release cdylib —
built with `panic = "abort"` — exports the same `T sieve` and has no unresolved
symbols either.

The parity check is also enforced from inside the test suite, so it cannot rot:

* `d1_exported_symbol_parity` — `nm -D` on both `.so`s; asserts the C export set
  is a subset of the Rust export set, that `sieve` is `T` in both, and that the C
  library still exports exactly one public symbol.
* `d2_no_unresolved_imports` — every undefined symbol of the Rust `.so` is libc /
  platform-runtime, and `ldd -r` reports nothing unresolved.
* `d3_artifact_under_test_is_fresh` — guards against a stale `.so`: `cargo test`
  does **not** rebuild `crate-type = ["cdylib"]` targets, so
  `target/<profile>/libSieve.so` can lag behind `src/lib.rs`. This actually
  happened during verification and made a deliberately broken Rust library pass
  every differential test; the harness now falls back to compiling a fresh cdylib
  with `rustc` whenever the cargo artifact is older than the source.
* `d4_cargo_artifact_matches` — when the cargo-built cdylib is up to date, it is
  loaded separately and must produce byte-identical output to the C library.

No `#[no_mangle]` wrapper had to be added and no C source was left untranslated:
`c_src` contains exactly one translation unit (`src/sieve.c`, 40 lines, one
function) and `src/lib.rs` implements it.
