# SYMBOLS.md — Phase A: exported-symbol surface

## What the C build produces

`c_src/CMakeLists.txt` contains exactly one target:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver src/main.c)
```

It is an **executable**, not a shared library. Consequently there is **no `.so`
surface to `dlopen`**: the program exports **zero defined dynamic symbols**, and
its only non-`main` function is declared `static void foo(int, int)` (internal
linkage). The observable public contract of this library-under-test is therefore
its **process interface**: stdin (`scanf("%d %d", &x, &y)`), stdout
(`printf("loop\n" / "x\n" / "y\n")`), and exit status.

For that reason the differential tests in `tests/` drive **both** compiled
artifacts as *processes* and compare stdout/stderr/exit-status byte for byte,
which is the exact analogue of "load both `.so`s and compare through the FFI
boundary" for an executable target. `libloading` is listed in
`[dev-dependencies]` as required and is used by `tests/symbols.rs` for the
symbol-surface assertions; it cannot be used to `dlopen` either artifact because
neither is a shared object (both are `ELF ... executable`, and the C one is not
even PIE — see below).

## Dynamic symbol tables (`nm -D`)

### C: defined dynamic symbols

```
$ nm -D --defined-only c_src/build/driver | wc -l
0
```

### Rust: defined dynamic symbols

```
$ nm -D --defined-only target/debug/driver | wc -l
0
```

**Symbol diff (C defined − Rust defined) = ∅.** There are no exported symbols to
mirror, so nothing needs a `#[no_mangle]`/`extern "C"` wrapper and no C module
was skipped by the translation (there is exactly one C translation unit,
`c_src/src/main.c`, and it is fully translated in `src/main.rs`).

### C: undefined/imported dynamic symbols

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __gmon_start__
                 U __isoc99_scanf@GLIBC_2.7
                 U __libc_start_main@GLIBC_2.34
                 U puts@GLIBC_2.2.5
```

All are libc/toolchain symbols. Note `puts@GLIBC_2.2.5`: gcc rewrote all three
`printf("...\n")` calls into `puts` (constant format, no conversions, trailing
newline). This is a pure code-generation detail — `puts` writes the string plus
one `'\n'`, byte-identical to the `printf` it replaced — so the Rust side's
`write_all(b"loop\n")` etc. remain byte-exact.

### Rust: undefined/imported dynamic symbols

68 entries, **all** of them libc (`read`, `write`, `writev`, `malloc`,
`memcpy`, `signal`, `sigaction`, `__libc_start_main`, `pthread_*`, `stat64`, …)
or the C++/Rust unwinder (`_Unwind_*@GCC_*`), which is part of the Rust standard
library runtime. **0 missing/undefined non-libc symbols.**

Verified mechanically by `tests/symbols.rs::rust_has_no_unresolved_non_libc_symbols`,
which shells out to `nm -D --undefined-only` and fails on any symbol that is not
matched by the libc/unwinder allowlist.

## Static symbol tables (informational)

Function symbols actually defined by each artifact:

| C (`nm --defined-only`) | binding | Rust counterpart | binding |
|---|---|---|---|
| `main` | `T` (global) | `main` | `T` (global) |
| `foo` | `t` (local — `static`) | `goto_loop::foo` | local (private `fn`) |
| `_start`, `_init`, `_fini`, `register_tm_clones`, … | CRT/toolchain | `_start`, … | CRT/toolchain |

`foo`'s internal linkage in C is mirrored by `foo` being a private (non-`pub`,
non-`#[no_mangle]`) function in Rust, so the *global* function surface —
`main` — matches exactly.

## Completion checklist for this file

- [x] Every symbol the C artifact exports dynamically (none) is also exported by
      the Rust artifact.
- [x] Symbol diff is empty.
- [x] `nm -D` shows 0 missing/undefined **non-libc** symbols in the Rust artifact.
- [x] No C translation unit was left untranslated (1 of 1 translated); no stubs
      or `unimplemented!()` anywhere in `src/`.

## Results

```
$ cargo test --test symbols
running 4 tests
test dynamic_export_diff_is_empty ... ok
test rust_has_no_unresolved_non_libc_symbols ... ok
test global_function_surface_matches ... ok
test neither_artifact_exposes_loadable_symbols ... ok
test result: ok. 4 passed; 0 failed
```

`run_all_configs.sh` repeats the `nm -D` diff for the **debug and release**
artifacts; both report an empty diff and zero unresolved non-libc symbols.
