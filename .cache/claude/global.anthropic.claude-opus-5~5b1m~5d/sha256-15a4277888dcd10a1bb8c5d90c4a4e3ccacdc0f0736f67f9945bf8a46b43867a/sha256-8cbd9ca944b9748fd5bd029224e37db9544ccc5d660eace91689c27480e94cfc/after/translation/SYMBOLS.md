# SYMBOLS.md — Phase A: exported-symbol surface

## Source of truth

C shared library built with:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

Artifact: `c_src/build/libharvest-work-nUPNTl.so`
(the CMakeLists derives the project/library name from the parent directory name).

Rust artifact: `translation/target/release/libpremultiply_lib.so`
(`crate-type = ["cdylib"]`, `[lib] name = "premultiply_lib"`).

## `nm -D --defined-only` on the C `.so` (non-libc, non-CRT symbols)

```
0000000000001139 T premultiply
```

Filtering note: `nm -D` on the C `.so` also lists the usual toolchain-supplied
dynamic symbols that are not part of the library's own API surface
(`_init`, `_fini`, `__bss_start`, `_edata`, `_end`, and the undefined libc
imports). Those are emitted by the linker / crt objects, not by `src/lib.c`,
and are deliberately excluded from the parity requirement below. The complete
set of symbols originating from the C *source* is the single function above.

## Parity table

| # | C symbol | type | C `.so` | Rust `.so` | status |
|---|----------|------|---------|------------|--------|
| 1 | `premultiply` | `T` (global text) | yes | yes | **MATCH** |

## Rust `.so` exported symbols originating from the crate

```
0000000000011760 T premultiply
```

`rust_eh_personality` is *not* present because `[profile.release] panic = "abort"`.
No other crate-originated dynamic symbols are exported: `cp_pixel_t` and
`cp_image_t` are types only (no runtime symbol in either language), and
`PIXEL_SIZE` / `c_float_to_u8` are private crate items that produce no dynamic
symbol.

## Missing-symbol analysis

Missing from Rust `.so`: **none**.

Every symbol defined by `c_src/src/lib.c` is exported by the Rust `cdylib` under
the exact same name, with the same binding (`GLOBAL`) and the same type
(`FUNC` / text). No `#[no_mangle]` wrapper had to be added and no C module was
found to be untranslated: `c_src/src/lib.c` contains exactly one function and
`c_src/include/lib.h` declares exactly that one function.

Verification command used for the diff (must print nothing):

```
diff <(nm -D --defined-only c_src/build/*.so \
        | awk '{print $3}' | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' | sort) \
     <(nm -D --defined-only translation/target/release/libpremultiply_lib.so \
        | awk '{print $3}' | grep -vE '^(_init|_fini|__bss_start|_edata|_end|__rust_.*|rust_eh_personality)$' | sort)
```

## Undefined (imported) symbols

The C `.so` imports nothing from libc for this translation unit other than what
the CRT adds (`__gmon_start__`, `_ITM_*`, `__cxa_finalize` weak refs). The Rust
`.so` has no non-libc undefined symbols. **0 missing / unresolved non-libc
symbols in the Rust build.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
buildable configuration is the default one (empty feature set).
`cargo check --no-default-features` and `cargo check --all-features` resolve to
the same unit, so symbol parity and the Phase B/C test suites need only be run
once; this is recorded in the Phase D checklist.
