# SYMBOLS.md — public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # -> c_src/build/libdriver.so
cd translation && cargo build --release                              # -> translation/target/release/libdriver.so
cd translation && cargo build                                        # -> translation/target/debug/libdriver.so
```

## C source inventory (completeness check)

The whole library is one translation unit; nothing is conditionally compiled.

| C file | public declarations | translated in Rust? |
|---|---|---|
| `c_src/include/lib.h` | `char *searchAndReplace(const char*, const char*, const char*)` | yes — `src/lib.rs` |
| `c_src/src/lib.c` | `searchAndReplace` (only non-static definition in the file) | yes — `src/lib.rs` |

`c_src/CMakeLists.txt` lists exactly one source file (`src/lib.c`), defines no
`target_compile_definitions`, and the source contains no `#ifdef`-gated
alternative implementations, so there is no untranslated module and no
macro-generated / namespaced symbol variants to reproduce.

## Defined dynamic symbols (`nm -D --defined-only`)

| symbol | C `.so` | Rust `.so` (release) | Rust `.so` (debug) | status |
|---|---|---|---|---|
| `searchAndReplace` | `T` | `T` | `T` | present in both — OK |

Symbol diff (`comm -23` of the C defined-symbol name list against the Rust
defined-symbol name list): **empty**. No symbol had to be added and no C source
had to be newly translated.

The Rust `.so` additionally exports nothing of its own: its defined dynamic
symbol set is exactly `{searchAndReplace}`, i.e. it is neither missing nor
over-exporting relative to the C `.so`.

## Undefined dynamic symbols (`nm -D --undefined-only`)

* C `.so`: `malloc`, `realloc`, `strdup`, `strlen`, `strncpy`, `strstr`
  (+ the usual weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__` stubs).
* Rust `.so`: `malloc`, `realloc`, `strdup`, `strlen` plus the Rust `std`
  runtime's own libc / `libgcc_s` imports (`memcpy`, `memmove`, `memset`,
  `bcmp`, `calloc`, `free`, `posix_memalign`, `abort`, `__errno_location`,
  `pthread_key_*`, `dl_iterate_phdr`, `_Unwind_*`, file/`stat` syscall
  wrappers used by panic backtraces, …).

**0 missing / undefined non-libc symbols in the Rust `.so`**: every undefined
entry resolves against `libc.so.6` or `libgcc_s.so.1`, both of which
`ldd translation/target/release/libdriver.so` shows as satisfied. The Rust
build does not import `strstr`/`strncpy` because those two are open-coded
(`c_strstr` / `c_strncpy`) — that is an implementation detail, not a missing
export.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the complete set
of feature combinations is a single one: the default (empty) feature set.
`scripts/check_features.sh` enumerates the features from `Cargo.toml` and runs
`cargo check` / `cargo test` for every combination (default,
`--no-default-features`) to prove this mechanically rather than by assumption.
