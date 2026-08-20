# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared libraries.

- C   : `c_src/build/libdriver.so`   (cmake, gcc 11.5.0, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
- Rust: `target/release/libdriver.so` (`cargo build --release`, `crate-type = ["cdylib"]`)

Regenerate with:

```sh
nm -D --defined-only c_src/build/libdriver.so     | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only target/release/libdriver.so  | awk '$2 ~ /^[TtWwDdBb]$/ {print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms      # MUST be empty
```

## Defined (exported) symbols

`nm -D --defined-only` on the C `.so` yields exactly 5 symbols. All 5 are
exported by the Rust `.so` under the exact same name.

| # | symbol         | C type / addr    | Rust `.so` | source of Rust export                          |
|---|----------------|------------------|------------|------------------------------------------------|
| 1 | `printLine`    | `T` @ 0x1149     | `T` YES    | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |
| 2 | `printIntLine` | `T` @ 0x116b     | `T` YES    | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |
| 3 | `bad`          | `T` @ 0x1192     | `T` YES    | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |
| 4 | `good`         | `T` @ 0x1239     | `T` YES    | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |
| 5 | `driver`       | `T` @ 0x12e8     | `T` YES    | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |

**Symbol diff (`comm -23 c.syms r.syms`): EMPTY.** No symbol is missing from the
Rust `.so`, so no C source file was skipped by the translation and no
`#[no_mangle]` wrapper needed to be added. `c_src` contains exactly one
translation unit (`src/driver.c`) plus one header (`include/driver.h`); the
header declares only `driver`, while `printLine`/`printIntLine`/`bad`/`good`
have external linkage in the `.c` file and are therefore exported too.

### Note: `good` and `bad` share one address in the Rust `.so`

In `target/release/libdriver.so` both `bad` and `good` resolve to the same
address (0x11d90). This is not a missing symbol — it is LLVM identical-code
folding. After translation the two bodies are literally identical (see
`ERRORS.md` / `CONFIGS.md` note on `alloca`), so the linker merged them. Both
names are still present and independently callable via `dlsym`, which is what
the differential tests do.

## Undefined symbols

The Rust `.so`'s undefined list must contain no non-libc entries.

| library | undefined symbols                                                                 |
|---------|-----------------------------------------------------------------------------------|
| C       | `printf`, `puts` + 4 weak toolchain stubs (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`) |
| Rust    | `printf`, `puts`, plus glibc (`malloc`, `memcpy`, `write`, `open64`, `pthread_key_create`, …) and `_Unwind_*` from libgcc |

**0 missing / 0 non-libc undefined symbols in the Rust `.so`.** Every Rust
undefined symbol is satisfied by `libc`/`libgcc_s`, which the loader provides;
the extra entries relative to C come from the Rust standard library's panic
runtime and allocator shim, not from unresolved translation units.

Interesting parity detail: gcc rewrites `printf("%s\n", line)` into `puts(line)`,
so the C `.so` imports `puts`. The Rust `.so` imports **both** `printf` and
`puts` because it calls `printf` directly for `%s\n`/`%d\n` and `std` references
`puts` elsewhere. The emitted bytes are identical either way (`puts` appends
exactly one `\n`), which the Phase B tests confirm byte-for-byte.

## Feature combinations

`Cargo.toml` has **no `[features]` table**, and `c_src/CMakeLists.txt` defines no
`option()`, no `target_compile_definitions`, and no `#ifdef`-selected sources
(`src/driver.c` is the only source; the only `#ifndef` is the `DRIVER_H_` include
guard). Therefore the complete set of valid build configurations is the single
empty one:

| # | cargo invocation                     | cmake configuration | status |
|---|--------------------------------------|---------------------|--------|
| 1 | `cargo test --no-default-features`   | default (only one)  | verified |

`--no-default-features` and the plain default build are the same configuration
because there is no `default` feature to disable. Phases B and C are run under
this combination, which is exhaustive for this crate.
