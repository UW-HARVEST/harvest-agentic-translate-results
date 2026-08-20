# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared libraries.

## How the two `.so` files are produced

```sh
# C  (ground truth)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/libdriver.so

# Rust (crate-type = ["cdylib"])
cargo build --offline
#   -> target/debug/libdriver.so
```

## C source inventory (completeness check)

Every translation unit compiled into the C `.so`, per `c_src/CMakeLists.txt`
(`add_library(driver SHARED src/driver.c)`):

| C file | public functions defined | translated in Rust? |
|--------|--------------------------|---------------------|
| `c_src/src/driver.c` | `driver` | yes — `src/lib.rs::driver` |
| `c_src/include/driver.h` | *(declarations only: `void driver(int, int)`)* | n/a (header) |

There is exactly **one** `.c` file in the project, so no module/file was skipped
by the translation. Nothing had to be newly translated for Phase A.

Note on the C dialect: `driver.c`/`driver.h` are written with **digraphs**
(`%:` = `#`, `<%` = `{`, `%>` = `}`) and the `<iso646.h>` alternative operator
spellings (`bitor` = `|`, `compl` = `~`). Verified with `gcc -E`:

```c
void driver(int x, int y) {
    int result = x | ~ y;
    printf("%d", result);
    puts("");
}
```

## Defined (exported) dynamic symbols

`nm -D --defined-only` output, sorted:

| # | symbol | C `.so` | Rust `.so` | type | notes |
|---|--------|---------|------------|------|-------|
| 1 | `driver` | `T` (0x1119) | `T` | func | `void driver(int x, int y)`; exported from Rust via `#[unsafe(no_mangle)] pub extern "C" fn driver` |

* Symbols only in C `.so`: **none**
* Symbols only in Rust `.so`: **none**
* No macro-generated / aliased / versioned exports exist in the C library
  (there are no macros in `driver.c` other than the `%:include` directives, and
  no `__attribute__((alias))`, no `.symver`, no visibility attributes).
* There are no exported data objects (`D`/`B`/`R`) in either library.

## Undefined (imported) symbols

The C `.so` imports `printf@GLIBC_2.2.5` and `puts@GLIBC_2.2.5` plus the usual
weak CRT hooks (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same `printf`/`puts` (the translation deliberately
calls the platform libc through `extern "C"` instead of `std::io`, so both
libraries write through the *same* glibc `stdout` `FILE` object), plus the Rust
runtime's own libc/libgcc imports (`_Unwind_*`, `malloc`, `memcpy`, `write`,
`__errno_location`, `pthread_key_*`, …).

**0 undefined non-libc / non-libgcc symbols** in the Rust `.so` — every import
is satisfied by `libc.so.6`, `libgcc_s.so.1` or is a weak CRT hook. This is
verified the authoritative way, by asking the dynamic linker itself: `ldd -r`
performs full function+data relocation resolution and reports nothing for either
library. `tests/phase_d_symbols.rs::rust_so_has_no_unresolved_non_libc_symbols`
re-runs that check on every test run.

Note on the *imported* (not exported) set: the `release` cdylib imports
`putchar` where the `dev` cdylib imports `puts`, because LLVM rewrites
`puts("")` into `putchar('\n')`. GCC performs the same rewrite at `-O2`; the C
here is built unoptimised so it keeps `puts`. The emitted bytes are identical
(a single `'\n'`), which the differential tests confirm for the release
artifact too, including under `setvbuf(_IONBF)` where each call reaches
`write(2)` separately. Imported-symbol names are not part of the ABI contract —
only the exported set is, and that matches exactly.

## Result

| gate | status |
|------|--------|
| every symbol exported by the C `.so` is exported by the Rust `.so`, exact name | PASS |
| symbol diff reaches empty | PASS (both directions) |
| 0 missing / undefined non-libc symbols in Rust | PASS |

Enforced automatically by `tests/phase_d_symbols.rs`
(`symbol_parity_c_so_vs_rust_so`), which re-runs the `nm -D` diff on every
`cargo test` run rather than trusting this snapshot.
