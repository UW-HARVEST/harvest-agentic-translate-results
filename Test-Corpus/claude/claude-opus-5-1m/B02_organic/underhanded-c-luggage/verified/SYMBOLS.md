# SYMBOLS.md — symbol surface of the C artifact vs. the Rust artifact

## 0. What the C build actually produces

`c_src/CMakeLists.txt` contains exactly one target:

```cmake
add_executable(driver src/luggage.c)
```

so the canonical C artifact is an **executable**, not a shared library.  `nm -D`
on that executable therefore lists only libc **imports** plus the `stderr`
copy-relocation — the program's own functions are ordinary local/global `T`
symbols in a non-PIE executable and are *not* part of a dynamic export table:

```
$ nm -D c_src/build/driver
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __gmon_start__
                 U __isoc99_scanf@GLIBC_2.7
                 U __libc_start_main@GLIBC_2.34
                 U calloc@GLIBC_2.2.5
                 U exit@GLIBC_2.2.5
                 U fwrite@GLIBC_2.2.5
                 U printf@GLIBC_2.2.5
0000000000404040 B stderr@GLIBC_2.2.5
                 U strcmp@GLIBC_2.2.5
                 U strcpy@GLIBC_2.2.5
```

```
$ nm --defined-only c_src/build/driver | grep ' T '
0000000000401186 T addRoutingDirectiveToList
00000000004013fd T main
000000000040129e T matches
00000000004012de T printMatchingDirectives
000000000040126d T superseded
00000000004011ea T supersedes
```

Consequences for verification:

1. The **public surface of the C artifact is its process interface**: `argc`/
   `argv`, `stdin`, `stdout`, `stderr` and the exit status.  The primary
   differential tests therefore run BOTH executables as subprocesses with
   identical argv + stdin and compare stdout, stderr and exit code
   byte-for-byte (`tests/differential_exec.rs`).
2. To *also* satisfy the "load both `.so`s with `libloading` and compare through
   the FFI boundary" requirement, the C translation unit is additionally built as
   a shared library **without modifying anything in `c_src/`** — only compiler
   flags are used:

   ```
   gcc -shared -fPIC -O0 -Dmain=luggage_main -o cbuild/libluggage.so c_src/src/luggage.c
   ```

   (`-Dmain=luggage_main` renames `main` so the translation unit is loadable as a
   library; every other function keeps its original name.)
   The Rust crate gained a matching `cdylib`/`rlib` target (`src/lib.rs`) that
   exports the *same six* symbols with the *same* C ABI and delegates to the very
   same translated logic that `src/main.rs` (the deliverable binary) uses — the
   core translation lives in `src/luggage_core.rs`, which is `include!`d by both
   targets, so the FFI tests exercise the real translated code, not a copy.

## 1. Symbol parity: C `.so` vs Rust `.so`

C shared library (`cbuild/libluggage.so`), defined text symbols:

| # | C symbol | exported by Rust `libdriver.so` | Rust implementation |
|---|----------|--------------------------------|---------------------|
| 1 | `addRoutingDirectiveToList` | yes (`#[no_mangle] extern "C"`) | `src/lib.rs` → `add_routing_directive_to_list` (`src/luggage_core.rs`) |
| 2 | `supersedes`                | yes (`#[no_mangle] extern "C"`) | `src/lib.rs` → `supersedes` |
| 3 | `superseded`                | yes (`#[no_mangle] extern "C"`) | `src/lib.rs` → `superseded` |
| 4 | `matches`                   | yes (`#[no_mangle] extern "C"`) | `src/lib.rs` → `matches` |
| 5 | `printMatchingDirectives`   | yes (`#[no_mangle] extern "C"`) | `src/lib.rs` → `print_matching_directives` |
| 6 | `luggage_main`              | yes (`#[no_mangle] extern "C"`) | `src/lib.rs` → `luggage_main_impl` (same code path as `fn main`) |

The parity check is automated: `tests/differential_ffi.rs::symbol_parity_c_so_vs_rust_so`
runs `nm -D --defined-only` on both `.so`s, filters out toolchain/libc noise
(`_ITM_*`, `__gmon_start__`, `_init`, `_fini`, `__cxa_*`, `rust_eh_personality`,
`_Unwind_*`, data symbols) and asserts the C set minus the Rust set is **empty**.
`check_all.sh` prints the same diff.

Result: **0 missing symbols** (see `RESULTS.md`).

## 2. Undefined (imported) symbols

The C executable imports only libc (`__isoc99_scanf`, `printf`, `fwrite`,
`calloc`, `strcmp`, `strcpy`, `exit`, `stderr`, `__libc_start_main`).
The Rust artifacts import libc equivalents through `std` (`memcpy`, `write`,
`malloc`, …) plus the Rust unwinder.  There are **no undefined non-libc symbols**
in the Rust artifacts (`nm -u target/release/driver | grep -v GLIBC | grep -v GCC`
is empty apart from the weak `_ITM_*`/`__gmon_start__`/`__cxa_*` markers, which
are weak in the C binary as well).

## 3. C source → Rust function map (completeness of the translation)

Every function and every file of the C source is translated; nothing was
skipped, nothing is stubbed.

| C source (`c_src/src/luggage.c`) | lines | Rust (`src/luggage_core.rs`) |
|---|---|---|
| `struct RoutingDirective`        | 12–20   | `struct RoutingDirective` + arena (`Arena`); C-layout mirror `CRoutingDirective` in `src/lib.rs` (`repr(C)`, size 120, offsets 0/4/13/20/24/28/112 — verified against `offsetof`) |
| `addRoutingDirectiveToList`      | 22–32   | `add_routing_directive_to_list` (recursion → loop, same insertion point) |
| `supersedes`                     | 34–47   | `supersedes` |
| `superseded`                     | 49–53   | `superseded` |
| `matches`                        | 56–58   | `matches` |
| `printMatchingDirectives`        | 60–82   | `print_matching_directives` |
| `main`                           | 84–131  | `luggage_main_impl` (called by `fn main` in `src/main.rs` and by the exported `luggage_main`) |
| `scanf("%d ")`                   | 102     | `scanf_time_stamp` + `Scanner::scan_d` |
| `scanf("%8[A-Z0-9] %6[A-Z0-9] ")`| 105     | `scanf_ids` + `Scanner::scan_set` |
| `scanf("%3[A-Z] %3[A-Z]")`       | 109     | `scanf_airports` |
| `scanf("%80[^\n]")`              | 112     | `scanf_comments` |
| `strcpy`/`strcmp`/`%s` semantics | various | `c_str`, `c_str_eq` (stop at first NUL) |
