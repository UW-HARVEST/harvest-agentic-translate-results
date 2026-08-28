# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-dfq9xw.so   (name = parent dir name, per CMakeLists.txt)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libhex2bin_lib.so   ([lib] name = "hex2bin_lib", cdylib)
```

## C `.so` — defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | type | source of definition |
|---|--------|------|----------------------|
| 1 | `hex2bin` | `T` (global text) | `c_src/src/lib.c:5`, declared `c_src/include/lib.h:4` |

That is the complete list. `c_src/CMakeLists.txt` compiles exactly one
translation unit (`src/lib.c`), which defines exactly one non-static function.
There are no macro-generated symbols, no global/static data symbols, no
constructors/destructors and no versioned aliases.

## C `.so` — undefined symbols (`nm -D -u`)

| symbol | kind | note |
|--------|------|------|
| `strchr@GLIBC_2.2.5` | `U` libc | used by the `ignore` handling |
| `_ITM_deregisterTMCloneTable` | `w` weak toolchain | not a library symbol |
| `_ITM_registerTMCloneTable` | `w` weak toolchain | not a library symbol |
| `__cxa_finalize@GLIBC_2.2.5` | `w` weak libc | not a library symbol |
| `__gmon_start__` | `w` weak toolchain | not a library symbol |

All of these are libc/toolchain, so none needs a Rust counterpart. `strchr` is
reimplemented inside the Rust crate as the private helper `strchr_found`
(`translation/src/lib.rs`), preserving the C-standard behaviour that the
terminating NUL of the search string is part of the string.

## Rust `.so` — defined dynamic symbols

| # | symbol | type | source |
|---|--------|------|--------|
| 1 | `hex2bin` | `T` (global text) | `translation/src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn hex2bin` |

## Symbol diff — MUST be empty

```sh
comm -23 <(nm -D --defined-only c_src/build/libharvest-work-dfq9xw.so   | awk '{print $3}' | sort -u) \
         <(nm -D --defined-only translation/target/release/libhex2bin_lib.so | awk '{print $3}' | sort -u)
```

Result: **empty** — every symbol exported by the C `.so` is exported by the Rust
`.so` under the exact same name.

* Missing symbols whose implementation exists but is unexported: **none**.
* Missing symbols whose C source was never translated: **none** — `src/lib.c` is
  the only C source file and its single function is fully translated (no stubs,
  no `unimplemented!()` anywhere in the crate).

Undefined non-libc symbols in the Rust `.so`: **none** (only the standard
`libc`/`libgcc_s`/`ld-linux` imports that any Rust cdylib pulls in).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the only
possible feature configuration is the default (empty) one. The automation loop in
`check_features.sh` enumerates the feature list from `Cargo.toml` and confirms
that `--no-default-features` and the default build are the only two
configurations, and that both compile and pass the full test suite.
