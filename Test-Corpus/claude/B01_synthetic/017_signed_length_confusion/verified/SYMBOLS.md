# SYMBOLS.md — exported symbol surface (Phase A / Phase D)

## What the C project actually builds

`c_src/CMakeLists.txt` declares **one** target:

```cmake
project(driver)
add_executable(driver src/main.c)
```

It is an **executable**, not a library, and it is built from a single
translation unit (`c_src/src/main.c`, 66 lines). There is no public header, no
`install()` rule and no library API. Consequently:

* The *observable contract* of this project is **process behavior**: bytes on
  stdout, bytes on stderr, and exit status / terminating signal for a given
  stdin. That is what `tests/differential_process.rs` verifies.
* The *symbol* surface below is recovered by additionally compiling the same
  translation unit as a shared object (`gcc -shared -fPIC`), out of tree, so
  `nm -D` has something to report. **Nothing in `c_src/` was modified.**

## `nm -D` — C shared object vs Rust shared object

Command used (non-function/glibc bookkeeping entries `_init`, `_fini`, `__*`,
`_edata`, `_end`, `_IO_stdin_used` filtered out):

```sh
gcc -shared -fPIC -o /tmp/libdriver.so c_src/src/main.c
nm -D --defined-only /tmp/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

| # | symbol | type | C `.so` | Rust `.so` | status |
|---|--------|------|---------|------------|--------|
| 1 | `printLine` | `T` (func) | yes | yes | **MATCH** — exported from `src/lib.rs` as `#[no_mangle] pub unsafe extern "C" fn printLine(*const c_char)` |
| 2 | `main`      | `T` (func) | yes | n/a — see below | **program entry point**, verified executable-to-executable |

### Symbol 1 — `printLine`

Originally the translation had no `extern "C"` export at all (`src/main.rs` was
a self-contained binary crate; `nm` showed no `printLine`). Per the Phase A
rule, the implementation existed in Rust but was not exported, so the export
wrapper was added rather than stubbed:

* the translated logic moved from `src/main.rs` to `src/lib.rs` (unchanged
  line-for-line, no behavior edits),
* `Cargo.toml` gained `[lib] crate-type = ["cdylib", "rlib"]` so an external
  caller can `dlopen` the Rust build exactly like the C one,
* `src/main.rs` is now a thin entry point (`driver::run()` + flush + `exit`).

`printLine` is exercised through `dlopen`/`dlsym` (via `libloading`) against
**both** shared objects in `tests/differential_ffi.rs`.

### Symbol 2 — `main`

`main` appears in the C `.so` only because a *program* translation unit was
compiled as a shared object; it is the process entry point, not a library API.
It is deliberately **not** re-exported from the Rust `cdylib`:

Rust's `bin` target generates its own C `main`, so adding
`#[no_mangle] pub extern "C" fn main()` to the shared `rlib` breaks the build
with a hard linker error (verified):

```
rust-lld: error: duplicate symbol: main
>>> defined at driver.…-cgu.0 (bin)
>>> defined at driver.…-cgu.0 in archive libdriver.rlib
```

`main` equivalence is therefore verified where it is actually observable — at
the process level, comparing the C `driver` executable against the Rust
`driver` executable over the full input surface (Phases B and C). Both
executables define `main` in their symbol tables:

```
$ nm --defined-only c_src/build/driver        | grep -w main   ->  T main
$ nm --defined-only target/release/driver     | grep -w main   ->  T main
```

For completeness, neither *executable* exports any function in its **dynamic**
symbol table — the C executable's `nm -D` contains only the imported
`stdin@GLIBC_2.2.5` object, and the Rust executable's is empty. So there is no
dynamic-symbol gap at the level of the artifact CMake actually produces.

## Undefined / imported symbols

The Rust `.so` imports only glibc and Rust-runtime symbols; there are **no
undefined non-libc symbols**:

```sh
nm -D -u target/release/libdriver.so   # all entries are GLIBC_* / ld-linux
```

## Result

* Missing/undefined non-libc symbols in the Rust build: **0**
* C `.so` function symbols not exported by the Rust `.so`: **0**
  (`printLine` matches; `main` is the entry point, covered exe-to-exe)

## Reproducing

```sh
./run_all.sh
```

builds the C executable (CMake) and C shared object (`gcc -shared`, out of
tree), enumerates the feature combinations from `Cargo.toml`, runs
`cargo check --all-targets` for each, diffs `nm -D` between the two shared
objects, and then runs both differential suites in the `dev` and `release`
profiles. Exit status 0 means every check passed.

Test suites (all comparisons go through the built artifacts, never through a
direct Rust call):

| suite | what it drives | cases |
|-------|----------------|-------|
| `tests/differential_process.rs` | the `driver` executables, end to end | 2904 |
| `tests/differential_descriptors.rs` | descriptor-level side effects (`SIGPIPE`, stdin offset) | 38 |
| `tests/differential_ffi.rs` | `printLine` via `dlopen`/`dlsym` on both `.so`s | 214 |

3156 differential cases per configuration; 4 configurations (2 profiles x 2
feature invocations) => ~12.6k comparisons, all matching.
