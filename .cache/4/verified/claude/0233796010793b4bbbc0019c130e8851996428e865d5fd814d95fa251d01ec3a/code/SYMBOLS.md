# SYMBOLS.md — public surface parity (Phase A / Phase D)

## Build-time configuration surface

| source | configurations |
|--------|----------------|
| `Cargo.toml` | **no `[features]` section at all** → exactly ONE configuration. `--no-default-features` == default. |
| `c_src/CMakeLists.txt` | one target: `add_executable(driver src/main.c)` + `target_link_libraries(driver m)`. No `option()`, no `#ifdef`/`#if` anywhere in `src/main.c` (verified by grep). |

So the complete enumeration of valid feature combinations is:

| # | combo | `cargo check` | `cargo test` |
|---|-------|---------------|--------------|
| 1 | `--no-default-features` (== default, the empty set) | PASS | PASS |

There is no second combination to enumerate; `grep -n features Cargo.toml` returns
nothing and the C has zero preprocessor configuration branches.

## Artifact kind: EXECUTABLE, not a shared library

`c_src/CMakeLists.txt` declares `add_executable(driver src/main.c)`, and
`src/main.c` defines exactly one program-level symbol — `main`. There is no
library API, no header, and no `#[no_mangle]` export surface to compare, so the
"load both `.so`s with `libloading`" recipe is not literally applicable here.

The equivalent black-box boundary for an executable is the **process ABI**:
`argc`/`argv` in, and (stdout bytes, stderr bytes, exit status / terminating
signal) out. Every differential test therefore executes BOTH real binaries as
child processes and compares those observables byte-for-byte. No Rust function is
ever called in-process, exactly as required: the tests only ever see what an
external caller sees.

* C binary: `c_src/build/driver`
* Rust binary: `target/release/driver`
* Both children are launched with `argv[0]` forced to the identical string via
  `std::os::unix::process::CommandExt::arg0`, so even the `Usage: %s ...`
  message is byte-comparable.
* `libloading` is still added to `[dev-dependencies]` as instructed and is used
  in `tests/ffi_libloading.rs`, where it loads `libm`/`libc` and calls `strtod`
  and `pow` across the FFI boundary to independently confirm that the values and
  `errno` codes both binaries report are the libc-native ones.

## `nm -D` — dynamic symbol table

### C binary (`nm -D c_src/build/driver`) — 10 entries, complete

| symbol | bind | kind | present in Rust `.so`/binary? |
|--------|------|------|-------------------------------|
| `_ITM_deregisterTMCloneTable` | weak | undefined (transactional-memory stub, emitted by crtstuff) | YES (weak undefined) |
| `_ITM_registerTMCloneTable` | weak | undefined (crtstuff) | YES (weak undefined) |
| `__errno_location@GLIBC_2.2.5` | global | **undefined — imported from libc** | **YES** — imported |
| `__gmon_start__` | weak | undefined (profiling hook) | YES (weak undefined) |
| `__libc_start_main@GLIBC_2.34` | global | undefined — imported from libc | YES — imported |
| `fprintf@GLIBC_2.2.5` | global | undefined — imported from libc | n/a — see note 1 |
| `pow@GLIBC_2.29` | global | **undefined — imported from libm** | **YES** — imported |
| `printf@GLIBC_2.2.5` | global | undefined — imported from libc | n/a — see note 1 |
| `stderr@GLIBC_2.2.5` | global | `B` **defined at 0x404040** | n/a — see note 2 |
| `strtod@GLIBC_2.2.5` | global | **undefined — imported from libc** | **YES** — imported |

`nm c_src/build/driver | grep ' T '` shows the only non-CRT text symbol defined
by the translation unit is `main` (plus `_start`, `_init`, `_fini`,
`_dl_relocate_static_pie` from the C runtime). `main` is not exported in
`.dynsym` and is not part of the callable surface of either artifact.

### Rust binary (`nm -D target/release/driver`)

```
$ nm -D target/release/driver | grep -vE '^ +[Uw] '     # locally DEFINED dynamic symbols
(empty)
$ nm -D target/release/driver | grep -E 'strtod|pow|__errno_location'
                 U __errno_location@GLIBC_2.2.5
                 U pow@GLIBC_2.29
                 U strtod@GLIBC_2.2.5
```

### Diff — semantically MISSING symbols: **NONE**

The three behaviour-defining libc/libm entry points the C program depends on —
`strtod`, `pow`, `__errno_location` — are imported by the Rust binary at the
*same* glibc version tags (`pow@GLIBC_2.29`, `strtod@GLIBC_2.2.5`,
`__errno_location@GLIBC_2.2.5`). The Rust translation deliberately does NOT
reimplement float parsing, `pow`, or `errno`; it calls the identical libc code,
which is why value-level and `errno`-level results cannot drift.

1. `fprintf` / `printf`: pure output formatting, not part of the observable
   contract. The Rust binary reaches the same bytes through
   `std::io::{stdout,stderr}` → `write`/`writev` syscalls. `tests/` asserts the
   emitted byte streams are identical, which is the actual requirement.
2. `stderr@GLIBC_2.2.5` appears as a *defined* `B` symbol only because the C
   binary is linked non-PIE (address `0x404040`) and the linker made a **copy
   relocation** of glibc's `stderr` FILE\* variable. It is glibc's data object,
   not an API the program provides; the Rust binary (PIE) needs no copy reloc.
   Nothing can call or link against it.

No symbol is stubbed, faked, or `unimplemented!()`. No C source file was left
untranslated: `c_src/src/main.c` is the only C file (`ls c_src/src` → 1 file)
and all 44 of its executable lines are translated in `src/main.rs`.

## Undefined-symbol resolution check

```
$ ldd target/release/driver          # all imports resolve
$ nm -D -u target/release/driver | grep -v GLIBC_ | grep -v GCC_
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __gmon_start__
```

The only three undefined symbols that are not versioned libc/libm/libgcc imports
are the exact same **weak** CRT stubs the C binary also leaves undefined (they
resolve to 0 and are never called). Every strong undefined symbol resolves:
`ldd` reports `libgcc_s.so.1`, `libm.so.6`, `libc.so.6` — no "not found" lines.

0 missing / 0 unresolved non-libc symbols. **Gate satisfied.**

## Result

| gate | status |
|------|--------|
| `nm -D`: C-exported symbols missing from Rust | **0** (3 documented non-behavioural exceptions, enforced by `tests/symbols.rs::d4`) |
| `nm -D`: unresolved non-libc/libgcc symbols in Rust | **0** (only the same 3 weak CRT stubs the C has) |
| behaviour-defining imports shared (`strtod`, `pow@GLIBC_2.29`, `__errno_location`) | **yes** (`tests/symbols.rs::d2`) |
| C source files translated | 1 of 1 (`c_src/src/main.c`) — nothing stubbed |
| feature combinations verified | 1 of 1 (`Cargo.toml` has no `[features]`) |

Automated by `tests/symbols.rs` (4 tests) and the `verify.sh` symbol diff, so the
gate is re-checked on every run rather than being a one-off observation.
