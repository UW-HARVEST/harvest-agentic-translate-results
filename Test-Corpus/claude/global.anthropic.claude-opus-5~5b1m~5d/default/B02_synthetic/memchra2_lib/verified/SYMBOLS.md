# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on the built C shared library
`c_src/build/libharvest-work-UKfGah.so` and the Rust cdylib
`translation/target/release/libmemchra2_lib.so`.

## C `.so` exported (defined, dynamic) symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-UKfGah.so
00000000000013e1 T memchra2
```

That is the complete list. `c_src/src/lib.c` is the only translation unit and
every other routine in it (`memchra`, `process_buffer`, `int_to_float_bits`,
`process_strings`, `safe_sum_array`, `interpret_as_int`, `count_occurrences`,
`complex_iteration`) is declared `static`, i.e. internal linkage, so it is not
part of the ABI surface. `c_src/include/lib.h` likewise declares only
`int memchra2(int a, int b, int c, int d);`.

## Parity table

| # | C symbol | type | Rust `.so` exports it? | notes |
|---|----------|------|------------------------|-------|
| 1 | `memchra2` | `T` (global text) | YES — `00000000000011b70 T memchra2` | `#[unsafe(no_mangle)] pub extern "C" fn memchra2(a,b,c,d) -> c_int` |

### Missing symbols

None. The symbol diff is EMPTY:

```
$ comm -23 <(nm -D --defined-only <C.so>  | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only <RS.so> | awk '{print $3}' | sort -u)
(no output)
```

### Undefined (imported) symbols in the Rust `.so`

All remaining `U` entries in the Rust `.so` are libc / Rust-runtime imports
(`memcpy`, `__libc_start_main`-family, unwinder, `_ITM_*`,
`__cxa_thread_atexit_impl`, `__rust_*` shims). There are 0 missing/undefined
NON-libc symbols that the Rust library fails to provide.

### Rust-only extra symbols

The Rust cdylib additionally exports mangled `_ZN*` Rust-internal and
`rust_eh_personality` / `__rust_*` runtime symbols. Extra exports are harmless
for parity (the requirement is one-directional: every C symbol must exist in
Rust).

## Static (internal-linkage) C functions — translation completeness

Even though these are not exported, all 8 were checked to exist as private Rust
functions so that no C source was skipped:

| C static function | Rust counterpart | present |
|---|---|---|
| `memchra` | `fn memchra` | yes |
| `process_buffer` | `fn process_buffer` | yes |
| `int_to_float_bits` | `fn int_to_float_bits` | yes |
| `process_strings` | `fn process_strings` | yes |
| `safe_sum_array` | `fn safe_sum_array` | yes |
| `interpret_as_int` | `fn interpret_as_int` | yes |
| `count_occurrences` | `fn count_occurrences` | yes |
| `complex_iteration` | `fn complex_iteration` | yes |
| `snprintf` (libc, used by `memchra2`) | `fn snprintf_into` helper | yes |

No module or file of the C sources was left untranslated; no stubs or
`unimplemented!()` placeholders exist.

## Binaries

`c_src/CMakeLists.txt` builds `add_library(... SHARED src/lib.c)` only — there
is no driver executable, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. The "compare binary stdout"
requirement is therefore not applicable.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one (`--no-default-features` is also
behaviourally identical since there are no default features). All phases below
are run under that single configuration, plus both dev and release profiles.

---

## Results

Symbol diff is EMPTY for both the release and the debug Rust cdylib
(`verify.sh` step 3 enforces this, and `tests/phase_d_symbols.rs` asserts it as
a test, additionally `dlsym`-ing and CALLING both exports to prove the
`#[no_mangle] extern "C"` wrapper is real code with the right ABI, not just a
name in the symbol table).

```
OK (release): 0 missing symbols. C exports: memchra2
OK (debug):   0 missing symbols. C exports: memchra2
```

No C source was left untranslated, and no symbol is stubbed.
