# SYMBOLS.md — Phase A symbol surface

Generated mechanically from `nm -D` on both shared objects:

- C:    `c_src/build/libdriver.so`               (cmake, `add_library(driver SHARED src/driver.c)`)
- Rust: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)

## Defined (exported) symbols

Every `T`-type (defined, global text) symbol in the C `.so`, and its status in the Rust `.so`:

| # | symbol | C `.so` | Rust `.so` | Rust definition site |
|---|--------|---------|------------|----------------------|
| 1 | `printLine` | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine` |
| 2 | `bad`       | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn bad` |
| 3 | `good`      | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn good` |
| 4 | `driver`    | `T` | `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

**Exported-symbol diff (C minus Rust): EMPTY.** No C source file/module was
skipped by the translation: `src/driver.c` is the only translation unit in
`CMakeLists.txt`, and all five of its function definitions are present in
`src/lib.rs`. Nothing is stubbed or `unimplemented!()`.

## Deliberately NOT exported (must stay absent to match C)

These have internal linkage (`static`) in the C source, so they are absent from
the C `.so` dynamic symbol table. The Rust translation keeps them as private
(non-`no_mangle`) `fn`s, so they are likewise absent. Parity here is a
*negative* requirement — exporting them would be a divergence.

| # | symbol | C source | in C `.so`? | in Rust `.so`? |
|---|--------|----------|-------------|----------------|
| 5 | `helperGood` | `static void helperGood()` | no | no |
| 6 | `helperBad`  | `static void helperBad()`  | no | no |

Note: `helperBad` is dead code in the C source too — it is `static` and never
referenced by any function in `driver.c`. The Rust side mirrors this by keeping
`helperBad` defined but referenced only from a `#[used]` static, which silences
`dead_code` without adding a dynamic symbol.

## Undefined (imported) symbols

| library | non-libc undefined symbols |
|---------|----------------------------|
| C    | none (`puts@GLIBC_2.2.5` only) |
| Rust | none — all imports are glibc (`puts`, `malloc`, `memcpy`, `write`, …) or libgcc unwinder (`_Unwind_*`) |

The C `printf("%s\n", line)` is lowered by the C compiler to a `puts` call —
`U puts@GLIBC_2.2.5` is the *only* import of the C `.so`. The Rust translation
calls `puts` directly through `extern "C"` for exactly this reason, which also
keeps both implementations on the same libc `stdout` FILE stream and therefore
the same buffering discipline.

Rust's extra imports come from the statically linked `std`/`panic_unwind`
runtime that every `cdylib` carries. They are all libc/libgcc and resolve
against the system loader, so there are **0 missing/undefined non-libc
symbols**.

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so            | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
buildable configuration is the default one (equivalently
`--no-default-features`). Correspondingly the C source contains no `#ifdef`
compile-time configuration. The symbol table above is therefore complete for
every feature combination.

## Verification result

The documented `diff` was run against both the release and debug Rust `.so`s:

```
=== C vs translation/target/release/libdriver.so ===
--- symbols in C .so but MISSING from Rust .so: (none - diff is EMPTY)
=== C vs translation/target/debug/libdriver.so ===
--- symbols in C .so but MISSING from Rust .so: (none - diff is EMPTY)
```

C exported `T` symbols `{bad, driver, good, printLine}` are matched exactly by
the Rust `.so`. `helperGood` and `helperBad` are absent from both (0 occurrences
each), as required. 0 missing and 0 undefined non-libc symbols.

Enforced continuously by `tests/phase_d_symbols.rs` (`sym_01`–`sym_05`) rather
than only by a one-off shell command, so it keeps holding under every profile
and feature combination.

### Harness-integrity note

`sym_05` asserts the two `.so` files are not byte-identical and that the Rust
one imports strictly more than the C one. This exists because an early version
of the harness resolved the Rust `.so` path with a cross-profile fallback;
`cargo test` does not uplift a cdylib that no test links, so the suite silently
loaded a **stale** `target/release/libdriver.so` and a deliberate mutation went
undetected. The path resolution now prefers the freshest of
`target/<profile>/deps/libdriver.so` and `target/<profile>/libdriver.so`, and
`assert_fresh` rejects any `.so` older than `src/lib.rs` or `Cargo.toml`.
