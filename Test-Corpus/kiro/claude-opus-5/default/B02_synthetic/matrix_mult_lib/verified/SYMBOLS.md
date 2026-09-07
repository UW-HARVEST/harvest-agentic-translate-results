# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` exported (defined) symbols → Rust `.so`

| # | symbol | C source (defn) | in C `.so` | in Rust `.so` | Rust site |
|---|--------|-----------------|-----------|---------------|-----------|
| 1 | `allocate_matrix`               | `src/matrix.c:33`  | T | T | `src/matrix.rs` `#[unsafe(no_mangle)]` |
| 2 | `free_matrix`                   | `src/matrix.c:66`  | T | T | `src/matrix.rs` |
| 3 | `initialize_matrix_from_string` | `src/matrix.c:78`  | T | T | `src/matrix.rs` |
| 4 | `multiply_matrices`             | `src/matrix.c:118` | T | T | `src/matrix.rs` |
| 5 | `matrix_to_string`              | `src/matrix.c:137` | T | T | `src/matrix.rs` |
| 6 | `write_to_file`                 | `src/write.c:32`   | T | T | `src/write.rs` |
| 7 | `driver`                        | `src/driver.c:35`  | T | T | `src/driver.rs` |

Note: `allocate_matrix` is absent from `include/matrix.h` but is **not** `static`
in `matrix.c`, so it is part of the exported ABI and is exported by Rust too.

`matrix_t` is a `typedef struct` — a type, not a symbol; it contributes no
dynamic symbol on either side. It is mirrored as `#[repr(C)] pub struct matrix_t`
(`*mut *mut c_int`, `c_int`, `c_int` = 16 bytes, align 8) in `src/matrix.rs`.

## Symbol diff

```
$ comm -3 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
          <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort \
            | grep -v -E '^(_ZN|_R|rust_|__rust|_ITM_|__cxa|_fini|_init)')
<empty>
```

**MISSING FROM RUST: none.** No `#[no_mangle]` wrapper had to be added and no
C module was untranslated — all three C translation units (`matrix.c`,
`write.c`, `driver.c`) have Rust counterparts (`matrix.rs`, `write.rs`,
`driver.rs`), plus `cstd.rs` for the libc surface.

## Undefined (imported) symbols

Every symbol the C `.so` imports is a libc/GCC symbol, and each is also imported
by the Rust `.so` (Rust calls straight through to libc rather than
reimplementing): `__errno_location`, `atoi`, `fclose`, `fopen`, `fprintf`,
`free`, `fwrite`, `malloc`, `perror`, `snprintf`, `stderr`, `strdup`,
`strerror`, `strlen`, `strtok_r`.

`strcat` is the one exception: C imports `strcat@GLIBC_2.2.5`, Rust provides a
byte-identical inline `cstd::strcat`. This is not an ABI difference (it is not
an exported symbol on either side) and it is covered behaviourally by the
`matrix_to_string` differential tests.

The Rust `.so` additionally imports unwinder / std-runtime symbols
(`_Unwind_*`, `memcpy`, `mmap64`, `pthread_key_create`, …). These are extra
*imports*, not missing exports, and are expected from any `cdylib`.

**0 missing exports, 0 undefined non-libc symbols in Rust.**

## Build / feature configurations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, therefore there is exactly ONE feature combination
(`--no-default-features` ≡ default ≡ `--all-features`) and no `#[cfg(feature)]`
in the source (`grep -rn 'cfg(feature' src/` → 0 hits). The C side likewise has
no `#ifdef` build switches (`grep -n '#if' c_src/src c_src/include` → 0 hits).
Phase D's "repeat for every feature combination" therefore collapses to the
single configuration, which is verified by
`translation/check_feature_combos.sh`.

`crate-type = ["cdylib"]` only — the crate builds **no binary**, and the C
`CMakeLists.txt` builds only `add_library(driver SHARED ...)` (there is no
`main()` anywhere in `c_src/`; `driver.c` exposes a `driver()` function). So the
"compare C and Rust binary stdout" gate is **not applicable**; the equivalent
end-to-end comparison is done by driving the exported `driver()` entry point and
byte-comparing the `matrix.txt` it writes (see `CONFIGS.md` rows D1–D6).

## Phase D result — symbol parity under every configuration

`check_feature_combos.sh` rebuilds the cdylib and re-runs the `nm -D` diff for
each feature invocation and each build profile, then runs the whole suite against
that exact `.so` (via `RUST_DRIVER_SO`):

```
== Cargo.toml declares no [features]: the feature cross-product is the single default configuration ==
== 3 feature combination(s) x 2 profiles ==
--no-default-features / dev      ok cargo check, ok build, ok nm -D parity: 0 missing, 86 tests passed
--no-default-features / release  ok cargo check, ok build, ok nm -D parity: 0 missing, 86 tests passed
<default>             / dev      ok cargo check, ok build, ok nm -D parity: 0 missing, 86 tests passed
<default>             / release  ok cargo check, ok build, ok nm -D parity: 0 missing, 86 tests passed
--all-features        / dev      ok cargo check, ok build, ok nm -D parity: 0 missing, 86 tests passed
--all-features        / release  ok cargo check, ok build, ok nm -D parity: 0 missing, 86 tests passed
ALL FEATURE COMBINATIONS AND PROFILES PASSED
```

Testing both profiles matters here even though the feature set is empty: the
release profile sets `panic = "abort"` and enables optimisation, so it is a
genuinely different binary from the one `cargo test` builds by default.

## C compilation flags (relevant to overflow semantics)

```
$ grep C_FLAGS c_src/build/CMakeFiles/driver.dir/flags.make
C_FLAGS = -fPIC
```

No `-ftrapv`, no `-fsanitize=signed-integer-overflow`, no optimisation level. The
C's signed-overflow sites (`matrix_to_string`'s `buffer_size`, and
`multiply_matrices`' accumulation) therefore wrap two's-complement, which is what
the Rust spells out with `wrapping_mul`/`wrapping_add`. This is confirmed
empirically, not just by reading flags: `cfg_c8_multiply_int_overflow_wraps` and
`cfg_c9_multiply_fuzz` compare overflowing products between the two `.so`s.
