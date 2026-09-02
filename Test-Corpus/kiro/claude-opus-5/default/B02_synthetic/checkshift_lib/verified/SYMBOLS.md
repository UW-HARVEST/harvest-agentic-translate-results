# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
C_SO=c_src/build/libharvest-work-PwHu6w.so
R_SO=translation/target/release/libcheckshift_lib.so
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # missing from Rust  -> MUST be empty
```

The whole C library is one translation unit (`c_src/src/lib.c`, built by
`c_src/CMakeLists.txt` as a single `SHARED` target). There are no macro-generated
exported symbols: `STRINGIFY` / `LOG_VALUE` expand only inside function bodies.

## Defined dynamic symbols

C `.so`: **10**  ·  Rust `.so`: **10**  ·  missing from Rust: **0**  ·  extra in Rust: **0**

| symbol | C `.so` | Rust `.so` | status |
|--------|---------|------------|--------|
| `add_with_static` | T | T | ok |
| `apply_operation` | T | T | ok |
| `checkshift` | T | T | ok |
| `compute_checksum` | T | T | ok |
| `execute_operation` | T | T | ok |
| `get_operation` | T | T | ok |
| `init_state` | T | T | ok |
| `multiply_with_static` | T | T | ok |
| `shift_with_static` | T | T | ok |
| `xor_operation` | T | T | ok |

`checkshift` is the only symbol declared in the public header
(`c_src/include/lib.h`); the other nine have external linkage in `lib.c` (they are
not `static`) and are therefore part of the C `.so`'s ABI surface. All nine are
tested directly through the `.so` exports, not only via `checkshift`.

The three file-scope `static int`s (`static_multiplier`, `static_addend`,
`static_shift_amount`) have internal linkage and are correctly **not** exported by
either library.

## Undefined symbols

Both libraries import only libc / runtime symbols. The C `.so` imports
`malloc`, `free`, `memcpy`, `printf`, `puts` (gcc rewrites the no-vararg
`printf("…\n")` calls to `puts`, which emits identical bytes). The Rust `.so`
imports the same five plus the Rust `std`/unwind runtime set (`_Unwind_*`,
`__tls_get_addr`, `mmap64`, `dl_iterate_phdr`, …).

**0 missing/undefined non-libc symbols in the Rust `.so`.**

Verified by `tests/symbols.rs::symbol_parity_c_vs_rust`, which re-runs the `nm -D`
diff at test time so the gate cannot silently rot, and by
`every_symbol_is_dlsym_able_from_both`, which requires each name to be actually
`dlsym`-able (present in `nm` is not the same as callable).

### Libc call parity

Exported-symbol parity is necessary but not sufficient: the *calls* a library makes
into libc are also part of its observable behaviour when an interposer is present,
and one of them turned out to matter.

* `malloc` / `free` — **had diverged.** LLVM recognises them by name and had deleted
  the Rust `.so`'s 12-byte `ComputeState` allocation, which removed `checkshift`'s
  allocation-failure branch. Fixed via `read_volatile` function-pointer
  trampolines; see the finding in `ERRORS.md` and the
  `err18b_allocator_call_parity` guard.
* `printf` / `puts` — both libraries reach the same process-wide `stdout`. gcc
  rewrites the C library's no-vararg `printf("…\n")` calls to `puts` (hence the
  `puts` import); the emitted bytes are identical and this is asserted, not assumed.
* `memcpy` — call counts differ (C calls it for the small fixed-size struct copies,
  Rust inlines them). Examined and deliberately not "fixed": `memcpy` cannot fail,
  no C branch is keyed on it, and no return value, state byte or emitted byte
  changes. See the note in `ERRORS.md`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent to the
default here). `tests/symbols.rs::no_cargo_features_declared` asserts this, so
that adding a feature later forces the Phase D matrix to be revisited.
`scripts/check_features.sh` enumerates and builds/tests every combination.
