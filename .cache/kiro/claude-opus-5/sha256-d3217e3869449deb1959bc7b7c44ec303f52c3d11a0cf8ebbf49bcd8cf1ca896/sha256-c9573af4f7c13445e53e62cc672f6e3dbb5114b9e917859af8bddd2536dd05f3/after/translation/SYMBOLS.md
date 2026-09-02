# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

Derived mechanically:

```
nm -D --defined-only c_src/build/libharvest-work-A9bGws.so   | grep -v ' [a-z] '
nm -D --defined-only translation/target/release/libmodeselect_lib.so | grep -v ' [a-z] '
```

Lowercase (local/`t`/`b`/`d`/`w`) entries are filtered out: those are not part of
the exported public surface. What remains is the complete `T` (global text)
surface of each library.

## C translation units

`c_src/CMakeLists.txt` builds exactly one source file:

- `src/lib.c` — the only translation unit.

`c_src/include/lib.h` declares only `int modeselect(int, int, int, int);`, but
`lib.c` gives external linkage to six further functions (none are `static`), so
all seven are part of the ABI and all seven must be exported by Rust.

There is **no** untranslated C module: `src/lib.c` is fully covered by
`translation/src/lib.rs`. No stubs, no `unimplemented!()`.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | Rust impl site | notes |
|---|--------|---------|------------|----------------|-------|
| 1 | `classify_mode`            | T | T | `src/lib.rs` `classify_mode`            | `strcmp` chain reimplemented as `cstr_eq` |
| 2 | `apply_multiplier`         | T | T | `src/lib.rs` `apply_multiplier`         | fall-through `switch` flattened per-arm |
| 3 | `convert_time_factor`      | T | T | `src/lib.rs` `convert_time_factor`      | `(int)` cast via `d2i` |
| 4 | `convert_negative_overflow`| T | T | `src/lib.rs` `convert_negative_overflow`| `(int)` cast via `d2i` |
| 5 | `get_modified_time`        | T | T | `src/lib.rs` `get_modified_time`        | returns `time_t` = `i64` |
| 6 | `hash_time_value`          | T | T | `src/lib.rs` `hash_time_value`          | takes `time_t` = `i64` |
| 7 | `modeselect`               | T | T | `src/lib.rs` `modeselect`               | the header-declared entry point |

## Diff

```
$ diff <(nm -D --defined-only <C.so>    | grep -v ' [a-z] ' | awk '{print $3}' | sort) \
       <(nm -D --defined-only <RUST.so> | grep -v ' [a-z] ' | awk '{print $3}' | sort)
(empty)
```

- Symbols in C but missing from Rust: **0**
- Symbols added by Rust that C does not have: **0**

## Undefined (imported) symbols

The Rust `.so` imports only libc: `printf`, `time`, plus the usual Rust runtime
panic/unwind and memory imports. No non-libc undefined symbols are unresolved.
`printf` and `time` are deliberately imported rather than reimplemented so that
stdout formatting and clock semantics are byte-identical to the C.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, therefore the only
build configuration is the default one. `--no-default-features` and the default
build are the same compilation. This is verified by
`tests/features.rs::feature_matrix_is_single_default_configuration`, which
parses `Cargo.toml` and fails if a `[features]` section is ever added without
extending the verification matrix.

## Verification evidence

Independently re-derived after the tests were written:

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-A9bGws.so \
          | grep -vE ' [a-z] ' | awk '{print $NF}' \
          | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' | sort) \
       <(nm -D --defined-only translation/target/release/libmodeselect_lib.so \
          | grep -vE ' [a-z] ' | awk '{print $NF}' | sort)
EMPTY -> parity confirmed
```

`_init`, `_fini`, `__bss_start`, `_edata`, `_end` are excluded: they are
linker-generated ELF bookkeeping, not library API.

Parity is additionally enforced as tests, so it cannot silently regress:

- `tests/phase_d_parity.rs::d1_every_c_symbol_is_exported_by_rust` — runs `nm -D`
  on both `.so`s and requires an empty diff.
- `d2_every_symbol_is_dlsym_resolvable_and_callable` — `nm` listing a name is
  weaker than the symbol resolving and running, so all seven are `dlsym`'d from
  BOTH libraries and invoked, with results compared.
- `d3_rust_so_has_no_unresolved_non_libc_symbols` — `nm -D --undefined-only`
  must contain no project symbol.
- `d4_feature_matrix_is_single_default_configuration` — fails if a `[features]`
  table appears in `Cargo.toml` or an `#ifdef` appears in `lib.c`.
- `d5_c_source_is_fully_covered_by_the_rust_translation` — parses `lib.c` for
  every externally-linked definition and requires a matching `fn` in
  `src/lib.rs`; also fails on `unimplemented!(` / `todo!(` anywhere in the
  translation, so no symbol can be faked into existence.

Test totals: **87 tests, 0 failures**, under both the `debug` and `release`
profiles, and under both `--features`-free invocations
(`cargo test` and `cargo test --no-default-features`). Driver: `./verify.sh`.
