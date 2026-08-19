# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Generated mechanically from `nm -D` on both shared objects.

* C   `.so`: `c_src/build/libString_Slice.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/{debug,release}/libString_Slice.so`
  (`[lib] name = "String_Slice"`, `crate-type = ["cdylib"]`)

Reproduce with:

```sh
./check_symbols.sh
```

## Defined (exported) dynamic symbols

`nm -D --defined-only --format=posix <so> | awk '{print $1}' | sort -u`

| # | C symbol | type in C `.so` | present in Rust `.so` | notes |
|---|----------|-----------------|-----------------------|-------|
| 1 | `slice`  | `T` (global text) | YES (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn slice(...)` in `src/lib.rs` |

**Missing symbols: 0.** The C library consists of exactly one translation unit
(`c_src/src/slicing.c`) that declares exactly one non-static function, and
`c_src/include/slicing.h` declares exactly that one prototype:

```c
int slice(char *mystr, int *start_ptr, int *stop_ptr);
```

There are no macro-generated symbols, no exported data objects, no `static`
helpers promoted to externals, and no additional `.c` files in
`c_src/CMakeLists.txt` (`add_library(String_Slice SHARED src/slicing.c)`), so no
C source was left untranslated. Nothing needed to be stubbed.

Observed in both the `debug` and the `release` profile: the Rust `.so` exports
exactly **one** defined dynamic symbol, `slice`, i.e. the symbol sets are not
merely a superset relationship but identical (`comm -13` is empty too). Extra
Rust-runtime exports would not have been a parity failure — the gate is that
every C symbol exists in Rust under the exact same name — but there are none.

## Undefined (imported) symbols — non-libc check

| `.so` | undefined symbols |
|-------|-------------------|
| C     | `printf`, `puts`, `strlen` (all `GLIBC_2.2.5`) + weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__` |
| Rust  | `printf`, `puts`, `strlen` + `_Unwind_*` (libgcc), and libc/std runtime imports (`malloc`, `free`, `memcpy`, `write`, `mmap64`, `pthread_key_create`, …) |

0 undefined **non-libc / non-runtime** symbols in the Rust `.so`. In particular
the Rust `.so` imports the *same* three C-library primitives the C `.so` does.

Note: the C compiler rewrote the three argument-less `printf("...\n")` error
messages into `puts("...")` (a standard GCC optimisation), which is why `puts`
appears in the C import list. `puts` emits exactly the same bytes. The Rust
translation calls `printf` for all three messages; the emitted bytes are still
identical, which the Phase B/C differential tests verify by capturing fd 1 —
including the case where the write itself fails (`ERRORS.md` row 26).

`ldd -r` on the Rust `.so` reports no unresolvable imports (see
`./check_symbols.sh`), i.e. every import is satisfied by libc/libgcc.

## Signature parity

| symbol | C prototype | Rust `extern "C"` signature |
|--------|-------------|-----------------------------|
| `slice` | `int slice(char *mystr, int *start_ptr, int *stop_ptr)` | `unsafe extern "C" fn slice(mystr: *mut c_char, start_ptr: *mut c_int, stop_ptr: *mut c_int) -> c_int` |

## Verification status

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` (1/1 C symbols present).
- [x] 0 undefined non-libc symbols in the Rust `.so`.
