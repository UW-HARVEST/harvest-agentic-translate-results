# SYMBOLS.md — Symbol parity between C `.so` and Rust `.so`

## Artifacts compared

| side  | artifact |
|-------|----------|
| C     | `c_src/build/libtranslated_rust.so` (cmake, default config, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`) |
| Rust  | `target/debug/libfindrep_lib.so` (`crate-type = ["cdylib"]`) |

## Feature combinations

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` defines
**no compile options / `#ifdef` switches** (the single translation unit
`src/lib.c` contains no preprocessor conditionals other than `#include`).
Therefore the complete set of valid build configurations is exactly one:

| # | configuration | cargo invocation |
|---|---------------|------------------|
| 1 | default = no features | `cargo check/test --no-default-features` (identical to `cargo check/test`) |

Both invocations were run and both are clean (see `logs`/session transcript).

## Defined (exported) symbols

`nm -D --defined-only` on both libraries (addresses elided; sorted):

| # | C symbol | present in Rust `.so` | notes |
|---|----------|-----------------------|-------|
| 1 | `add_to_accumulator`        | YES (`T`) | `#[no_mangle] extern "C"` |
| 2 | `multiply_with_multiplier`  | YES (`T`) | `#[no_mangle] extern "C"` |
| 3 | `subtract_from_accumulator` | YES (`T`) | `#[no_mangle] extern "C"` |
| 4 | `divide_multiplier`         | YES (`T`) | `#[no_mangle] extern "C"` |
| 5 | `process_octal_string`      | YES (`T`) | `#[no_mangle] extern "C"` |
| 6 | `find_and_replace_char`     | YES (`T`) | `#[no_mangle] extern "C"` |
| 7 | `validate_and_normalize`    | YES (`T`) | `#[no_mangle] extern "C"` |
| 8 | `findrep`                   | YES (`T`) | `#[no_mangle] extern "C"`; the only symbol declared in `include/lib.h` |

**Missing symbols: 0.** The C `.so` exports 8 global text symbols; the Rust
`.so` exports the same 8 names. No C translation unit was skipped: `c_src`
contains exactly one source file (`src/lib.c`, 174 lines) and one header
(`include/lib.h`, 1 line), and every function definition in it is translated in
`src/lib.rs`.

C file-scope objects (`accumulator`, `multiplier`, `operation_count`,
`operations[4]`) are `static` in C, so they are **not** exported by the C `.so`
(confirmed: no `B`/`D` data symbols in `nm -D`). The Rust translation likewise
keeps them private (`static ACCUMULATOR/MULTIPLIER/OPERATION_COUNT`,
`static OPERATIONS`), so there is nothing to export for them. The two
`typedef`s (`operation_func`, `string_processor`) produce no symbols.

## Undefined symbols (imports)

All undefined symbols in the Rust `.so` are libc / libgcc-unwind imports
(`malloc`, `memcpy`, `strlen`, `write`, `_Unwind_*`, `__tls_get_addr`, …).
**0 missing/undefined non-libc symbols.** The C `.so` imports
`memchr`, `sprintf`, `strcpy`, `strlen` — all libc.

## Artifacts verified

The symbol comparison was run for every Rust artifact and every C build used by
the suite:

| Rust artifact | C artifact | symbol diff |
|---------------|------------|-------------|
| `target/debug/libfindrep_lib.so` | `c_src/build/libtranslated_rust.so` (`-O0`) | empty |
| `target/release/libfindrep_lib.so` (`panic = "abort"`) | `c_src/build/libtranslated_rust.so` (`-O0`) | empty |
| both of the above | out-of-tree `CMAKE_BUILD_TYPE=Release` (`-O2`) C build | empty (the C `-O2` build exports the same 8 symbols) |

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/debug/libfindrep_lib.so   | awk '{print $3}' | sort)
# -> empty (exit 0)
```
