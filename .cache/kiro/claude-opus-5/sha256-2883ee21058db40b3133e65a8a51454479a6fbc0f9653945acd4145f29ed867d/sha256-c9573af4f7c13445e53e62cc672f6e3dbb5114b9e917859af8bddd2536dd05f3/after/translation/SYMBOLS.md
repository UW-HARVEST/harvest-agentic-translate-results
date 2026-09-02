# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported (dynamic, defined) symbols

`nm -D --defined-only c_src/build/libdriver.so`

```
00000000000011b5 T driver
```

## Rust `.so` exported (dynamic, defined) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

```
0000000000011770 T driver
```

Both `.so`s export exactly one dynamic, defined symbol, and it is the same one.
The diff is empty:

```
diff <(nm -D --defined-only c_src/build/libdriver.so         | awk '{print $2, $3}') \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $2, $3}')
# -> no output
```

## Parity table

| # | C symbol | type | exported by Rust `.so`? | notes |
|---|----------|------|-------------------------|-------|
| 1 | `driver` | `T` (global text) | YES — `T driver` | `#[unsafe(no_mangle)] pub extern "C" fn driver(c_int, c_int, c_int)` in `src/lib.rs` |

## Symbols in the C source that are NOT exported (and must not be)

Derived from `c_src/src/driver.c`:

| C entity | linkage in C | Rust counterpart | exported? |
|----------|--------------|------------------|-----------|
| `static int y = 123;` | internal (file-scope static) | `static Y: AtomicI32 = AtomicI32::new(123)` | no — correct |
| `static int multi_stage(int x, int z)` | internal (`static` function) | `fn multi_stage(x: c_int, z: c_int) -> c_int` | no — correct |

`nm --defined-only c_src/build/libdriver.so` confirms `multi_stage` is `t`
(local text) and `y` is `d` (local data), matching the Rust side where both are
private items and neither appears in `nm -D`.

## Undefined (imported) symbols

C `.so` imports, from `nm -D --undefined-only`:

```
U printf@GLIBC_2.2.5
U puts@GLIBC_2.2.5
w _ITM_deregisterTMCloneTable, _ITM_registerTMCloneTable, __cxa_finalize, __gmon_start__
```

(gcc rewrites the four no-conversion `printf("...\n")` calls into `puts`, and
keeps real `printf` for `"Result: %d\n"`.)

Rust `.so` imports the same `printf` and `puts` from libc, plus the Rust
`std`/`libgcc` runtime set (`malloc`, `free`, `memcpy`, `memset`, `mmap64`,
`pthread_key_*`, `_Unwind_*`, `dl_iterate_phdr`, …). Every one resolves to
libc / libgcc / the platform loader — there are **no undefined
translation-specific symbols**.

## Result

- [x] Every symbol the C `.so` exports is also exported by the Rust `.so`, with
      the exact same name.
- [x] 0 missing symbols.
- [x] 0 undefined non-libc symbols in the Rust `.so`.
- [x] No stubs, no `unimplemented!()`, no `todo!()` anywhere in `src/`
      (verified: `grep -rn 'unimplemented!\|todo!\|panic!("not' translation/src`
      returns nothing).

## How this is re-checked

`translation/verify.sh` re-derives everything above mechanically: it builds the C
`.so` if absent, enumerates the powerset of the features in `Cargo.toml`, and for
each combination runs `cargo check`, builds the `cdylib`, `comm`-diffs the two
symbol sets, checks for undefined non-libc imports, and runs the full
differential suite.

Negative controls confirming the checks are not vacuous:

| control | result |
|---------|--------|
| replace `#[unsafe(no_mangle)]` with `#[allow(dead_code)]` | `symbol parity FAILED — missing from Rust .so`, `PHASE D: FAIL` |
| add two dummy features to `Cargo.toml` | enumerated 5 combinations (default + 4 subsets), each verified |

### Harness pitfall found and fixed

With `crate-type = ["cdylib"]`, `cargo test` compiles `src/lib.rs` **only** as a
test harness — it never emits `libdriver.so`. The first version of the test
harness merely *searched* `target/{debug,release}` for the `.so` and silently
`dlopen`ed a stale artifact from an earlier `cargo build --release`. Six injected
source mutations all went undetected. `tests/common/mod.rs` now shells out to
`cargo build --release --lib --target-dir target/difftest` before `dlopen`, and
asserts the resulting `.so` is newer than every file in `src/`.

Re-run of the same six mutations against the fixed harness — all detected:

| mutation | detected |
|----------|----------|
| `if z != 3` → `if z != 4` | yes (10 tests fail) |
| `"Ok!\n"` → `"OK!\n"` | yes |
| drop `Y.store(local_y, …)` | yes |
| `result = 2` → `result = 3` | yes |
| `if x != 1` → `if x != 0` | yes |
| `"Result: %d\n"` → `"Result: %d \n"` | yes |
| unmutated baseline | 0 failures |

`.cargo/config.toml` sets `RUST_TEST_THREADS = "1"`, because the harness observes
output by `dup2`-ing over file descriptor 1 and libtest's own progress writes
would otherwise be captured by a concurrent test (this produced exactly one
false-positive "divergence" before it was fixed).
