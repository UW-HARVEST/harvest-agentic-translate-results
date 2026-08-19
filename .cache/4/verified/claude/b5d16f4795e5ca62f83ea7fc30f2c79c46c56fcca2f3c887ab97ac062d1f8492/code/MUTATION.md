# MUTATION.md — evidence that the differential suite can actually detect bugs

Passing tests prove nothing on their own: a suite that compares two stale or
empty byte streams passes perfectly. This file records the mutation testing used
to show the Phase B/C suite has real detection power, and the one harness bug it
uncovered.

Reproduce with the script pattern in this file's history: patch `src/lib.rs`,
`cargo build`, `cargo test`, then restore. Each mutant must be CAUGHT.

## The harness bug this found (important)

The first mutation run reported **10 of 10 mutants ESCAPED** — including
blatant ones like deleting the `NULL` guard. Root cause:

> `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` lib target,
> because the integration tests `dlopen` the library instead of linking it.

Every test had been validating an artifact from a previous build. The suite was
green and completely vacuous. Two fixes:

1. `tests/common/mod.rs::assert_fresh` compares the `.so` mtime against every
   `src/**/*.rs` (and `c_src/**/*.{c,h}`) and **refuses to run** when stale,
   instead of silently testing the wrong binary.
2. `verify.sh` always runs `cargo build` before `cargo test`.

A second, similar false-negative was also fixed: libtest's own progress output
(`test cfg_16_bad_single ... `) leaked into capture windows when tests ran on
parallel threads, producing bogus "DIVERGENCE" reports. Fixed by forcing
`RUST_TEST_THREADS = "1"` (`.cargo/config.toml`) plus a loud leak detector
(`detect_leak`) that panics rather than scrubbing bytes.

## Results after the fixes

| # | mutant | result | caught by |
|---|--------|--------|-----------|
| 1 | `printLine`: drop the `NULL` guard (`if !line.is_null()` → `if true`) | **CAUGHT** | `err_01_print_line_null`, `err_01b`, `err_generic_null_storm`, `err_generic_mixed_valid_and_invalid_stream` |
| 2 | `printLine`: treat empty as absent (also skip when `*line == 0`) | **CAUGHT** | `cfg_08_print_line_empty`, `cfg_14`, `cfg_24` |
| 3 | `printLine`: pass `line` AS the format string (`printf(line)`) | **CAUGHT** | `cfg_08`–`cfg_15`, `cfg_24` (9 tests) |
| 4 | `printIntLine`: `%d` → `%u` | **CAUGHT** | `cfg_03`, `cfg_04`, `cfg_05`, `cfg_06`, `cfg_07`, `cfg_24` |
| 5 | `printIntLine`: drop the trailing newline | **CAUGHT** | 17 tests |
| 6 | `printLine`: emit `\r\n` instead of `\n` | **CAUGHT** | `cfg_08`–`cfg_15`, `cfg_24` |
| 7 | `printLine`: truncate to 8 bytes (`%.8s`) | **CAUGHT** | `cfg_10`–`cfg_15`, `cfg_24` |
| 8 | `driver`: `useGood != 0` → `useGood == 1` | ESCAPED — *output-equivalent* | — |
| 9 | `driver`: invert the branch | ESCAPED — *output-equivalent* | — |
| 10 | `bad`: print `data[1]` instead of `data[0]` | ESCAPED — *output-equivalent* | — |
| 11 | `bad`: seed the region with `1`s instead of `0`s | ESCAPED — *output-equivalent* | — |

**7/7 observable mutants caught. The 4 escapes are provably unobservable**, not
coverage gaps — established from the C side, independently of the Rust:

```
$ ./equiv          # links against c_src/build/libdriver.so
bad ->0
good->0
drv0->0
drv1->0
```

- **8, 9** — `bad()` and `good()` differ only in `alloca` size (10 vs 40 bytes);
  both print `data[0]`, which the copy loop always sets to `0`. The C emits the
  identical byte string `"0\n"` on both branches, so **no** stdout-based test can
  observe which branch `driver` took. The C ground truth does not distinguish
  them, therefore neither can a differential test. This is asserted explicitly by
  `cfg_equivalence_premise_bad_and_good_are_output_identical`, which will start
  failing if the premise ever breaks (at which point `err_07b` becomes a real
  routing check).
- **10** — after the loop `data[1] == source[1] == 0 == data[0]`.
- **11** — the copy loop overwrites all ten slots with `source`'s zeros before
  anything is printed, so the initial contents cannot be observed.

Mutants 8 and 9 are the only source-level semantics not pinned behaviourally.
They are pinned by inspection instead: the Rust
`if useGood != 0 { good() } else { bad() }` is structurally identical to the C
`if (useGood) { good(); } else { bad(); }`. This limitation is inherent to the
library — it is recorded rather than hidden.
