# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C `.so`: built from **both** translation units listed in
  `c_src/CMakeLists.txt` (`src/mdcore.c` + `src/mdmain.c`) with the same
  `-DOP=<op> -DREPEAT=<n>` flags CMake passes
  (`scripts/build_c_so.sh <op> <n>` → `cbuild/so/libdriver_<op>_<n>.so`).
* Rust `.so`: `translation/target/release/libmacrodepth_add_5.so`
  (`[lib] crate-type = ["cdylib"]`).

The exported set is **identical for all 24 `(OP, REPEAT)` configurations**
(verified: `nm -D --defined-only` name/type list hashes to the same value for
every one of the 24 C builds and every one of the 24 Rust builds).

## Exported symbols

| # | symbol | nm type | C definition | Rust definition | status |
|---|--------|---------|--------------|-----------------|--------|
| 1 | `op_add`        | `T` (text)          | `mdcore.c:28` `int op_add(int,int)` | `src/mdcore.rs` `#[no_mangle] extern "C" fn op_add` | ✅ present |
| 2 | `op_sub`        | `T` (text)          | `mdcore.c:29` `int op_sub(int,int)` | `src/mdcore.rs` `#[no_mangle] extern "C" fn op_sub` | ✅ present |
| 3 | `op_mul`        | `T` (text)          | `mdcore.c:30` `int op_mul(int,int)` | `src/mdcore.rs` `#[no_mangle] extern "C" fn op_mul` | ✅ present |
| 4 | `G_OP`          | `D` (`.data`, writable) | `mdcore.c:36` `int (*G_OP)(int,int) = OP_FN(OP);` | `src/mdcore.rs` `#[no_mangle] static mut G_OP` | ✅ present (fixed placement, see below) |
| 5 | `G_OP_NAME`     | `D` (`.data`, writable) | `mdcore.c:37` `const char *G_OP_NAME = STR(OP);` | `src/mdcore.rs` `#[no_mangle] static mut G_OP_NAME` | ✅ present (fixed placement, see below) |
| 6 | `helper_call`   | `T` (text)          | `mdcore.c:39` | `src/mdcore.rs` `helper_call` | ✅ present |
| 7 | `helper_ptr`    | `T` (text)          | `mdcore.c:47` | `src/mdcore.rs` `helper_ptr` | ✅ present |
| 8 | `use_generated` | `T` (text)          | `mdcore.c:54` | `src/mdcore.rs` `use_generated` | ✅ present |
| 9 | `main`          | `T` (text)          | `mdmain.c:28` `int main(int argc, char **argv)` | `src/mdmain.rs` `#[no_mangle] unsafe extern "C" fn main` | ✅ **TRANSLATED in this pass** — the whole `mdmain.c` module had never been translated |

## Deliberately *not* exported (matches C)

| C symbol | why | Rust |
|----------|-----|------|
| `accum_<OP>` (`DEFINE_ACCUM(OP)` → `static int accum_add(int n)`, `mdmacros.h:95`) | declared `static` → internal linkage, absent from `nm -D` | private `fn accum` in `src/mdcore.rs` |
| the `mdmacros.h` macros (`STEP_*`, `REP0..REP7`, `DISPATCH_REP`, `INIT_*`, `OP_FN`, `STR`, `CAT`, …) | preprocessor-only, never emit a symbol | `const`s / `#[inline] fn`s in `src/mdmacros.rs` |

## Fixes made in this pass

1. **`main` was entirely missing** (the `mdmain.c` translation unit had not been
   translated). Translated in full as `src/mdmain.rs`, exported as `main` with
   the C signature. It is *not* a stub: it parses `argv` with `atoi`, runs the
   `REP<REPEAT>` unrolled loop, calls `helper_call`/`helper_ptr`/
   `use_generated(REPEAT)`, dispatches through the **`G_OP` global**, and prints
   the same two `printf` lines.
2. **`G_OP` / `G_OP_NAME` were read-only.** As plain Rust `static`s with
   relocations they landed in `.data.rel.ro` (section made read-only by RELRO),
   while the C globals are non-`const` and land in writable `.data`. `nm -D`
   reports `D` for both, so the symbol table alone hid the difference — but a
   caller storing into `G_OP` (which the C program supports, and which changes
   what `main` dispatches to) would have segfaulted against the Rust `.so`.
   Both are now `static mut`, i.e. section `.data` (verified with
   `readelf -sW` + `readelf -S`).

## Undefined-symbol audit (`nm -D --undefined-only`)

The Rust `.so` has **0 undefined non-libc symbols**. Every `U`/`w` entry is a
glibc or unwinder import (`printf@GLIBC`, `fprintf@GLIBC`, `atoi@GLIBC`,
`stderr@GLIBC`, `memcpy`, `malloc`, `_Unwind_*`, `__cxa_finalize`, …), exactly
the class of imports the C `.so` also has. `scripts/check_symbols.sh` performs
the whole comparison (C-exported ⊆ Rust-exported, plus the non-libc undefined
check) for every configuration and prints an empty diff.
