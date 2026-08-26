# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on the C shared object and the Rust `cdylib`.

## How the two `.so` files are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/main.c)`, so the
sanctioned CMake build produces an **executable**, not a shared object:

```
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/driver   (PIE executable, exports nothing in .dynsym)
```

For differential testing through the FFI boundary the same, unmodified
translation unit is additionally compiled as a shared object (no changes are
made inside `c_src/`; the artifact is written under `target/`):

```
gcc -shared -fPIC -o target/cdiff/libcdriver.so c_src/src/main.c
```

The Rust side is `crate-type = ["rlib", "cdylib"]`, giving `libdriver.so`.

`tests/common/mod.rs` rebuilds **both** artifacts when the test suite runs, so
the comparison can never use a stale one:

* `libcdriver.so` — `cc -shared -fPIC` on `c_src/src/main.c`;
* `libdriver.so` — a nested `cargo build --lib --target-dir target/cdiff/rustlib`
  (a private target dir, so the nested cargo never contends with the outer
  `cargo test`'s build lock). This is **required**: `cargo test` compiles the
  library only as an `rlib` for the test targets and leaves
  `target/debug/libdriver.so` untouched, so a test that loaded that path would
  silently verify whatever was built last. Feature flags are forwarded to the
  nested build through `CDIFF_CARGO_ARGS` (see `verify.sh`).

## C `.so` dynamic symbols (`nm -D target/cdiff/libcdriver.so`)

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001171 T bad
000000000000119d T good
00000000000011bd T main
0000000000001139 T printLine
                 U puts@GLIBC_2.2.5
```

## Parity table

Every symbol **defined** (`T`/`D`/`B`) by the C `.so` must also be exported by
the Rust `.so` under the exact same name.

| # | C symbol | C type | C signature | Rust `.so` | Rust export site | status |
|---|----------|--------|-------------|------------|------------------|--------|
| 1 | `printLine` | `T` (text, global) | `void printLine(const char *line)` | `T printLine` | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn printLine` | ✅ present |
| 2 | `bad` | `T` (text, global) | `void bad(void)` | `T bad` | `src/lib.rs` `#[no_mangle] pub extern "C" fn bad` | ✅ present |
| 3 | `good` | `T` (text, global) | `void good(void)` | `T good` | `src/lib.rs` `#[no_mangle] pub extern "C" fn good` | ✅ present |
| 4 | `main` | `T` (text, global) | `int main(int argc, char *argv[])` | `T main` | `src/lib.rs` `#[no_mangle] pub extern "C" fn main` | ✅ present |

### Linker/toolchain-generated symbols (not part of the API)

| C symbol | kind | Rust `.so` | note |
|----------|------|------------|------|
| `_ITM_deregisterTMCloneTable` | `w` weak undefined | `w` present | emitted by the linker in both |
| `_ITM_registerTMCloneTable` | `w` weak undefined | `w` present | emitted by the linker in both |
| `__cxa_finalize@GLIBC_2.2.5` | `w` weak undefined | `w` present | glibc DSO teardown hook |
| `__gmon_start__` | `w` weak undefined | `w` present | profiling hook |

### Undefined (imported) libc symbols — not exports, no parity requirement

| C `.so` imports | Rust `.so` equivalent |
|-----------------|-----------------------|
| `puts@GLIBC_2.2.5` (gcc's `printf("%s\n", s)` → `puts(s)` optimisation) | `write@GLIBC_2.2.5` / `writev` via `std::fs::File::write_all` on fd 1 |

The Rust `.so` additionally imports the usual Rust `std` support symbols
(`_Unwind_*`, `malloc`, `memcpy`, `pthread_key_*`, …) and exports Rust-mangled
(`_ZN…`/`_R…`) internal symbols. Extra Rust-side symbols are harmless: the
requirement is C-defined ⊆ Rust-defined, which holds.

## Static (non-exported) C functions — deliberately NOT exported by Rust

| C function | C linkage | reason it is absent from both `.so` files |
|------------|-----------|-------------------------------------------|
| `static void helperBad(void)` | internal | `static` in C ⇒ not in `.dynsym`; translated as private `imp::helper_bad` (never called, exactly like the C, which `bad()` does **not** call) |
| `static void helperGood(void)` | internal | `static` in C ⇒ not in `.dynsym`; translated as private `imp::helper_good`, called only from `good()` |

## Verification command / result

```
$ nm -D --defined-only target/cdiff/libcdriver.so \
    | awk '{print $NF}' | sort > c.syms          # bad, good, main, printLine
$ nm -D --defined-only target/cdiff/rustlib/debug/libdriver.so \
    | awk '{print $NF}' | sort > r.syms
$ comm -23 c.syms r.syms | wc -l                 # C-defined symbols missing from Rust
0

$ nm -D --undefined-only target/cdiff/rustlib/debug/libdriver.so | awk '{print $NF}' \
    | grep -vcE "@GLIBC|@GCC|^_ITM_|^__cxa_|^_Unwind_|^__gmon_start__"
0
```

The same two checks run as tests: `tests/symbol_parity.rs`
(`phase_d_rust_so_exports_every_c_symbol`,
`phase_d_every_c_symbol_is_dlsym_resolvable_in_rust` — which additionally
`dlsym`s every C symbol out of the Rust `.so` —,
`phase_d_rust_so_has_no_unresolved_non_libc_symbols`,
`phase_d_static_helpers_are_not_exported`).

**Result: 0 missing symbols.** No stubs were used — every export is backed by
the real translated implementation in `src/imp.rs`.
The whole C translation unit (4 exported functions + 2 `static` helpers +
`main`) is translated; nothing was skipped.

`nm -u target/debug/libdriver.so` lists only glibc/`_Unwind_*` imports, i.e.
0 missing/undefined non-libc symbols.
