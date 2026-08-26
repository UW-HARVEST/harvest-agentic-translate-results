# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on the built shared libraries.

* C library:    `c_src/build/libStaticLoop.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust library: `target/release/libStaticLoop.so`
  (built with `cargo build --release`)

## Whole-library inventory

The C build (`c_src/CMakeLists.txt`) compiles exactly one translation unit into
one shared library:

```
add_library(StaticLoop SHARED src/staticloop.c)
```

`c_src/` contains only:

| file | role |
|------|------|
| `c_src/CMakeLists.txt`      | build definition (1 target, 1 source file) |
| `c_src/include/staticloop.h`| public header: `static_sum`, `driver` |
| `c_src/src/staticloop.c`    | the only implementation file (43 lines) |

There is no second module, no globbing, and no macro-based symbol renaming in
the header, so the full public surface is the two functions below. Nothing in
the C sources is left untranslated: `src/lib.rs` covers 100 % of
`c_src/src/staticloop.c`.

## Defined (exported) dynamic symbols

`nm -D --defined-only <lib> | sort`

| # | C symbol | C type | Rust `.so` exports it? | Rust definition |
|---|----------|--------|------------------------|-----------------|
| 1 | `static_sum` | `T` (global text) | YES | `src/lib.rs` — `#[unsafe(no_mangle)] pub extern "C" fn static_sum(update: c_int) -> c_int` |
| 2 | `driver`     | `T` (global text) | YES | `src/lib.rs` — `#[unsafe(no_mangle)] pub extern "C" fn driver(stride: c_int)` |

Raw output:

```
$ nm -D --defined-only c_src/build/libStaticLoop.so | sort
0000000000001119 T static_sum
0000000000001139 T driver

$ nm -D --defined-only target/release/libStaticLoop.so | sort
0000000000011ce0 T driver
0000000000011e00 T static_sum
```

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libStaticLoop.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/release/libStaticLoop.so | awk '{print $NF}' | sort)
(no output)
```

**MISSING FROM RUST: none. The diff is EMPTY.**

Nothing had to be added or newly translated for symbol parity: both C exports
already existed in Rust with `#[unsafe(no_mangle)] extern "C"` wrappers, and no
symbol is stubbed / `unimplemented!()`.

## Non-exported / linker-generated symbols (informational)

The C `.so` additionally lists these, none of which are library API — they are
produced by the toolchain (weak GCC/ITM hooks) or are libc imports:

```
U printf@GLIBC_2.2.5
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

The function-local `static int sum` inside `static_sum` is compiled to the
*local* data symbol `sum.0` (visible in `nm` but **not** in `nm -D`), so it is
not part of the ABI. The Rust translation mirrors it with a private
`static mut SUM: c_int = 0;`.

## Undefined symbols in the Rust `.so`

`nm -D -u target/release/libStaticLoop.so` lists only libc / libgcc-unwind
imports (`printf`, `memcpy`, `malloc`, `free`, `_Unwind_*`, `pthread_key_*`,
`dl_iterate_phdr`, …) plus the same weak toolchain hooks as the C library.

**0 missing / undefined non-libc symbols.** ✅

## Feature-combination note

`Cargo.toml` declares **no `[features]` table**, and `c_src/CMakeLists.txt`
declares **no options / `#ifdef` switches** (there is not a single `#if` or
`#ifdef` in `c_src/src/staticloop.c` beyond the header include guard). The
complete set of valid build configurations is therefore the single empty
feature set — see `CONFIGS.md` for the enumeration and the commands used to
verify it.
