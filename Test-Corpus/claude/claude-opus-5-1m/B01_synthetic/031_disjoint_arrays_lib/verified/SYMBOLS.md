# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libdriver.so`
  (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: built by the harness with
  `rustc --crate-type=cdylib --crate-name=driver --edition=2021` into
  `target/<profile>/harness-so-opt<N>-da<on|off>/libdriver.so`.

  This matters: **`cargo test` does not emit the cdylib at all.** For a crate
  whose only `[lib]` `crate-type` is `cdylib`, `cargo test` builds the library as
  an rlib to link the test binaries and produces no `libdriver.so`, so pointing
  the harness at `target/<profile>/libdriver.so` silently loads a stale
  `cargo build` artifact. `tests/common/mod.rs` compiles the artifact under test
  itself and rebuilds it whenever `src/lib.rs` is newer. The
  `cargo build --release` artifact is additionally verified via
  `DRIVER_RUST_SO` (see `run_all_feature_combos.sh`).

Regenerate / re-diff with:

```sh
nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u > /tmp/c.syms
nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # must be EMPTY
```

The same diff is enforced automatically by `tests/phase_d_symbols.rs`
(`symbol_parity_c_so_vs_rust_so`), which fails if the C `.so` exports any name
the Rust `.so` does not.

## C source inventory (completeness check)

`c_src/CMakeLists.txt` globs exactly one translation unit into the library:

| C source file | translated to | status |
|---------------|---------------|--------|
| `c_src/src/driver.c`     | `src/lib.rs` | fully translated (3/3 functions) |
| `c_src/include/driver.h` | (declarations only — `driver`) | n/a |

No C source file is untranslated, so no symbol is missing because a module was
skipped.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | kind | C definition | Rust definition |
|---|--------|---------|------------|------|--------------|-----------------|
| 1 | `fma_array` | `T` (global text) | `T` (global text) | function | `driver.c:27` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn fma_array` |
| 2 | `call_fma`  | `T` (global text) | `T` (global text) | function | `driver.c:33` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn call_fma` |
| 3 | `driver`    | `T` (global text) | `T` (global text) | function | `driver.c:48` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

Notes:

* Only `driver` is declared in the public header; `fma_array` and `call_fma`
  have external linkage in `driver.c` (no `static`), so the C `.so` exports
  them too. All three are therefore part of the ABI under test and all three
  are exercised directly by the differential tests.
* There are no namespace/renaming macros in the C sources (the only `#define`
  is the `DRIVER_H_` include guard), so linker names equal source names.
* No exported data symbols, no weak aliases, no macro-generated symbols.

### Observed `nm -D --defined-only` output

C `.so`:

```
0000000000001139 T fma_array
00000000000011c9 T call_fma
00000000000013b4 T driver
```

Rust `.so`:

```
0000000000011dc0 T call_fma
0000000000011ff0 T driver
00000000000120a0 T fma_array
```

**Diff (`comm -23 c.syms r.syms`): EMPTY — 0 missing symbols.**

## Undefined (imported) symbols

Not required to match (they reflect the runtime, not the ABI), but recorded
because one difference is behaviourally relevant:

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|------------|------|
| `__isoc99_sscanf@GLIBC_2.7` | imported | – | gcc rewrites `sscanf` -> `__isoc99_sscanf` via `<stdio.h>` |
| `sscanf@GLIBC_2.2.5` | – | imported | Rust `extern "C" { fn sscanf(...) }` binds the legacy entry point |
| `printf@GLIBC_2.2.5` | imported | imported | identical |

`__isoc99_sscanf` and the legacy `sscanf` differ only in the interpretation of
the GNU `%a`/`%as` allocation modifier. The only format string used is
`"%d%zn"`, which contains no `%a`, so both entry points share the exact same
code path. `tests/phase_b_valid.rs::sscanf_entrypoint_equivalence_d_zn` pins
this down by calling both glibc entry points with `"%d%zn"` over randomized
inputs and asserting identical results, so the difference cannot silently
become a divergence.
