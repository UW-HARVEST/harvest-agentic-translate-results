# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

## Build configuration surface

`translated_rust/Cargo.toml` has **no `[features]` section at all**, therefore:

* the set of valid feature combinations is exactly **one**: the empty set
  (`cargo check --no-default-features`, which is identical to the default build).
* `c_src/CMakeLists.txt` defines **no** `target_compile_definitions`, no
  `option()`, no `#ifdef`-driven configuration and compiles exactly one
  translation unit (`src/lib.c`). There is no `-D` build-time switch either.
* Consequently Phase B/C only has to be repeated for one configuration, but the
  loop script `run_all_features.sh` is provided and still iterates over the
  enumerated (single) combination so the process is mechanical.

| # | combination | `cargo check` |
|---|-------------|---------------|
| 1 | `--no-default-features` (== default, no features exist) | PASS |

## C `.so` exported symbols (`nm -D --defined-only`)

Source of truth: `c_src/build/libtranslated_rust.so`
Rust `.so`: `target/{debug,release}/libgotomach_lib.so`

| # | symbol | type | in C `.so` | in Rust `.so` | note |
|---|--------|------|-----------|---------------|------|
| 1 | `process_value`  | `T` (global text) | yes | yes | `#[no_mangle] extern "C"` |
| 2 | `double_value`   | `T` | yes | yes | `#[no_mangle] extern "C"` |
| 3 | `triple_value`   | `T` | yes | yes | `#[no_mangle] extern "C"` |
| 4 | `gotomach`       | `T` | yes | yes | `#[no_mangle] extern "C"`, the header API |

**Missing from Rust `.so`: 0.**

### Deliberately NOT exported (must stay private, `static` in C)

These are `static` in `lib.c`, so they are *absent* from the C `.so` dynamic
symbol table. The Rust translation keeps them private too (no `#[no_mangle]`),
which is the correct parity behaviour:

| symbol | why not exported |
|--------|------------------|
| `is_valid_state`    | `static bool is_valid_state(ProcessorState*)` |
| `check_char_flag`   | `static bool check_char_flag(char)` |
| `init_processor`    | `static ProcessorState* init_processor(size_t, operation_fn)` |
| `cleanup_processor` | `static void cleanup_processor(ProcessorState*)` |

Macros `MAKE_FUNC_NAME`, `LOG_MSG`, `CREATE_LABEL` generate no symbols
(`MAKE_FUNC_NAME`/`CREATE_LABEL` are never expanded in the C source; `LOG_MSG`
expands to a `printf` call, and is modelled by the `log_msg!` Rust macro).

## Undefined-symbol audit

`nm -D --undefined-only`:

* C `.so`: `free`, `malloc`, `puts` (+ weak `_ITM_*`, `__cxa_finalize`,
  `__gmon_start__`).
  Note gcc rewrote `printf("literal\n")` into `puts("literal")`.
* Rust `.so`: `free`, `malloc`, `puts`, plus the Rust `std`/`libunwind`
  runtime imports (`memcpy`, `mmap64`, `pthread_key_create`, `_Unwind_*`, …).
  LLVM performs the same `printf` → `puts` simplification in release mode.

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.**

## Verification command

```sh
./symbol_parity.sh          # prints the diff; exits non-zero if any C symbol is missing
```

## Phase D result

```
$ ./symbol_parity.sh
C    .so: c_src/build/libtranslated_rust.so    (4 exported symbols)
Rust .so: target/debug/libgotomach_lib.so (4 exported symbols)

--- symbols in C but MISSING from Rust ---
<none>

--- symbols only in Rust (extra, allowed) ---
<none>

--- non-libc undefined symbols in Rust .so ---
<none>

PASS: symbol parity complete (4/4 C symbols present in Rust, 0 unexpected undefined)
```

Verified for BOTH `target/debug/libgotomach_lib.so` and
`target/release/libgotomach_lib.so`. The script is guarded against vacuous
passes (it fails if either symbol list comes back empty) and was validated with
a negative control (`RUST_SO_PATH=/lib64/libz.so.1 ./symbol_parity.sh` → exit 1).

`tests/phase_d_symbols.rs` re-checks the same thing from Rust:

| test | what it proves |
|------|----------------|
| `d0_two_distinct_libraries_are_loaded` | anti-vacuity: the two `.so`s are different files AND every symbol resolves to a *different* address in each, so the differential tests really compare two implementations |
| `d1_every_c_symbol_is_exported_by_rust` | `nm -D` set difference (C → Rust) is empty |
| `d2_expected_symbol_set` | the C export set is exactly the 4 documented symbols, and the four `static` helpers are NOT exported by Rust either |
| `d3_all_symbols_resolvable_via_dlsym_in_both` | every symbol is `dlsym`-able in both and behaves the same |
| `d4_rust_has_no_non_libc_undefined_symbols` | no dangling imports in the Rust `.so` |
| `d5_no_heap_growth_in_either_implementation` | allocation parity: 1 200 `gotomach` calls per impl grow `mallinfo2().uordblks` by < 1 MiB (validated by deleting `free(temp_buffer)` — the test then reported a 108 MB leak) |

## Completion gate

| requirement | status |
|-------------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 non-libc undefined in Rust | PASS (4/4, debug + release) |
| Phase B: every row of `CONFIGS.md` (51) passes across randomized inputs | PASS (51/51) |
| Phase C: every row of `ERRORS.md` (17 + G1–G7) has a passing differential test | PASS (14 tests covering all rows) |
| Holds under EVERY feature combination | PASS — the crate has **no** `[features]`, so there is exactly 1 combination; `run_all_features.sh` enumerates it mechanically and additionally re-runs the whole suite against the optimised **release** `.so` |

Reproduce everything with:

```sh
./run_all_features.sh     # cargo check + build + symbol parity + Phase B/C/D, debug and release
```
