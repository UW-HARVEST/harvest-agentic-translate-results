# SYMBOLS.md — surface map (Phase A)

## Target kind: EXECUTABLE, not a shared library

`c_src/CMakeLists.txt` contains exactly one target:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver src/main.c)
```

There is no `add_library`, no `SHARED`, no install/export set, and no header
files at all (`c_src/` contains only `CMakeLists.txt` and `src/main.c`).
Consequently:

* the C artifact is an **ELF executable**, and
* every function except `main` is declared `static`, so **nothing is exported
  dynamically**.

`nm -D --defined-only` proves it (0 defined dynamic symbols on both sides):

| artifact | `nm -D --defined-only` count |
|----------|------------------------------|
| `c_src/build/driver` (C) | **0** |
| `target/release/driver` (Rust) | **0** |

Therefore the "load both `.so`s with `libloading` and compare exported
symbols" recipe has no `.so` and no exported symbol to work with. The
faithful equivalent of the FFI boundary for this artifact is the **process
boundary**: `argv` + `stdin` bytes in, `stdout` / `stderr` bytes + wait-status
out. All differential tests in `tests/` therefore drive **both compiled
programs as external processes** (`std::process::Command`) and never call any
Rust function directly — the Rust code under test is only ever reached through
the real `main` of the real linked binary, exactly as an external caller
(the shell) reaches it.

In addition, `tests/dlopen_c_lib.rs` **does** use `libloading`: it compiles
`c_src/src/main.c` a second time as a position-independent **shared object**
(`cc -shared -fPIC`, without modifying anything in `c_src/`), `dlopen`s it,
and calls its exported `main` symbol through the FFI boundary with `stdin`/
`stdout` redirected. This gives a second, independent channel to the C ground
truth and exercises the loader path required by the task.

## 1. C dynamic symbol table (`nm -D c_src/build/driver`)

Defined (exported) symbols: **none**.

Imported / weak symbols (these are libc/loader symbols, not part of the
program's own surface):

| symbol | class | note |
|--------|-------|------|
| `_ITM_deregisterTMCloneTable` | `w` weak undefined | libc/gcc boilerplate |
| `_ITM_registerTMCloneTable`   | `w` weak undefined | libc/gcc boilerplate |
| `__gmon_start__`              | `w` weak undefined | libc/gcc boilerplate |
| `__isoc99_scanf@GLIBC_2.7`    | `U` undefined | ← `scanf("%d %d %d", ...)` |
| `__libc_start_main@GLIBC_2.34`| `U` undefined | C runtime entry |
| `printf@GLIBC_2.2.5`          | `U` undefined | ← `printf("Result: %d\n", ...)` |
| `puts@GLIBC_2.2.5`            | `U` undefined | ← gcc rewrites the constant-string `printf("...\n")` calls into `puts` |

Rust side: `nm -D --defined-only target/release/driver` is also empty, and every
undefined symbol it lists is a libc / libgcc-unwind symbol (`read`, `write`,
`writev`, `malloc`, `signal`, `_Unwind_*`, …). **0 missing/undefined non-libc
symbols.**

## 2. Full static symbol table of the C program (`nm`)

This is the real code surface. Every entity is accounted for in the Rust
translation:

| C symbol | binding | kind | Rust counterpart in `src/main.rs` |
|----------|---------|------|-----------------------------------|
| `main` | `T` global | function, `int main()` | `fn main()` (process entry point) |
| `multi_stage` | `t` local (`static`) | function, `static int multi_stage(int x, int z)` | `fn multi_stage<W: Write>(out, g, x, z) -> i32` |
| `y` | `d` local (`static`) | file-scope `static int y = 123;` mutated by `scanf` | `Globals { y: i32 }`, initialised to `123` |
| `_start`, `_init`, `_fini`, `_dl_relocate_static_pie`, `frame_dummy`, `register_tm_clones`, `deregister_tm_clones`, `__do_global_dtors_aux`, `completed.0`, `_DYNAMIC`, `_GLOBAL_OFFSET_TABLE_`, `__TMC_END__`, `__bss_start`, `__data_start`, `_edata`, `_end`, `__do_global_dtors_aux_fini_array_entry`, `__frame_dummy_init_array_entry` | — | CRT / linker-generated | provided by the Rust CRT + linker (not program code) |

Implicit libc behaviour that is part of the observable surface and is therefore
re-implemented in Rust (see `src/main.rs`):

| C library behaviour used | Rust counterpart |
|--------------------------|------------------|
| `scanf` `%d` directive: skip `isspace`, optional `+`/`-`, decimal digits, `strtol` saturation at `LONG_MAX`/`LONG_MIN`, narrowing to `int` (mod 2^32), ungetc of the first non-digit | `ScanReader::scan_i32` |
| whitespace in the format string matches zero or more whitespace bytes | `ScanReader::skip_whitespace` |
| a failed conversion aborts the whole `scanf` call, leaving later arguments untouched | nested `if let Some(v) = …` chain in `main` |
| `scanf` return value ignored by the C code | return value discarded in Rust |
| `printf` write errors ignored | `let _ = out.write_all(..)` |
| default `SIGPIPE` disposition (`SIG_DFL`) of a C process → death by signal 13 on a broken stdout pipe | `restore_default_sigpipe()` resets `SIGPIPE` to `SIG_DFL`, undoing the Rust runtime's `SIG_IGN` |
| glibc `stdin` stream: buffer of `st_blksize` bytes (`_IO_file_doallocate`, `BUFSIZ` fallback), one `read()` per underflow, and the exit-time `_IO_cleanup` → `_IO_SYNC` that returns unconsumed read-ahead to a seekable descriptor | `CStdin` (`new`/`next_byte`/`unread`/`sync`), called from `main` after the output is flushed |

## 3. Missing-symbol resolution (Phase A / Phase D rule)

Symbol diff between the C `.so`/executable and the Rust executable:

```
comm -23 <(nm -D --defined-only c_src/build/driver | awk '{print $NF}' | sort) \
         <(nm -D --defined-only target/release/driver | awk '{print $NF}' | sort)
```

→ **empty** (both sides export nothing; there is no `.so` and no exported
symbol in this target kind). No C source file was left untranslated:
`c_src/src/main.c` is the only C file (62 lines) and all three of its entities
(`main`, `multi_stage`, `y`) are translated in `src/main.rs`. Nothing is
stubbed and there is no `unimplemented!()`/`todo!()` anywhere in `src/`.

Automated check: `tests/symbols.rs::symbol_parity_c_vs_rust`.
