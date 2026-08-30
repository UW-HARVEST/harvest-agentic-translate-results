# MUTATION.md — proof the differential suite actually bites

Passing tests only mean something if they can fail. Every mutant below was
injected into `src/lib.rs`, the whole suite was re-run, and `src/lib.rs` was
restored from a pristine copy afterwards (verified with `cmp`).

## A real harness bug this exposed

The first mutation run reported **all mutants passing**. The cause was not the
tests but the harness: `cargo test` builds the test binaries but **not** the
`cdylib` artifact, so `tests/harness/mod.rs` was `dlopen`ing a stale
`target/debug/libdriver.so` from an earlier `cargo build`. Every source change —
and therefore every injected bug — was invisible.

Fixed in `tests/harness/mod.rs`:

* the cdylib is now **always** rebuilt before loading, never "built only if missing";
* a freshness assertion fails loudly if `libdriver.so` is older than `src/lib.rs`;
* the nested `cargo build` has its stdout sent to `/dev/null` and is forced to
  run *before* fd 1 is redirected, so it can never pollute a capture.

Two further harness bugs were found and fixed the same way:

* libtest's own progress output ("`test cfg_… ok`") was landing inside capture
  windows because tests ran on parallel threads. Fixed by pinning
  `RUST_TEST_THREADS = "1"` in `.cargo/config.toml`, asserting it at capture
  time, and flushing Rust's `stdout` before each redirect.

## Results — 11 of 14 mutants caught

| # | mutation | result |
|---|----------|--------|
| 1 | `printLine`: `"%s\n"` → `"%s"` (drop newline) | **CAUGHT** (12 tests) |
| 2 | `printLine`: `"%s\n"` → `"[%s]\n"` | **CAUGHT** (12 tests) |
| 3 | `printLine`: NULL guard removed (`if !line.is_null()` → `if true`) | **CAUGHT** (3 tests) |
| 4 | `printIntLine`: `%d` → `%u` | **CAUGHT** (7 tests) |
| 5 | `printIntLine`: `%d` → `%ld` (wrong vararg width) | **CAUGHT** (7 tests) |
| 6 | `printIntLine`: prints `intNumber + 1` | **CAUGHT** (15 tests) |
| 7 | `printIntLine`: prints `intNumber.abs()` | **CAUGHT** (7 tests) |
| 8 | `bad()`: prints `1` instead of `data[0]` | **CAUGHT** (7 tests) |
| 9 | `good()`: prints `1` instead of `data[0]` | **CAUGHT** (7 tests) |
| 10 | `good`: `#[unsafe(no_mangle)]` removed | **CAUGHT** (parity test) |
| 11 | `good`: exported as `good_TYPO` | **CAUGHT** (parity test) |
| 12 | `driver`: `useGood != 0` → `useGood == 1` | escaped — **equivalent** |
| 13 | `driver`: branches swapped (`useGood == 0`) | escaped — **equivalent** |
| 14 | `bad()`: copy loop runs 9 times instead of 10 | escaped — **equivalent** |

## Why 12–14 are equivalent mutants, not test gaps

**12 and 13.** In the C, `good()` and `bad()` emit byte-identical output.
Verified directly against the C `.so`:

```
good -> 0
bad  -> 0
```

`driver`'s only effect is choosing between two functions with indistinguishable
observable behaviour, so **no** test driving the library through its public
surface can detect which branch was taken. This is a property of the C, not a
weakness of the suite. The dispatch predicate is therefore verified by source
correspondence instead — C `if (useGood)` ↔ Rust `if useGood != 0` — and the
fact is pinned by `parity_good_and_bad_are_observationally_identical`, which
will start failing the moment `good()` and `bad()` ever diverge, at which point
mutants 12–13 become detectable.

Note that the suite *does* still constrain `driver` as tightly as is observable:
`err_e8_driver_out_of_range_int` and `boundary_out_of_range_enum_values` assert
that `driver(v)` reproduces `good()`'s bytes for every non-zero `v` (including
`INT_MIN`, `INT_MAX`, every single-bit value, and values whose low byte is zero
such as `0x100` — which would catch a translation that truncated the flag to
`bool`/`u8`) and `bad()`'s bytes for `v == 0`.

**14.** `bad()` prints only `data[0]`, which the loop writes on its very first
iteration; `source` is all zeros. Trimming the loop from 10 to 9 iterations
cannot change `data[0]`, so the printed value is `0` either way. (In the C the
`alloca` memory is uninitialised, but `data[0]` is always assigned before the
read, so this holds for the C as well.)

## Reproducing

```sh
cp src/lib.rs /tmp/lib.rs.pristine
sed -i 's|b"%d\\n\\0"|b"%u\\n\\0"|' src/lib.rs   # inject
cargo test --offline                              # observe failures
cp /tmp/lib.rs.pristine src/lib.rs                # restore
cmp /tmp/lib.rs.pristine src/lib.rs               # verify restored
```
