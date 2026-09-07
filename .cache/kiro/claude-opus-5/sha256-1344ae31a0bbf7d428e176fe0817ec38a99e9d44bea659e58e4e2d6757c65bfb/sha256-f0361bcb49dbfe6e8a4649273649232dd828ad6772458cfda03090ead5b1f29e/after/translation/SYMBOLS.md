# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```
nm -D --defined-only c_src/build/libharvest-work-oCQEed.so      | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libconfusion_lib.so | awk '{print $3}' | sort
comm -23 c_syms.txt r_syms.txt      # => empty
```

## Dynamic symbols defined by the C `.so`

| # | symbol | C signature (from `src/lib.c`) | exported by Rust `.so` |
|---|--------|-------------------------------|------------------------|
| 1 | `confuse_types`  | `int confuse_types(ProcessState*, int)`            | yes |
| 2 | `confusion`      | `int confusion(int, int, int, int)`                | yes |
| 3 | `create_state`   | `ProcessState* create_state(int, int)`             | yes |
| 4 | `destroy_state`  | `void destroy_state(ProcessState*)`                | yes |
| 5 | `process_buffer` | `int process_buffer(ProcessState*, char)`          | yes |
| 6 | `update_flags`   | `void update_flags(ProcessState*, int)`            | yes |

Note: only `confusion` is declared in `include/lib.h`; the other five have
external linkage in `src/lib.c` and therefore appear in the dynamic symbol
table, so the Rust `.so` must export them too. There are no macro-generated
symbols (`STRINGIFY`, `DEBUG_VAR`, `LOG_OPERATION` expand to `printf` calls
only).

**Missing from Rust `.so`: NONE.** No module of the C source was left
untranslated; `src/lib.c` is the only translation unit in `CMakeLists.txt`.

## Undefined symbols in the Rust `.so`

All undefined symbols are libc / compiler-runtime imports
(`malloc`, `free`, `printf`, `snprintf`, `strlen`, `memcpy`, `_Unwind_*`,
`__cxa_finalize`, glibc stdio/syscall helpers, …).

**0 missing/undefined non-libc symbols.**

The C `.so` additionally imports `memchr`; the Rust translation implements the
`memchr` scan inline (`slice::iter().position()`), which is an implementation
detail and not an exported-surface difference.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and **no
`[[bin]]`** target (`crate-type = ["cdylib"]`, single `src/lib.rs`). Therefore
the only build configuration is the default one, and
`cargo test --no-default-features` is equivalent to `cargo test`. Both were
run (see PHASE_D notes at the bottom of `CONFIGS.md`).

## Verification record (re-checked after all fixes)

```
$ nm -D --defined-only c_src/build/libharvest-work-oCQEed.so | awk '{print $3}' | sort -u > c.txt
$ nm -D --defined-only translation/target/release/libconfusion_lib.so | awk '{print $3}' | sort -u > r.txt
$ comm -23 c.txt r.txt        # symbols in C but not Rust
(empty)
```

`phase_d.sh` re-runs this diff for **every** profile (`release`, `dev`) and
every feature combination (`--no-default-features`, default, `--all-features`)
and reports `symbol parity: OK (6 C symbols, 0 missing)` for all six
combinations, followed by the full differential test suite in each.

- [x] 0 symbols missing from the Rust `.so`
- [x] 0 undefined non-libc symbols in the Rust `.so`
- [x] No stubbed / `unimplemented!()` symbols — all six are real translations
      of `src/lib.c`, exercised by the differential tests
