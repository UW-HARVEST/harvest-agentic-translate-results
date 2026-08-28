# SYMBOLS.md — Public symbol surface (Phase A)

## Source of truth

The C library is built by `c_src/CMakeLists.txt`, which derives the project (and
therefore library) name from the **parent directory name** of `c_src`:

```cmake
cmake_path(GET CMAKE_CURRENT_SOURCE_DIR PARENT_PATH parent)
cmake_path(GET parent FILENAME project_name)
project(${project_name})
add_library(${project_name} SHARED src/lib.c)
```

In this checkout that yields `c_src/build/libharvest-work-qEO2oO.so`. Tests must
therefore **glob** `c_src/build/lib*.so` rather than hard-code the name.

The Rust crate is `crate-type = ["cdylib"]`, `name = "div_euclid_lib"`, producing
`translation/target/{debug,release}/libdiv_euclid_lib.so`.

C compile flags actually used (from `build/CMakeFiles/*.dir/flags.make`):
`C_FLAGS = -fPIC` — i.e. **no `-O` flag, so the C reference is built at `-O0`**.
This matters: `src/lib.c` contains one signed-overflow path (see `ERRORS.md`
row 8) whose observable result is the `-O0` two's-complement wrap.

## Complete C translation unit inventory

| C source file | translated to | status |
|---------------|---------------|--------|
| `c_src/src/lib.c` (33 lines, 1 function) | `translation/src/lib.rs` | complete |
| `c_src/include/lib.h` (1 declaration) | n/a (header) | complete |

There is exactly **one** C source file and **one** public function. No module was
skipped, so no additional translation work was required for symbol parity.

## `nm -D --defined-only` comparison

### C `.so`

```
00000000000010f9 T div_euclid
```

### Rust `.so` (non-`_`-prefixed, i.e. excluding Rust/`std` internals)

```
div_euclid
```

### Symbol parity table

| # | symbol | C `.so` | Rust `.so` | declared in `include/lib.h` | resolution |
|---|--------|---------|------------|-----------------------------|------------|
| 1 | `div_euclid` | `T` (global text) | `T` (global text) | yes — `int div_euclid(int v1, int v2);` | already exported via `#[unsafe(no_mangle)] pub extern "C"` |

**Missing symbols: 0.** **Extra non-internal Rust symbols: 0.**

There are no macro-generated symbols, no exported globals/data symbols, and no
exported `static` helpers in the C (`nm` shows a single `T` entry), so the diff
is empty by construction.

## Undefined-symbol audit (`nm -D --undefined-only`)

The C `.so` imports only weak CRT hooks
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`).

The Rust `.so` imports those same weak CRT hooks plus **libc and libgcc-unwind
symbols only** (`malloc`, `free`, `memcpy`, `memset`, `abort`, `mmap64`,
`dl_iterate_phdr`, `_Unwind_*`, `pthread_key_*`, …). These come from Rust's
`std`/panic-runtime, not from untranslated C code.

**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Feature configurations

`translation/Cargo.toml` has **no `[features]` section**, so the crate has
exactly one feature configuration (empty default). Phase D's "every feature
combination" therefore reduces to:

| # | invocation | meaning |
|---|------------|---------|
| 1 | `cargo test --offline` | default (= no features) |
| 2 | `cargo test --offline --no-default-features` | identical set (no default feature list exists) |
| 3 | `cargo test --offline --all-features` | identical set (no features declared) |

All three are still run explicitly by `run_all.sh` to prove the symbol set and
the differential results are invariant.

> Note: `cargo` must be invoked with `--offline` in this sandbox
> (crates.io index is unreachable); `libloading 0.8.9` and `cfg-if 1.0.4` are
> present in the local registry cache.

## Verified result

`run_all.sh` recomputes the diff for each of the 6 configurations
(dev/release x default/`--no-default-features`/`--all-features`) and reports:

```
symbol diff: EMPTY (1 C symbol(s) all present in Rust)
undefined symbols: libc/unwind only
```

for every one of them. This is also enforced as a test
(`tests/phase_d_symbols.rs`), so it cannot silently regress:

* `phase_d_every_c_symbol_is_exported_by_rust` — `nm -D --defined-only` set
  difference (C minus Rust) must be empty;
* `phase_d_rust_has_no_untranslated_undefined_symbols` — every undefined symbol
  in the Rust `.so` must be libc/libgcc-unwind;
* `phase_d_rust_so_is_loadable_and_symbol_is_callable_via_dlsym` — the
  `#[no_mangle]` wrapper is reachable exactly as an external C caller reaches
  it.

**0 symbols missing. No C source was left untranslated; no stubs were added.**
