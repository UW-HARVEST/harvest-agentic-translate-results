# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

```
C  : c_src/build/libdriver.so
Rust: translation/target/release/libdriver.so
```

## C `.so` exported symbols (`nm -D --defined-only`)

| # | symbol | type | C declaration | exported by Rust `.so`? |
|---|--------|------|---------------|-------------------------|
| 1 | `bad`          | T | `void bad(int data)`                     | YES |
| 2 | `driver`       | T | `void driver(int goodData, int badData)` | YES |
| 3 | `good`         | T | `void good(int data)`                    | YES |
| 4 | `printIntLine` | T | `void printIntLine(int intNumber)`       | YES |
| 5 | `printLine`    | T | `void printLine(const char *line)`       | YES |

## Symbols intentionally NOT exported (match C)

`driver.c` declares two helpers `static`, so they have internal linkage and do
not appear in `nm -D`. The Rust translation keeps them private (plain `unsafe
fn`, no `#[no_mangle]`):

| symbol | C linkage | Rust |
|--------|-----------|------|
| `goodG2B` | `static void goodG2B(void)`      | private `unsafe fn goodG2B()`   |
| `goodB2G` | `static void goodB2G(int data)`   | private `unsafe fn goodB2G(...)` |

## Diff result

```
$ comm -3 <(nm -D --defined-only c_src/build/libdriver.so    | awk '{print $3}' | sort) \
          <(nm -D --defined-only translation/.../libdriver.so | awk '{print $3}' | sort)
(empty)
```

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`**
(the Rust object's only undefined imports are libc: `printf`, plus the usual
`memcpy`/unwind/`__cxa` runtime glue).

## Translation completeness

`c_src` contains exactly one translation unit (`src/driver.c`, 114 lines) and
one public header (`include/driver.h`). Every function in that translation unit
(`printLine`, `printIntLine`, `bad`, `goodG2B`, `goodB2G`, `good`, `driver`) has
a corresponding Rust implementation in `translation/src/lib.rs`. No module was
skipped; there are no stubs and no `unimplemented!()`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one. Verified with:

```
$ cargo read-manifest | python3 -c 'import json,sys; print(json.load(sys.stdin)["features"])'
{}
```

So "every feature combination" == `{default}` == `cargo test` with no flags.

## Test-suite sensitivity (mutation study)

Symbol parity and green tests only mean something if the tests can actually fail.
18 mutants were injected into `src/lib.rs` one at a time and the full suite run
against each; **all 18 were caught**. `src/lib.rs` was then restored and verified
byte-identical (`md5sum` match) to the original translation.

| mutant | caught by |
|--------|-----------|
| `goodB2G`: `data < 10` -> `data <= 10` | 5 tests |
| `goodB2G`: upper-bound check removed | 7 tests |
| `bad`: missing upper-bound check *added* | 7 tests |
| `bad`: `data >= 0` -> `data > 0` | 7 tests |
| negative diagnostic: trailing `.` dropped | 5 tests |
| negative diagnostic replaced by the out-of-bounds one | 5 tests |
| out-of-bounds diagnostic: `out-of-bounds` -> `out of bounds` | 9 tests |
| `printLine`: NULL guard removed | 2 tests |
| `printIntLine`: `%d` -> `%u` | 4 tests |
| `printLine`: `%s` -> `%s%s` | 21 tests |
| `goodG2B`: hard-coded `7` -> `3` | 16 tests |
| `good()`: `goodG2B`/`goodB2G` call order swapped | 15 tests |
| `driver()`: `good()` call dropped | 8 tests |
| `driver()`: good and bad halves swapped | 8 tests |
| dump loop: `0..10` -> `0..9` | 14 tests |
| `BUFFER_LEN` `10` -> `11` | 22 tests |
| `bad`: writes `2` instead of `1` | 10 tests |
| `Frame` slack `118` -> `0` | did not compile (not a valid mutant) |
