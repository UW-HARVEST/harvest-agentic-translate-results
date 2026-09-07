# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` dynamic defined symbols (ground truth)

| # | symbol | C declaration | in Rust `.so`? | notes |
|---|--------|---------------|----------------|-------|
| 1 | `printLine`    | `void printLine(const char *line)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine` |
| 2 | `printIntLine` | `void printIntLine(int intNumber)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printIntLine` |
| 3 | `bad`          | `void bad(int data)`               | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn bad` |
| 4 | `good`         | `void good(int data)`              | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn good` |
| 5 | `driver`       | `void driver(int goodData, int badData)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

**Missing from Rust `.so`: NONE.** The symbol diff is empty in both directions
(no extra non-libc exports on the Rust side either).

## Deliberately NOT exported (correct)

`nm` on the C object (including local symbols) also shows:

```
000000000000121f t goodG2B
00000000000012a0 t goodB2G
```

Lower-case `t` = local. Both are `static` in `c_src/src/driver.c`, so they are
**not** part of the ABI. The Rust translation keeps them as private `fn goodG2B()`
/ `fn goodB2G(data)`. They are still exercised indirectly — they are the entire
body of the exported `good()`. Exporting them would be a *divergence*, not a fix.

Also local in C and irrelevant to the ABI (toolchain-generated):
`_init`, `_fini`, `deregister_tm_clones`, `register_tm_clones`,
`__do_global_dtors_aux`, `frame_dummy`.

## Undefined (imported) symbols

C imports: `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, plus the weak
`_ITM_*`/`__cxa_finalize`/`__gmon_start__` set.

> GCC rewrites `printf("%s\n", line)` into `puts(line)`, which is why the C `.so`
> imports `puts` as well. Byte-for-byte the emitted stream is identical
> (`line` followed by `\n`), so the Rust side using `printf("%s\n", …)` is
> ABI- and output-equivalent. Both libraries resolve to the *same* glibc
> `stdout` FILE object in-process, so buffering/interleaving also matches.

Rust imports: `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, and the usual Rust
`std` runtime set (`memcpy`, `malloc`, `mmap64`, `_Unwind_*`, `pthread_key_*`,
…). **0 missing/undefined non-libc symbols** — every undefined symbol in the
Rust `.so` is provided by glibc / libgcc_s, exactly as for any Rust `cdylib`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default one. `--no-default-features` and any
`--features <combo>` are therefore equivalent to the default build; the
"all feature combinations" gate collapses to the single default configuration.
Verified by `scripts/verify_all.sh`, which builds every combination, diffs
`nm -D` against the C `.so` for each, re-runs the Phase B/C/D suites for each,
and asserts all combinations export an identical symbol set.
`sym_06_no_cargo_features_declared` fails if a `[features]` section is ever
added without extending the matrix.

## Verdict

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
