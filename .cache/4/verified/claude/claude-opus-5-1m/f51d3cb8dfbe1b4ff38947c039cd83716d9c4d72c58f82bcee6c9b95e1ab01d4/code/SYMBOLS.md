# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

## What was built

`c_src/CMakeLists.txt` declares `add_executable(driver src/main.c)` — the C
project is an **executable**, not a library, so CMake alone produces no `.so`.
To get a dlopen()-able comparison target, `build.rs` additionally compiles the
*same, unmodified* `c_src/src/main.c` with `-shared -fPIC -O2`. Nothing in
`c_src/` is modified; all outputs go to `OUT_DIR`.

| artifact | built from | by |
|---|---|---|
| `c_driver` (exe) | `c_src/src/main.c` | `build.rs` (`cc -O2`), same as CMake's `add_executable` |
| `libc_driver.so` | `c_src/src/main.c` | `build.rs` (`cc -shared -fPIC -O2`) |
| `driver` (exe) | `src/main.rs` | cargo (`CARGO_BIN_EXE_driver`) |
| `librust_driver.so` | `src/lib.rs` | `build.rs` (`rustc --crate-type cdylib -O -C panic=abort`) |
| `libdriver.so` | `src/lib.rs` | cargo (`crate-type = ["cdylib", "rlib"]`) |

## Exported (defined) symbols — `nm -D --defined-only`

The C shared library defines exactly **one** symbol:

```
0000000000001050 T main
```

The Rust shared library defines the same one:

```
0000000000012430 T main        # librust_driver.so  (rustc-built)
00000000000125b0 T main        # libdriver.so       (cargo-built)
```

`_init`/`_fini` are linker-generated section stubs and are excluded, as they are
not part of either library's API.

### Symbol diff

| direction | result |
|---|---|
| C symbols missing from Rust `.so` | **0** (empty) |
| Rust symbols not present in C `.so` | **0** (empty) |

```
$ comm -23 <(nm -D --defined-only libc_driver.so   | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only librust_driver.so | awk '{print $3}' | sort -u)
<no output>
```

The sets are **identical**, so no export wrapper needed adding and no C source
went untranslated — `c_src/src/main.c` (5 non-comment lines) is the entire C
library, and all of it is translated in `src/hello.rs` + `src/lib.rs`.

`main` is exported from Rust by `#[no_mangle] pub extern "C" fn main() ->
c_int` in `src/lib.rs`, matching C's `int main(void)` ABI. It is a real
translation, not a stub.

## Undefined (imported) symbols

| library | undefined symbols | unresolved at load? |
|---|---|---|
| `libc_driver.so` | `puts@GLIBC` (gcc rewrote `printf` of a `\n`-terminated literal into `puts`), plus weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__` | none |
| `librust_driver.so` | 49: glibc (`write`, `memcpy`, `pthread_*`, …) + `_Unwind_*@GCC_*` from `libgcc_s.so.1` (a `NEEDED` entry) | none |

```
$ ldd -r librust_driver.so | grep -i undefined
<no output>
$ nm -D --undefined-only librust_driver.so | grep -vE 'GLIBC|GCC_|^\s+w '
<no output>
```

**Gate: 0 missing exports, 0 unresolved non-libc symbols.** ✅

## Note on the `main` symbol

Exporting a symbol literally named `main` from a `cdylib` is legal and links
cleanly: a shared object's `main` is an ordinary global function. `libloading`
resolves it against the specific library handle (`RTLD_LOCAL`), so the tests get
the library's `main` and never the test harness's own entry point. This is
verified in `tests/ffi_differential.rs::c_and_rust_so_export_only_main`, which
asserts both handles yield distinct, non-null `main` pointers that differ from
each other.
