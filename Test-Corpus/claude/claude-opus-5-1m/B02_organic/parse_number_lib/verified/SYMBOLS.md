# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

Artifacts compared:

* C   : `c_src/build/libdriver.so`   (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust: `target/debug/libdriver.so`  (`cargo build`, `crate-type = ["cdylib", "rlib"]`)

## Build-time configurations

`Cargo.toml` has **no `[features]` section**, therefore the only valid feature
combination is the empty one (`--no-default-features`, which is identical to the
default build).

`c_src/CMakeLists.txt` declares no `option()`, no `target_compile_definitions`,
and no conditional sources — a single unconditional `add_library(driver SHARED src/lib.c)`.
There is therefore exactly **one** build configuration on each side.

| # | configuration | `cargo` invocation | status |
|---|---------------|--------------------|--------|
| 1 | default / no features | `cargo check --no-default-features` | OK |
| 2 | default (implicit)    | `cargo check`                      | OK |

## Exported (dynamic, defined) symbols

`nm -D --defined-only <so>`:

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|-----------|------|
| `parse_number` | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn parse_number` |

**C exports 1 symbol; Rust exports the same 1 symbol. Symbol diff is EMPTY.**

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/debug/libdriver.so  | awk '{print $NF}' | sort)
(no output)
```

## Undefined (imported) symbols

All undefined symbols on both sides resolve out of libc / libgcc:

* C: `free`, `malloc`, `memcpy`, `strtod` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).
* Rust: the same libc set plus the Rust `std` runtime's libc/`_Unwind_*` imports
  (`abort`, `calloc`, `realloc`, `memmove`, `memset`, `mmap64`, `dl_iterate_phdr`,
  `pthread_key_*`, …). Every one of them is a **libc / libgcc** symbol.

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**

## Translation completeness

`c_src` contains exactly one translation unit:

```
c_src/include/lib.h   38 lines   (types + the single public prototype)
c_src/src/lib.c      114 lines   (2 function-like macros + parse_number)
```

Everything in it is translated in `src/lib.rs`:

| C entity (`c_src`) | Rust counterpart (`src/lib.rs`) |
|--------------------|---------------------------------|
| `typedef int cJSON_bool`            | `pub type cJSON_bool = c_int` |
| `#define true` / `#define false`    | `CJSON_TRUE` / `CJSON_FALSE` |
| `#define INT_MIN` / `INT_MAX`       | `C_INT_MIN` / `C_INT_MAX` |
| `#define cJSON_Number (1 << 3)`     | `CJSON_NUMBER` |
| `struct parse_buffer`               | `#[repr(C)] pub struct parse_buffer` |
| `struct cJSON`                      | `#[repr(C)] pub struct cJSON` |
| `can_access_at_index(buffer, index)` | `fn can_access_at_index` |
| `buffer_at_offset(buffer)`           | `fn buffer_at_offset` |
| `strtod` (libc)                      | `unsafe extern "C" { fn strtod }` — the *same* libc symbol, so value rounding and end-pointer semantics are bit-identical by construction |
| `parse_number`                       | `#[unsafe(no_mangle)] pub unsafe extern "C" fn parse_number` |

No C source file, function, or macro is left untranslated; no Rust symbol is a
stub.

## Test surface

| file | tests | covers |
|------|-------|--------|
| `tests/common/mod.rs`      | — (harness) | `libloading` loaders for BOTH `.so`s, `Case`/`Outcome` model, read guard, poisoned out-params, SplitMix64 PRNG |
| `tests/abi_layout.rs`      | 4  | struct layout, both `.so`s loadable, `parse_number` exported by both, smoke test |
| `tests/valid_path.rs`      | 24 | Phase B — every `CONFIGS.md` row C1–C24, randomized |
| `tests/error_path.rs`      | 22 | Phase C — every `ERRORS.md` row E1–E20 + generic boundaries |
| `tests/exhaustive.rs`      | 10 | exhaustive enumeration (C26–C30) + equivalent-mutant justifications |
| `tests/heavy_fuzz.rs`      | 3  | C31 — all axes, 6 seeds × 20 000 cases, huge lengths, call sequences |
| `tests/null_item_crash.rs` | 2 (+1 helper) | E21/E21b — `item == NULL` fault parity, compared via child-process exit status/signal |
| `tests/misaligned.rs`      | 1 (+1 helper) | E22/C25 — misaligned `cJSON *` / `parse_buffer *` parity |
| **total**                  | **66** | |

Scripts:

* `./verify.sh`  — enumerates feature combos, builds the C `.so`, `cargo check`s
  every combo, diffs `nm -D`, and runs the suite for every combo × profile.
* `./mutants.sh` — injects 35 bugs into `src/lib.rs` and confirms the suite
  catches them (see `MUTANTS.md`).

## ABI layout parity (verified in `tests/abi_layout.rs`)

| type | C `sizeof` / offsets (x86-64 SysV) | Rust `size_of` / `offset_of` |
|------|-----------------------------------|------------------------------|
| `cJSON`        | 16; `type`@0, `valueint`@4, `valuedouble`@8 | 16; `type_`@0, `valueint`@4, `valuedouble`@8 |
| `parse_buffer` | 32; `content`@0, `length`@8, `offset`@16, `depth`@24 | 32; same |
| `cJSON_bool`   | 4 (`int`) | 4 (`c_int`) |
