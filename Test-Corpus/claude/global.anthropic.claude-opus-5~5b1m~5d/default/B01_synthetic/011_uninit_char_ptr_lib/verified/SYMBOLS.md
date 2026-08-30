# Phase A.1 — Symbol surface (`nm -D`)

## Source inventory

`c_src/CMakeLists.txt` builds exactly one translation unit into `libdriver.so`:

```
add_library(driver SHARED src/driver.c)
```

Headers: `c_src/include/driver.h`, which declares only `void driver(int useGood);`.

`c_src/src/driver.c` defines four **non-static, default-visibility** functions:

| C definition                        | header-declared? | exported? |
|-------------------------------------|------------------|-----------|
| `void printLine(const char *line)`  | no               | yes       |
| `void bad(void)`                    | no               | yes       |
| `void good(void)`                   | no               | yes       |
| `void driver(int useGood)`          | yes              | yes       |

There is no second module, no macro-generated symbol family, no `#ifdef`
compile-time configuration (the preprocessor is used only for the `DRIVER_H_`
include guard) and no data symbol. The whole library is 59 lines, all of it
translated — nothing was skipped, stubbed, or left as `unimplemented!()`.

## `nm -D --defined-only` comparison

C — `c_src/build/libdriver.so`:

```
000000000000115b T bad
0000000000001194 T driver
0000000000001172 T good
0000000000001139 T printLine
```

Rust — `translation/target/release/libdriver.so`:

```
00000000000116dc T bad
00000000000116f4 T driver
0000000000011720 T good
0000000000011744 T printLine
```

## Parity table

| # | symbol      | type | in C `.so` | in Rust `.so` | Rust implementation | status |
|---|-------------|------|-----------|---------------|---------------------|--------|
| 1 | `printLine` | `T` (func) | yes | yes | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn printLine` | OK |
| 2 | `bad`       | `T` (func) | yes | yes | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn bad` | OK |
| 3 | `good`      | `T` (func) | yes | yes | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn good` | OK |
| 4 | `driver`    | `T` (func) | yes | yes | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn driver` | OK |

**Missing from Rust: none — the symbol diff is empty.** The Rust `.so` exports
nothing extra either, apart from the loader boilerplate (`_init`, `_fini`,
`__bss_start`, `_edata`, `_end`) that every shared object carries.

## Undefined (imported) symbols

C `.so` imports exactly one real symbol: **`puts`** — `gcc` rewrites
`printf("%s\n", line)` in `printLine` into a `puts` call — plus the weak
`__cxa_finalize`, `_ITM_*` and `__gmon_start__` loader hooks.

The Rust `.so` also imports **`puts`** (the translation deliberately calls the
same libc routine, so stdio buffering, interleaving *and* the stack residue the
routine leaves behind are identical). Its remaining imports are the ordinary libc
and `_Unwind_*` symbols that `libstd`/`compiler_builtins` need — `malloc`,
`memcpy`, `write`, … — all of which resolve against `libc.so.6`/`libgcc_s.so.1`.

**0 missing or unresolvable non-libc undefined symbols**, verified with `ldd -r`
by `tests/symbols.rs::rust_so_has_no_unresolved_symbols`.

## Automated checks (`tests/symbols.rs`)

| test | checks |
|------|--------|
| `symbol_parity_c_subset_of_rust` | every C export exists in the Rust `.so`; the diff must be empty |
| `symbol_surface_is_exactly_the_four_functions` | the C surface is exactly those four, and the Rust adds nothing that looks like API |
| `rust_so_has_no_unresolved_symbols` | `ldd -r` reports no undefined symbol |
| `both_import_the_same_stdio_routine` | both `.so`s import `puts` |
| `link_configuration_matches_c` | neither `.so` uses `BIND_NOW` (see below) |
| `asm_frame_layout_matches_c` | the normalised instruction stream of all four functions is identical to gcc's |
| `all_symbols_resolve_via_dlsym` | all four are reachable through `dlsym` from an external caller |
| `test_harness_runs_single_threaded` | guards the fd-1 stdout-capture mechanism |

## Non-symbol ABI surface that also had to match

Symbol parity turned out to be necessary but **not** sufficient. `bad()`
reproduces the original's CWE-457 read of an uninitialised local, so it prints
whatever bytes happen to sit just below its own frame — which makes two
otherwise-invisible link/codegen properties observable. Both were found by the
differential tests and fixed:

1. **Frame layout.** `bad()`'s indeterminate slot is at `entry_rsp - 16`, and its
   contents are decided by the frame sizes and local spills of `driver`, `good`
   and `printLine`. Idiomatic Rust codegen puts different things there, so all
   four bodies are emitted as `naked` functions reproducing gcc's `-O0` output
   (see the module docs in `src/lib.rs`). Pinned by `asm_frame_layout_matches_c`;
   without it, `good(); bad();` prints `string\nstring\n` in C but `string\n\n`
   in Rust.
2. **Lazy vs eager PLT binding.** `rustc` defaults to `-z now`; `gcc`, as invoked
   by `c_src/CMakeLists.txt`, does not. With `-z now` the dynamic linker's
   `_dl_runtime_resolve` never runs, and that routine's stack residue is exactly
   what `driver(0)` observes. `build.rs` therefore requests `-Wl,-z,lazy`.
   Pinned by `link_configuration_matches_c`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the complete set
of build configurations is:

| combo | command |
|-------|---------|
| default (= empty feature set) | `cargo test` |
| `--no-default-features`       | `cargo test --no-default-features` |
| `--all-features`              | `cargo test --all-features` |

All three select identical code, but all three are still exercised — against both
the `dev` and the `release` profile — by `./verify.sh`, which derives the
combination list from `Cargo.toml` rather than hard-coding it.
