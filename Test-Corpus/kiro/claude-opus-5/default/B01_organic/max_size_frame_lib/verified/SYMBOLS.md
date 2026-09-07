# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

## Build commands

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-zlMwzp.so

cd translation && cargo build --release
# -> translation/target/release/libmax_size_frame_lib.so
```

## C source inventory (completeness check)

The CMake target compiles exactly one translation unit:

* `c_src/src/lib.c` (10 lines) — defines `max_size_frame`
* `c_src/include/lib.h` (5 lines) — `typedef uint32_t tflac_u32;` + one prototype

There are no other `.c` files, no macro-generated symbol families, and no
`#ifdef`-gated alternate definitions. So no C module was skipped by the
translation; the Rust crate's single function is the whole library.

## Symbol table

| # | C symbol (`nm -D`) | type | exported by Rust `.so`? | notes |
|---|--------------------|------|-------------------------|-------|
| 1 | `max_size_frame`   | `T` (global text) | YES (`T`) | `#[unsafe(no_mangle)] pub extern "C" fn` in `translation/src/lib.rs` |

### Symbols exported by C but missing from Rust

**NONE.**

### Undefined (imported) non-libc symbols in the Rust `.so`

**NONE.** The Rust `.so`'s undefined symbols are only the usual
`libc`/`libgcc_s`/`ld-linux` runtime entries (`memcpy`, `__cxa_thread_atexit_impl`,
`_Unwind_Resume`, ...) pulled in by the Rust runtime, plus the standard
`_ITM_*`/`__gmon_start__` weak optional symbols. No library-specific symbol is
left unresolved.

### Extra symbols exported by Rust (not in C)

**NONE.** `nm -D --defined-only` on the Rust `.so` yields exactly one line:
`T max_size_frame`. The dynamic symbol sets are therefore *identical*, not
merely a superset.

## Gate status

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` (C set is a subset of Rust set).
- [x] `nm -D` shows 0 undefined/unresolved non-libc symbols in the Rust `.so`.
- [x] No `unimplemented!()` / stub / fake implementation was introduced.

## Verification evidence

`nm -D --defined-only --format=posix` on both objects, sorted and `diff`ed:

```
$ diff <(nm -D --defined-only --format=posix c_src/build/libharvest-work-zlMwzp.so | awk '{print $1}' | sort) \
       <(nm -D --defined-only --format=posix translation/target/release/libmax_size_frame_lib.so | awk '{print $1}' | sort)
# (no output — sets are identical)
```

Each side defines exactly one dynamic symbol:

```
C    : T max_size_frame
Rust : T max_size_frame
```

The Rust `.so`'s undefined symbols were also enumerated; all 49 are
`glibc`/`libgcc` imports (`memcpy@GLIBC_2.14`, `_Unwind_Resume@GCC_3.0`,
`malloc`, `pthread_key_create`, ...) plus the weak-optional
`__gmon_start__` / `_ITM_*TMCloneTable` trio. Zero library-specific symbols are
unresolved.

This diff is additionally asserted from inside the test suite by
`symbol_parity_nm_diff`, and re-checked by `translation/verify_all.sh`.

### Codegen cross-check

Disassembly of both definitions confirms the same computation. The C (`-O0`)
form is a literal stack-based transcription; the Rust (`-O3`) form is the
branchless `cmov` version of the same expression:

```
C    : ... imul / imul / add $0x7 / shr $0x3 / add / add $0x12 / ret
Rust : ... imul / imul / cmov / add / shr $0x3 / lea / add $0x12 / ret
```

Both end in `shr $0x3` (unsigned divide by 8) and `add $0x12` (the `18U`
constant), with `imul` for the wrapping multiplies. The Rust `.so` contains no
call instruction and no panic landing pad in this function, so it cannot abort
where the C wraps.
