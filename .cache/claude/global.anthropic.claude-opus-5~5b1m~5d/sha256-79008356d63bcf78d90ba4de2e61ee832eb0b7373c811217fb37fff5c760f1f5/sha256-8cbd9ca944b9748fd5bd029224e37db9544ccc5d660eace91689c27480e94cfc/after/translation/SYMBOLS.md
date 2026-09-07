# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## Exported (defined, dynamic) symbols

| # | symbol | C `libdriver.so` | Rust `libdriver.so` | notes |
|---|--------|------------------|---------------------|-------|
| 1 | `parse_number` | `T` (0x1139) | `T` (0x11890) | `cJSON_bool parse_number(cJSON *const, parse_buffer *const)` — declared in `c_src/include/lib.h:38`, defined in `c_src/src/lib.c:13`. Rust: `#[unsafe(no_mangle)] pub unsafe extern "C" fn parse_number` in `src/lib.rs`. |

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(no output — identical)
```

**MISSING FROM RUST: none.** The C translation unit is a single file
(`src/lib.c`) containing exactly one non-static function; there are no
macro-generated exports, no `#ifdef`-gated extra entry points, and no other
compilation units in `c_src/CMakeLists.txt` (`add_library(driver SHARED src/lib.c)`).
Nothing was skipped by the translation.

### Non-exported C constructs (nothing to export)

| C construct | kind | Rust counterpart |
|---|---|---|
| `can_access_at_index(buffer, index)` | function-like macro (`src/lib.c:8`) | `unsafe fn can_access_at_index` (private `#[inline]`) — correctly NOT exported |
| `buffer_at_offset(buffer)` | function-like macro (`src/lib.c:10`) | `unsafe fn buffer_at_offset` (private `#[inline]`) — correctly NOT exported |
| `cJSON_bool` | `typedef int` | `pub type cJSON_bool = c_int` |
| `true` / `false` | `#define ((cJSON_bool)1/0)` | `CJSON_TRUE` / `CJSON_FALSE` consts |
| `INT_MIN` / `INT_MAX` | `#define` | `INT_MIN` / `INT_MAX` consts |
| `cJSON_Number` | `#define (1 << 3)` = 8 | `cJSON_Number` const |
| `parse_buffer` | struct (4 fields) | `#[repr(C)] pub struct parse_buffer` |
| `cJSON` | struct (`type`, `valueint`, `valuedouble`) | `#[repr(C)] pub struct cJSON` (`type` → `type_`) |

### ABI layout parity (verified in `tests/differential.rs::layout_parity`)

| type | size | align | field offsets |
|---|---|---|---|
| `parse_buffer` | 32 | 8 | content 0, length 8, offset 16, depth 24 |
| `cJSON` | 16 | 8 | type 0, valueint 4, valuedouble 8 |

### Undefined (imported) symbols

C imports: `free`, `malloc`, `memcpy`, `strtod` (plus weak `_ITM_*`,
`__cxa_finalize`, `__gmon_start__`).

Rust imports the same four libc functions (`malloc`, `free`, `strtod`,
`memcpy`) and additionally pulls in the Rust `std` runtime / unwinder
(`_Unwind_*`, `pthread_key_*`, `dl_iterate_phdr`, …). **0 missing/undefined
non-libc symbols**: every `U` entry in the Rust `.so` resolves against
`libc`/`libgcc_s`, none is an unresolved project symbol.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are therefore equivalent, and both are exercised (see
`run_all.sh`). There are also no `#ifdef`/`#if` conditionals in `c_src`
(only the unconditional `#ifdef true` / `#ifdef false` redefinition guards in
`lib.h`), so the C side likewise has a single configuration.

## Harness note (important)

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact. Verified
empirically: editing `src/lib.rs` and running `cargo test` left
`target/release/libdriver.so` byte-identical (same md5), so a naive harness that
loads `target/<profile>/libdriver.so` silently validates STALE code. Two
deliberately injected bugs (`cJSON_Number = 9`, and advancing `offset` by
`number_string_length` instead of `after_end - number_c_string`) went
**undetected** under that setup.

`tests/common/mod.rs::build_rust_so()` therefore shells out to
`cargo build --release --lib --target-dir target/ffi-harness` before loading, and
loads `target/ffi-harness/release/libdriver.so`. A separate `--target-dir` is
required to avoid deadlocking on the build lock held by the enclosing
`cargo test`. After this fix both injected bugs are caught (and a third,
dropping `b'E'` from the scanner's match arms), so the suite is
negative-controlled.
