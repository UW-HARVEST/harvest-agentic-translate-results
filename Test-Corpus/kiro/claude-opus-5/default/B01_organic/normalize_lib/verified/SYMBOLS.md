# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-OhlKp8.so` (name comes from
  `CMakeLists.txt`, which derives `project_name` from the parent directory name).
* Rust `.so`: `translation/target/release/libnormalize_lib.so`
  (`[lib] crate-type = ["cdylib"], name = "normalize_lib"`).

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-OhlKp8.so | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libnormalize_lib.so | awk '{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # must print nothing
```

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | source of the Rust export |
|---|--------|-----------|---------------|---------------------------|
| 1 | `normalize` | yes (`T`) | yes (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn normalize` in `src/lib.rs` |

The C translation unit `c_src/src/lib.c` contains exactly one function
definition and `c_src/include/lib.h` declares exactly one prototype, so the
public surface is a single symbol. There are no macro-generated symbols, no
namespace/prefix macros, no `#ifdef`-gated alternate names, no exported
globals, and no additional C source files in `add_library(...)`.

**Missing symbols: 0.** Nothing needed to be exported or translated.

## Undefined (imported) symbols

C `.so` imports: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `memset`, `sqrtf`.

Rust `.so` imports: the same CRT/ITM hooks plus libc (`memset`, `memcpy`,
`malloc`, `free`, …), libgcc unwinder (`_Unwind_*`) and glibc syscall wrappers
pulled in by `std`. `sqrtf` is absent because rustc lowers `f32::sqrt` to the
`sqrtss` instruction rather than a libm call — this is a codegen difference,
not a behavioural one (both are the IEEE-754 correctly-rounded square root).

**Undefined non-libc / non-toolchain symbols in the Rust `.so`: 0.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only configuration is the default (empty) feature set. `--no-default-features`
and the default build are the same build. Verified with:

```sh
grep -n '^\[features\]' translation/Cargo.toml   # no match
```

Both `cargo test` and `cargo test --no-default-features` are run by
`run_all.sh`, and both must be green.

## Results

Checked mechanically by `tests/symbols.rs` (3 tests) and again, independently of
cargo, by section 4 of `run_all.sh`:

```
===== nm -D symbol diff (C vs Rust debug) =====
OK: 0 symbols missing from the Rust .so (debug)
C exports: normalize
===== nm -D symbol diff (C vs Rust release) =====
OK: 0 symbols missing from the Rust .so (release)
C exports: normalize
```

* `symbols_rust_defines_everything_the_c_so_defines` — the `nm -D
  --defined-only` set difference (C minus Rust) is empty.
* `symbols_rust_has_no_unresolved_non_runtime_imports` — every undefined symbol
  in the Rust `.so` is either a libc/libgcc/CRT import or one the C `.so`
  imports too.
* `symbols_c_surface_is_still_just_normalize` — fails loudly if the C source
  grows another entry point, so this file cannot silently go stale.

Nothing had to be added: no symbol was missing, and no C module was left
untranslated (`c_src` contains exactly `include/lib.h` and `src/lib.c`, and
`add_library` lists only `src/lib.c`).
