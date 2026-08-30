# SYMBOLS.md — Exported symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source surface

The whole C library is a single translation unit, `c_src/src/driver.c`
(40 lines). It defines exactly two functions with external linkage:

| C definition | declared in header? |
|---|---|
| `int foo(const char *in, char c)` | no (not in `driver.h`, but non-`static` → exported) |
| `void driver(const char *in)` | yes (`c_src/include/driver.h`) |

There are no macros that generate symbols, no global variables, no `static`
helpers, and no other `.c` files in `CMakeLists.txt`. So the complete expected
export set is `{driver, foo}`.

## Defined dynamic symbols (`nm -D --defined-only`, code/data only)

| symbol | C `.so` | Rust `.so` | status |
|---|---|---|---|
| `driver` | `T` | `T` | ✅ present in both |
| `foo`    | `T` | `T` | ✅ present in both |

`foo` is *not* declared in `driver.h`, but because it is not `static` it is a
real public dynamic symbol of the C library; the Rust translation therefore
also exports it via `#[unsafe(no_mangle)] pub unsafe extern "C" fn foo`.

The Rust `.so` additionally exports the usual Rust/`cdylib` runtime symbols
(`_init`, `_fini`, `__bss_start`, `_edata`, `_end`, and Rust std internals).
Extra exports are harmless — the requirement is that every C symbol is present
in Rust, which holds.

### Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so           | awk '$2=="T"{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort)
(empty)
```

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C
module was left untranslated (there is only one C module and it is fully
translated: `foo` and `driver` in `translation/src/lib.rs`).

## Undefined (imported) symbols

C `.so` imports: `printf@GLIBC_2.2.5`, `strchr@GLIBC_2.2.5` (+ weak
`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Rust `.so` imports the same two libc functions — the translation deliberately
calls libc `strchr` and libc `printf` directly so that search semantics and
stdout formatting/buffering are byte-identical — plus the standard Rust
runtime imports (`malloc`/`free`/`memcpy`/`_Unwind_*`/`pthread_key_*`/…).

**Undefined non-libc symbols in the Rust `.so`: 0.** Every `U`/`w` entry
resolves to glibc (`GLIBC_*` versioned) or to the platform unwinder
(`GCC_*`), both of which are present at load time; `dlopen` of the Rust
`.so` succeeds in the differential tests, which is the empirical proof.

## Checklist

- [x] Every `T` symbol of the C `.so` is exported by the Rust `.so` with the
      exact same name.
- [x] `nm -D` shows 0 missing symbols and 0 undefined non-libc symbols for
      the Rust `.so`.
- [x] No stubs / `unimplemented!()` / `todo!()` anywhere in
      `translation/src/lib.rs` (verified by grep).
