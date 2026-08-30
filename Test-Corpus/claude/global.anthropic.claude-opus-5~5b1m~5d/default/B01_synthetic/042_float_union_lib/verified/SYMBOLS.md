# SYMBOLS.md — Phase A symbol surface map

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C:    `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## Translation-unit inventory (completeness check)

The whole C library is three files. Every one is accounted for in the Rust crate,
so there is no skipped module:

| C source file | contents | translated in |
|---|---|---|
| `c_src/CMakeLists.txt` | build only: `add_library(driver SHARED src/driver.c)` | `translation/Cargo.toml` (`crate-type = ["cdylib"]`) |
| `c_src/include/driver.h` | one declaration: `void driver(double f);` | `src/lib.rs` — `pub extern "C" fn driver(f: f64)` |
| `c_src/src/driver.c` | `union raw_double_t` + `driver()` body | `src/lib.rs` — `union raw_double_t` + `driver()` |

`src/driver.c` is the only compiled translation unit, and it defines exactly one
external function. There are no additional `.c` files, no macro-generated symbol
families (no `#define`-based namespace prefixing anywhere in the header), and no
exported globals or data symbols. So the expected exported surface is a single
symbol.

## Exported (defined) dynamic symbols

`nm -D --defined-only` on each library:

| # | symbol | C `.so` | Rust `.so` | type | status |
|---|--------|---------|------------|------|--------|
| 1 | `driver` | `T` (0x1109) | `T` (0x116d0) | `void driver(double)` | **MATCH** |

Counts: C exports 1 defined dynamic symbol; Rust exports 1 defined dynamic
symbol.

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
(no output)
```

**The symbol diff is EMPTY in both directions.** No symbol is missing from the
Rust `.so`, and the Rust `.so` exports no extra public symbol either (the crate
is a `cdylib`, so Rust's own generic/monomorphised items and the std allocator
shims stay local and are not published as dynamic exports). Nothing needed a
`#[no_mangle]` wrapper to be added, and no C source file was left untranslated,
so no stubbing was required or performed.

## Undefined (imported) symbols

The C library imports exactly one real function, `printf@GLIBC_2.2.5`, plus the
four standard weak CRT/ITM hooks (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`).

The Rust library imports that same `printf@GLIBC_2.2.5` — this is deliberate and
is the crux of the translation: the Rust `driver` formats by calling the *same*
libc `printf`, so `%a` / `%.4f` rounding, non-finite spellings, locale handling,
and `stdout` buffering are identical by construction rather than by
re-implementation.

The Rust library additionally imports symbols pulled in by the Rust standard
library (`_Unwind_*` from libgcc, and libc/pthread entries such as `malloc`,
`free`, `memcpy`, `write`, `dl_iterate_phdr`, `pthread_key_create`, …). These
are all resolved by libc / libgcc_s, which are already in the process image.

**0 missing non-libc symbols in the Rust `.so`** — i.e. every undefined symbol
in the Rust library resolves against the platform C runtime, and none refers to
a Rust-side item that failed to be emitted. Verified with `ldd -r`, which
reports no unresolved symbols for either library.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the only build configuration is the default one. There is
correspondingly no `#[cfg(feature = ...)]` in `src/lib.rs`. The
`--no-default-features` build is byte-identical to the default build, and both
were exercised (see `tests/`). Debug and release profiles were both tested,
which matters here because `[profile.release] panic = "abort"` applies to
release only.
