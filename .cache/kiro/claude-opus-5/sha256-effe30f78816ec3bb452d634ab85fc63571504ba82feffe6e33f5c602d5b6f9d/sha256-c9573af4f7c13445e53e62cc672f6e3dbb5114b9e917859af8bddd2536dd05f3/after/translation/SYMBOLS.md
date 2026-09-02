# SYMBOLS.md — Exported-symbol parity between the C `.so` and the Rust `.so`

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C shared library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort > /tmp/c_syms.txt

# Rust shared library
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort > /tmp/rust_syms.txt

comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # in C, missing from Rust
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # in Rust, absent from C
```

## Translation-unit inventory

The whole library is a single translation unit, so there is no possibility of a
"whole module never translated" gap:

| C source file | contains | translated in |
|---|---|---|
| `c_src/src/driver.c` | `printLine`, `driver` | `translation/src/lib.rs` |
| `c_src/include/driver.h` | declaration of `driver` only (no types, no macros, no enums) | n/a (header has no definitions) |

`grep -nE '^[A-Za-z_].*\(' c_src/src/driver.c` yields exactly two function
definitions; both are non-`static`, so both have external linkage and both must
appear in the `.so`.

## Defined (exported) dynamic symbols

| # | symbol | in C `.so` | in Rust `.so` | Rust definition | status |
|---|--------|-----------|---------------|-----------------|--------|
| 1 | `driver`    | yes (`T`) | yes (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(data: c_int)` | MATCH |
| 2 | `printLine` | yes (`T`) | yes (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine(line: *const c_char)` | MATCH |

`printLine` is absent from the public header but is **not** `static` in
`driver.c`, so it is part of the exported ABI and is exported by the Rust side
too. No symbol required a new wrapper and no C source was found untranslated,
so neither Phase A remediation rule (add-export / translate-missing-module) had
to be applied.

### Symbol diff

```
$ comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing from Rust
(empty)
```

**0 missing symbols.** The Rust `.so` additionally exports Rust-runtime
housekeeping symbols (`rust_eh_personality` and friends are not exported in
this build; the extra dynamic entries are only the standard
`_init`/`_fini`-class entries injected by the linker), which is expected and
harmless — the requirement is one-directional (C ⊆ Rust).

## Undefined (imported) dynamic symbols

Both objects import the same libc primitives for the core work. Note that
`printf("%s\n", line)` is lowered to `puts(line)` by **both** GCC (C build) and
LLVM (Rust release build), so `printf` appears in neither import list:

| symbol | C `.so` | Rust `.so` |
|---|---|---|
| `memset@GLIBC_2.2.5`  | yes | yes |
| `strncpy@GLIBC_2.2.5` | yes | yes |
| `puts@GLIBC_2.2.5`    | yes | yes |

The Rust `.so` imports an additional set of libc / libgcc symbols
(`malloc`, `free`, `_Unwind_*`, `__cxa_thread_atexit_impl`, `dl_iterate_phdr`,
`pthread_key_*`, …) that come from the statically linked Rust `std` runtime.
These are all resolved by the platform's `libc`/`libgcc_s`, i.e. **0 unresolved
non-libc undefined symbols**:

```
$ nm -D --undefined-only translation/target/release/libdriver.so \
    | awk '{print $2}' | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$'
(empty)
```

## Result

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name.
- [x] `nm -D` shows 0 missing symbols and 0 unresolved non-libc undefined
      symbols in the Rust `.so`.
