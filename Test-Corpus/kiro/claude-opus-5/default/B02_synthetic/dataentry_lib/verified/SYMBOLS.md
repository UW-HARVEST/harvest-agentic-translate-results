# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C   `.so`: `c_src/build/libharvest-work-CVzZrt.so` (built from `c_src/src/lib.c`)
- Rust `.so`: `translation/target/release/libdataentry_lib.so` (`crate-type = ["cdylib"]`)

## C public symbols (`nm -D --defined-only`)

| # | symbol | type | present in Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `dataentry` | `T` (global text) | YES (`T dataentry`) | `#[unsafe(no_mangle)] pub extern "C" fn dataentry` |

## Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only rust.so | awk '{print $3}' | sort -u)
<empty>
```

**0 missing symbols.** The diff is empty.

## Why the surface is exactly one symbol

`c_src/include/lib.h` declares exactly one prototype:

```c
int dataentry(int a, int b, int c, int d);
```

Every other function in `c_src/src/lib.c` is declared `static`
(`find_entry`, `process_name`, `calculate_lookup`, `create_entries`,
`modify_entries`) and so has internal linkage and no dynamic symbol. The
file-scope data (`lookup_table`) is `static` as well. There are no macros that
generate additional exported symbols, no `#ifdef`-gated extra sources, and
`CMakeLists.txt` compiles a single translation unit (`src/lib.c`).

Consequently the Rust crate keeps the five helpers as private `fn`s with the
same names and exports only `dataentry`. No module of C source was skipped:
`c_src/src/lib.c` (199 lines, one TU) is translated in full, and no stubs or
`unimplemented!()` are present.

```
$ grep -c 'unimplemented!\|todo!\|panic!("not' translation/src/lib.rs
0
```

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / runtime imports
(`malloc`, `free`, `memcpy`, `__libc_start_main`-family, unwinding hooks). No
non-libc undefined symbols.
