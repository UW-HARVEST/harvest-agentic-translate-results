# SYMBOLS.md — Phase A symbol surface

## Build shape

`c_src/CMakeLists.txt` contains exactly one target:

```cmake
add_executable(driver src/main.c)
```

So the C artifact is an **executable**, not a shared library, and
`logs/prompt.md` (the original translation prompt) confirms this:
*"This is an EXECUTABLE."* The Rust side mirrors that with `[[bin]] name =
"driver"`.

Consequently the primary observable surface is the **process contract**
(bytes on `stdin` → bytes on `stdout` → exit status), not a set of dynamic
library exports. Both surfaces are enumerated and verified below.

## 1. Dynamic symbol table of the C executable

`nm -D c_src/build/driver`:

| symbol | bind | kind | notes |
|---|---|---|---|
| `_ITM_deregisterTMCloneTable` | weak | undefined | libitm hook, absent at runtime |
| `_ITM_registerTMCloneTable`   | weak | undefined | libitm hook, absent at runtime |
| `__gmon_start__`              | weak | undefined | profiling hook, absent at runtime |
| `__isoc99_scanf@GLIBC_2.7`    | global | undefined (**U**) | libc import |
| `__libc_start_main@GLIBC_2.34`| global | undefined (**U**) | libc import |
| `printf@GLIBC_2.2.5`          | global | undefined (**U**) | libc import |
| `putchar@GLIBC_2.2.5`         | global | undefined (**U**) | libc import (tail-call from `printf("\n")`) |

**There are zero *defined* (exported) dynamic symbols.** Every entry is
either an undefined libc import or a weak undefined toolchain hook. The
`0 missing/undefined non-libc symbols` requirement is therefore satisfied
trivially for the executable form: the set of non-libc symbols the C
artifact exports is empty, so the Rust artifact cannot be missing any.

Verified mechanically by `tests/symbol_parity.rs`
(`test_c_so_exports_are_all_present_in_rust_so`, `test_c_exe_dynamic_symbols_are_libc_only`).

## 2. Defined symbols in the C translation unit

`nm c_src/build/driver | grep -E ' [Tt] '` (application symbols only):

| C symbol | linkage | Rust counterpart | exported from Rust `.so`? |
|---|---|---|---|
| `main`      | `T` global | `fn main` (`src/main.rs`) | n/a — process entry point |
| `driver`    | `T` global | `driver` (`src/lib.rs`, `#[no_mangle] pub extern "C"`) | **yes** |
| `print_hex` | `t` **static** | `print_hex` (`src/lib.rs`, private) | no — `static` in C, so not exported by C either |

`print_hex` is `static` in `main.c`, so it has internal linkage and is
deliberately *not* exported on either side. `driver` has external linkage
and is the one application function that becomes a real dynamic export when
`main.c` is compiled as a shared object.

## 3. Shared-library form (for FFI-level differential testing)

To exercise the `#[no_mangle] extern "C"` wrapper exactly as an external
caller would, both sides are additionally built as shared objects by
`tests/common/mod.rs`:

* **C**: `gcc -shared -fPIC -o libcdriver.so c_src/src/main.c`
  (`c_src/` itself is never modified — the compile is driven from the test
  harness and the output lands in `target/`.)
* **Rust**: `[lib] crate-type = ["cdylib", "rlib"]` → `target/<profile>/libdriver.so`

`nm -D` on the two shared objects, filtered to application symbols
(i.e. dropping libc imports, `_init`/`_fini`, `_edata`/`_end`/`__bss_start`
and the weak toolchain hooks):

| symbol | C `.so` | Rust `.so` (default) | Rust `.so` (`--features c_main`) | status |
|---|---|---|---|---|
| `driver` | defined `T` | defined `T` | defined `T` | **match** |
| `main`   | defined `T` | absent | defined `T` | **match** under `c_main` |
| `print_hex` | not exported (`static`) | not exported | not exported | **match** |

`main` is defined in the C shared object because `main.c` is a *program*
translation unit that we additionally compiled with `-shared`. To make the
symbol diff reach **exactly empty**, `src/lib.rs` exports a real C-ABI `main`
— a genuine translation of `int main()`, not a stub — behind the `c_main`
feature:

```rust
#[cfg(all(feature = "c_main", not(test)))]
#[no_mangle]
pub extern "C" fn main() -> core::ffi::c_int {
    run();
    0
}
```

The feature exists because a Rust `[[bin]]` emits a `main` of its own, which
would be a duplicate-symbol link error. With `c_main` enabled `src/main.rs`
becomes `#![no_main]` and this exported `main` *is* the program entry point,
so the executable keeps working identically — verified by running the whole
differential suite under both feature combinations (`./run_all.sh`).

`not(test)` is required for the same reason: libtest generates its own entry
point for the unit-test binary.

So the symbol diff is:

| configuration | `C .so` exports − `Rust .so` exports |
|---|---|
| `--no-default-features` | `{main}` — `main` lives in the `[[bin]]` instead, where `nm` confirms it (§2) |
| `--no-default-features --features c_main` | **∅ (empty)** |

Both are asserted by `tests/symbol_parity.rs::test_c_so_exports_are_all_present_in_rust_so`.

## 4. Completion status

- [x] Every non-libc symbol exported by the C artifact is also exported by
      the Rust artifact with the identical name (`driver`, and `main` under
      `c_main`). The symbol diff reaches empty.
- [x] No stubs: `driver` and `main` in Rust are the real translations of the C
      `driver` and `main`, calling the real `print_hex` translation.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`
      (asserted with `ldd -r` in `test_rust_so_has_no_unresolved_non_libc_symbols`).
- [x] Statically-linked C symbols (`print_hex`) are private on both sides.
- [x] No C source file was left untranslated: `c_src/src/main.c` is the only
      source file in the project (`add_executable(driver src/main.c)`), and all
      three of its functions have Rust counterparts.
