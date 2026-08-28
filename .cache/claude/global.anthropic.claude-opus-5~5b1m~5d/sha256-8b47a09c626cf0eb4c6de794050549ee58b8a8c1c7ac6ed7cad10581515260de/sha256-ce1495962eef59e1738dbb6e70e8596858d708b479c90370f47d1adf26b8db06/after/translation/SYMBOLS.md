# SYMBOLS.md — Phase A: exported-symbol surface

Mechanically derived from `nm -D` on both shared objects.

```
C   : c_src/build/libharvest-work-K67KsE.so      (cmake target name == parent dir name)
Rust: translation/target/release/libcleanup_lib.so
```

Regenerate with:

```sh
nm -D --defined-only c_src/build/lib*.so                     | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libcleanup_lib.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # MUST be empty
```

## C source inventory (completeness check)

`c_src` contains exactly one translation unit and one public header, so there is
no possibility of a whole module having been skipped:

| C file | contents | translated in |
|--------|----------|---------------|
| `c_src/include/lib.h` | declares `cleanup` | `translation/src/lib.rs` |
| `c_src/src/lib.c` | defines `cleanup`, `print_result`, `cleanup_resources`; macros `STRINGIZE`, `TO_STRING` | `translation/src/lib.rs` |

`STRINGIZE` / `TO_STRING` are preprocessor-only (no symbol emitted); their
expansion `TO_STRING(numbers)` == the literal `"numbers"` is materialised in the
Rust as the constant `STRINGIZED_NUMBERS`.

## Defined (exported) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | signature | status |
|---|--------|---------|------------|-----------|--------|
| 1 | `cleanup`           | `T` | `T` | `int cleanup(int,int,int,int)`      | present in both |
| 2 | `print_result`      | `T` | `T` | `void print_result(const char*,int)`| present in both |
| 3 | `cleanup_resources` | `T` | `T` | `void cleanup_resources(char*)`     | present in both |

`comm -23 c.syms r.syms` → **empty**: 0 symbols missing from the Rust `.so`.
No stubs were introduced; all three are full translations of the C bodies.

The Rust `.so` additionally exports nothing else (no `rust_eh_personality`,
no mangled items) — `crate-type = ["cdylib"]` plus `#[unsafe(no_mangle)]`
yields exactly the three C names.

## Undefined (imported) symbols — libc / unwinder only

C `.so` imports: `free`, `malloc`, `printf`, `puts`, `snprintf`, `strlen`,
`strncmp` (all `@GLIBC_2.2.5`), plus the weak `_ITM_*`, `__cxa_finalize`,
`__gmon_start__`.

Note `puts`: gcc rewrites `printf("%s\n", p)` → `puts(p)`. LLVM performs the
same rewrite for the Rust translation, so the Rust `.so` also imports `puts`
and both libraries emit byte-identical stdout through the same stream.

Rust `.so` imports the same libc entry points that matter for behaviour
(`malloc`, `free`, `printf`, `puts`, `snprintf`, `strlen`, `strncmp`) plus the
usual Rust runtime set (`_Unwind_*`, `memcpy`, `mmap64`, `pthread_key_*`, …).
There are **0 undefined non-libc symbols**: every import resolves out of
`libc.so.6` / `libgcc_s.so.1`, verified with

```sh
ldd -r translation/target/release/libcleanup_lib.so   # no "undefined symbol" lines
```

## Feature matrix

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
complete feature powerset is a single configuration: the (empty) default.
`--no-default-features` and `--all-features` resolve to the same unit. Both are
still exercised explicitly by `tests/run_all.sh`, in debug and release, because
the release profile applies materially different optimisations (e.g. the
`printf` → `puts` rewrite above).
