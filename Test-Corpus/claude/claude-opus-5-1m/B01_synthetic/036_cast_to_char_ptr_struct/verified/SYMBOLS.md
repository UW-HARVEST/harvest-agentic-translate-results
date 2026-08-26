# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## Scope of the library

The whole C project is a single translation unit, `c_src/src/main.c` (52 lines).
`c_src/CMakeLists.txt` builds it as an **executable**
(`add_executable(driver src/main.c)`), so for symbol comparison it is also
compiled as a shared library:

```sh
gcc -shared -fPIC -O2 -o target/cbuild/libcdriver.so c_src/src/main.c
```

The Rust side is built as a `cdylib` (`src/lib.rs`, `crate-type = ["cdylib"]`)
plus an equivalent binary (`src/main.rs`) mirroring `add_executable`.
Both share one implementation module, `src/imp.rs`.

## `nm -D --defined-only` comparison

C shared library (`target/cbuild/libcdriver.so`):

```text
0000000000001170 T driver
0000000000001080 T main
```

Rust shared library (`target/debug/libdriver.so`):

```text
0000000000017f40 T driver
0000000000017f60 T main
```

| # | C symbol | C type | Rust `.so` exports it? | Rust definition |
|---|----------|--------|------------------------|-----------------|
| 1 | `driver` | `T` (global text) | yes | `#[no_mangle] pub extern "C" fn driver(floors: c_int)` in `src/lib.rs` |
| 2 | `main`   | `T` (global text) | yes | `#[no_mangle] pub extern "C" fn main() -> c_int` in `src/lib.rs` |

**Symbol diff: EMPTY.** Every symbol the C `.so` exports is exported by the
Rust `.so` under the exact same name.

### Symbols deliberately NOT exported

| C function | C linkage | exported by C `.so`? | Rust |
|------------|-----------|----------------------|------|
| `print_hex` | `static void print_hex(unsigned char *p, int len)` | no (internal) | `imp::print_hex`, not `#[no_mangle]` — matches |

Exporting `print_hex` would be a *divergence* from the C `.so`, so it is left
internal. It is exercised through `driver`, which is its only caller.

### Undefined (imported) symbols

The C `.so` imports only libc symbols; nothing needs a Rust counterpart:

```text
U __isoc99_scanf@GLIBC_2.7     (from main -> scanf("%d", &x))
U printf@GLIBC_2.2.5           (from print_hex -> printf("%02x", ...))
U putchar@GLIBC_2.2.5          (from print_hex -> printf("\n") folded to putchar)
w _ITM_deregisterTMCloneTable, _ITM_registerTMCloneTable, __gmon_start__,
  __cxa_finalize@GLIBC_2.2.5   (toolchain weak symbols)
```

The Rust `.so` imports the *same* stdio entry points, because the translation
calls them rather than reimplementing them (see the ERRORS.md divergence table,
rows E20–E23):

```text
U printf@GLIBC_2.2.5
U scanf@GLIBC_2.2.5
```

(`scanf` and glibc's `__isoc99_scanf` differ only in the handling of the
deprecated GNU `%a` allocation modifier, which this format string does not use;
for `%d` they run the same conversion code.)

`nm -D` on the Rust `.so` shows **0 missing/undefined non-libc symbols**: the
rest of its undefined set is the standard Rust-std libc/pthread/dl surface, and
no symbol from the C library is left unresolved. `tests/phase_d_symbols.rs`
proves this operationally by `dlopen`ing both libraries with `RTLD_NOW`, which
binds every relocation eagerly and fails if anything is unresolved.

### Verification

`tests/phase_d_symbols.rs` re-derives both symbol lists at test time with
`nm -D --defined-only` and asserts:

* every `T`/`W` symbol of the C `.so` is defined by the Rust `.so` (diff empty),
  and the Rust `.so` exports no *extra* public C symbol either
  (`d1_symbol_diff_is_empty`);
* both symbols are reachable through `dlopen`/`dlsym` from both libraries, and
  `print_hex` is resolvable in neither (`d2_symbols_are_reachable_via_dlsym`);
* both libraries load under `RTLD_NOW`, which binds every relocation eagerly and
  therefore proves there are no unresolved symbols (`d3_no_unresolved_symbols`);
* `c_src/` still contains exactly one translation unit, so no C module has been
  left untranslated (`d4_no_untranslated_c_sources`);
* the crate still declares no Cargo features and the C build no configuration
  switches, so the single-configuration claim in CONFIGS.md cannot go stale
  (`d5_single_build_configuration`).
