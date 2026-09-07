# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libSieve.so

cd translation && cargo build --release
# -> translation/target/release/libSieve.so
```

## C source inventory (completeness check)

The whole C library is two files; every C source file is accounted for in the
Rust crate, so no module was skipped:

| C file | contents | Rust counterpart |
|--------|----------|------------------|
| `c_src/include/sieve.h` | declares `void sieve(int start);` | `translation/src/lib.rs` (`pub extern "C" fn sieve`) |
| `c_src/src/sieve.c` | defines `sieve` (the only function, 9 lines of body) | `translation/src/lib.rs` |

`grep -c '^[a-zA-Z].*(' c_src/src/sieve.c` yields exactly one function
definition (`sieve`); there are no `static` helpers, no macros that generate
symbols, no global variables, and no other translation units.

## Exported (defined, dynamic) symbols

### C: `nm -D --defined-only c_src/build/libSieve.so`

```
0000000000001109 T sieve
```

### Rust: `nm -D --defined-only translation/target/release/libSieve.so`

```
00000000000116e0 T sieve
```

### Parity table

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|-----------|--------------|--------|
| 1 | `sieve` | `T` (text, global) | yes | yes | **MATCH** — real translation, not a stub |

**Missing from Rust: none.** The symbol diff is empty:

```
$ diff <(nm -D --defined-only c_src/build/libSieve.so         | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libSieve.so | awk '{print $NF}' | sort)
$ # (no output)
```

## Undefined (imported) symbols

The C library imports one non-weak libc symbol:

```
U printf@GLIBC_2.2.5
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

The Rust library imports `printf@GLIBC_2.2.5` as well (the translation
deliberately calls the platform `printf` so stdout buffering and the exact byte
stream match), plus the usual Rust `std`/`libunwind` runtime imports
(`_Unwind_*`, `malloc`, `memcpy`, `pthread_key_create`, `dl_iterate_phdr`, ...).

**0 missing/undefined non-libc symbols in the Rust `.so`** — every undefined
symbol in the Rust `.so` is provided by glibc / libgcc_s, which are linked at
load time. Verified by `ldd -r`:

```
$ ldd -r translation/target/release/libSieve.so   # no "undefined symbol" lines
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default (empty) feature set. `cargo check
--no-default-features` and `cargo check` compile the identical code, therefore
the "every feature combination" requirement collapses to a single combination
here. This is verified by `check_features.sh`.

## Binary / driver targets

Neither project builds an executable:

* `c_src/CMakeLists.txt` contains only `add_library(Sieve SHARED src/sieve.c)`
  — no `add_executable`.
* `translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` —
  there is no `src/main.rs` and no `[[bin]]` section.

So the "compare C and Rust binary stdout" gate is not applicable; instead, the
stdout produced *through the FFI boundary* by each `.so` is compared
byte-for-byte in Phase B (the library's entire observable output is its stdout).
