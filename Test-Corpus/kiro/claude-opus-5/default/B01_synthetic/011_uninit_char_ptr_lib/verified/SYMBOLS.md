# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Defined (exported) symbols

The C library is built by CMake from a single translation unit (`src/driver.c`)
with no visibility attributes and no renaming macros, so linker names equal
source names.

| # | C symbol | type | exported by C `.so` | exported by Rust `.so` | status |
|---|----------|------|---------------------|------------------------|--------|
| 1 | `printLine` | `T` (text, global) | yes | yes | MATCH |
| 2 | `bad`       | `T` | yes | yes | MATCH |
| 3 | `good`      | `T` | yes | yes | MATCH |
| 4 | `driver`    | `T` | yes | yes | MATCH |

Only `driver` is declared in the public header (`include/driver.h`); `printLine`,
`bad` and `good` have external linkage in `driver.c` and are therefore part of
the `.so`'s dynamic symbol surface too. All four are treated as public entry
points and are tested directly (Phase B/C), not just through `driver`.

**Missing symbols: none.** No `#[no_mangle]` wrapper had to be added and no C
module was left untranslated — `src/driver.c` is the entire library. Nothing is
stubbed or `unimplemented!()`.

## Symbol diff

```
comm -3 <(nm -D --defined-only C   | awk '{print $NF}' | sort) \
        <(nm -D --defined-only RUST| awk '{print $NF}' | sort)
```

Result: **empty**. Exported-symbol parity is exact.

## Undefined (imported) symbols

The C `.so` imports only `puts@GLIBC_2.2.5` (gcc lowers
`printf("%s\n", line)` to `puts(line)`) plus the usual weak CRT hooks
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports `puts@GLIBC_2.2.5` — the same call the C makes, so
stdout formatting and buffering are shared — plus libc/`libgcc` runtime symbols
pulled in by the Rust standard library (`malloc`, `memcpy`, `write`, `_Unwind_*`,
`pthread_*`, …).

**0 missing/undefined non-libc symbols.** Every Rust import is either libc
(`GLIBC_*`), the unwinder shipped in `libgcc_s` (`_Unwind_*@GCC_*`), or a weak
CRT hook. There are no unresolved references to project code; verified by
loading the Rust `.so` with `dlopen(..., RTLD_NOW)`, which resolves every
relocation eagerly and succeeds.

## Feature / configuration surface

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. Neither side builds an executable
(`Cargo.toml` has no `[[bin]]`, `c_src/CMakeLists.txt` has no
`add_executable`), so there is no driver-binary stdout comparison to make.

## Automated check

Symbol parity is not just a one-off command; it is enforced by
`tests/phase_d_symbols.rs` (4 tests, all passing):

* `every_c_symbol_is_exported_by_rust` — diffs `nm -D --defined-only` between the
  two objects and requires the difference to be empty. It first asserts the C
  side actually contains `driver`/`printLine`/`bad`/`good`, so a parsing failure
  cannot produce a vacuously empty diff.
* `rust_has_no_unresolved_project_symbols` — requires every Rust import to be
  `@GLIBC_*`, `@GCC_*`/`_Unwind_*`, or a weak CRT hook, then `dlopen`s the Rust
  `.so` with `RTLD_NOW` as positive proof that nothing is left dangling.
* `both_libraries_resolve_all_four_entry_points_by_name` — resolves all four
  symbols through `dlsym` in **both** objects, proving the Rust exports are
  callable by an external consumer and not merely present in the symbol table.

Re-run with `cargo test --test phase_d_symbols`. Verified under all three
feature combinations via `scripts/all_features.sh`.
