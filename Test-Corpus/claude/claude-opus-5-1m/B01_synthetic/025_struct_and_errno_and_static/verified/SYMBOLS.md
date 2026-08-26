# SYMBOLS.md — exported-symbol parity

## What the C build produces

`c_src/CMakeLists.txt` contains exactly one target:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver src/main.c)
```

There are no `option()`s, no `target_compile_definitions`, no `#ifdef`s anywhere
in `c_src/src/main.c`, and no `add_library`. So the C side has **one** build
configuration and its primary artifact is an **executable**, not a shared
library.

`c_src/src/main.c` is a single translation unit. Everything in it is `static`
except two functions:

| C declaration | linkage |
|---|---|
| `int main()` | external (process entry point) |
| `void run(int extra_bedrooms)` | external |
| `static void add_floor(house_t *)` | internal |
| `static void add_bedrooms(house_t *, int)` | internal |
| `static void add_floor_to_the_house()` | internal |
| `static void print_the_house()` | internal |
| `static bool parse_val(const char *, int *)` | internal |
| `static house_t the_house` | internal (data) |

So the entire externally visible symbol set of the translation unit is
`{ main, run }`. `static` symbols are *not* part of the ABI and are not
required to appear in the Rust artifacts (they are still verified
behaviourally — see `CONFIGS.md` / `ERRORS.md`).

## Artifacts compared

Because the C target is an executable, parity is checked on **both** shapes:

| shape | C artifact | Rust artifact |
|---|---|---|
| executable | `gcc -O2 -o c_driver c_src/src/main.c` (identical to the CMake target) | `target/{debug,release}/driver` (`[[bin]] driver`) |
| shared object | `gcc -O2 -shared -fPIC -o libc_driver.so c_src/src/main.c` | `target/{debug,release}/libdriver_ffi.so` (`ffi/` cdylib) |

The `ffi/` workspace member exists purely so that a `.so`-to-`.so` comparison is
possible: it is a `cdylib` that re-exports the same two C-ABI symbols from the
same translated code the executable uses. (The exported `main` cannot live in
the `driver` rlib, because a `#[no_mangle] fn main` inside an rlib collides with
the entry point of every test-harness binary that links it — verified: `error:
entry symbol main declared multiple times`. Hence a separate leaf crate.)

## `nm -D --defined-only` on the shared objects

```
$ nm -D --defined-only libc_driver.so          # C
00000000000010a0 T main
0000000000001220 T run

$ nm -D --defined-only libdriver_ffi.so        # Rust
0000000000013d00 T main
0000000000013d10 T run
```

**Diff: empty.** Every symbol the C `.so` exports, the Rust `.so` exports under
the exact same name.

Undefined (imported) symbols of the C `.so` — all libc, all satisfied:
`__cxa_finalize`, `__errno_location`, `fgets`, `printf`, `puts`, `stdin`,
`strtol`, plus the weak `_ITM_*` / `__gmon_start__` stubs. The Rust `.so`
imports only libc/`libgcc_s` symbols (`memcpy`, `write`, `__errno_location`,
`_Unwind_*`, …); `nm -D -u` shows **0 undefined non-libc symbols**.

## `nm` on the executables

Application-level `T` (global text) symbols:

```
$ nm c_driver | grep ' T '                     # C
... T _init            (CRT)
... T _fini            (CRT)
... T _start           (CRT)
... T _dl_relocate_static_pie   (CRT)
... T main
... T run

$ nm target/release/driver | grep ' T '        # Rust
... T _start           (CRT)
... T main
... T run
... T rust_eh_personality                (Rust runtime)
... T _RNvCs..._7___rustc12___rust_alloc (Rust runtime allocator shims)
...
```

**Application symbol diff: empty** — `{ main, run }` on both sides.

The residual differences are runtime/CRT artifacts, not translated code:

* `_init` / `_fini` / `_dl_relocate_static_pie` come from `crti.o`/`crtn.o` and
  the non-PIE glibc startup files that `gcc` links; the Rust `cc` link line uses
  `-pie` and modern `crt1.o`, which has no `_init`/`_fini` pair. Nothing in
  `main.c` maps to them.
* `rust_eh_personality` and the `___rustc*` allocator/panic shims are the Rust
  standard library's own runtime hooks — the analogue of the libc symbols the C
  binary imports rather than defines.
* `nm -D --defined-only` on the C executable lists `stdin@GLIBC_2.2.5`. That is
  a *copy relocation* of libc's `stdin` object created because `gcc` links the
  executable non-PIE; it is libc data, not a symbol defined by `main.c`. The
  Rust executable is PIE and therefore has an empty defined dynamic symbol
  table. Both reference the same libc `stdin`.

## Nothing is stubbed

`run` is a real translation (`src/house.rs::run_global`, wrapped in
`src/main.rs` and `ffi/src/lib.rs`) and `main` is a real translation
(`src/lib.rs::c_main_with`). There is no `unimplemented!()`, `todo!()`,
`panic!("not implemented")` or empty-body export anywhere in the crate:

```
$ grep -rn 'unimplemented!\|todo!\|not implemented' src ffi/src
(no matches)
```

No C source file was skipped: `c_src/` contains exactly one `.c` file
(`src/main.c`, 87 lines) and all 7 of its functions plus its one file-scope
object are translated in `src/house.rs` and `src/parse.rs`.

## Checklist

- [x] `nm -D` on the C `.so` vs the Rust `.so`: 0 missing defined symbols.
- [x] `nm -D -u` on the Rust `.so`: 0 undefined non-libc symbols.
- [x] `nm` application `T` symbols on the C exe vs the Rust exe: 0 missing.
- [x] No stubs / fake exports.
