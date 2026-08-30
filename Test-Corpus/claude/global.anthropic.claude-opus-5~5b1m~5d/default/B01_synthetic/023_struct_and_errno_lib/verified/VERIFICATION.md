# Verification report

Differential verification of `translation/` (Rust) against `c_src/` (C, ground
truth). Both are built as shared objects; **every** call in every test goes
through `libloading`/`dlsym` into the exported C-ABI symbols of the two `.so`s.
The Rust crate is never called directly, so the `#[no_mangle] extern "C"`
wrappers are themselves under test.

## How to reproduce

```bash
./run_tests.sh        # build both .so's, run the suite in debug + release
./check_features.sh   # same, for every Cargo feature combination
```

`--test-threads=1` is mandatory (and set by both scripts): stdout is captured at
the file-descriptor level, which is process-global.

## Test inventory

| file | phase | tests | what it covers |
|------|-------|-------|----------------|
| `tests/harness/mod.rs` | — | (support) | dual `.so` loading, fd-level stdout capture, `house_t` mirror, fixed-seed `splitmix64` PRNG, independent model of the C algorithm |
| `tests/smoke.rs` | — | 4 | both `.so`s load and export both symbols; capture works and does not leak between calls |
| `tests/configs.rs` | B | 29 | one test per `CONFIGS.md` row C1–C30 |
| `tests/errors.rs` | C | 16 | one test per `ERRORS.md` row E1–E12 plus generic FFI-boundary hardening |
| `tests/symbols.rs` | D | 4 | `nm -D` parity, dlsym callability, no unresolved own-code symbols, `ldd -r` clean |

**53 tests, ~14 000 differential input comparisons per profile.**

Each comparison asserts:

1. stdout bytes are **byte-identical** between C and Rust, and
2. for `run`, the caller-visible mutation of `house_t` matches field-by-field,
   with `bathrooms` compared by **raw IEEE-754 bit pattern** (so `-0.0` vs `+0.0`
   and NaN sign/payload differences cannot hide), and
3. for the error rows, the output is exactly the 18-byte sentinel
   `"An error occurred\n"` — not merely "both failed somehow".

## Results

| gate | status |
|------|--------|
| `cargo check` clean | PASS (no errors, no warnings) |
| `SYMBOLS.md`: `nm -D` symbol diff C → Rust | **empty** — `driver`, `run` both exported; 0 unresolved non-libc symbols (`ldd -r` clean) |
| Phase B: every `CONFIGS.md` row (C1–C30) | PASS across randomized inputs |
| Phase C: every `ERRORS.md` row (E1–E12) | PASS |
| Feature combinations | 1 (no `[features]` declared); PASS |
| Profiles | `dev` **and** `release` both PASS |
| Extra hardening: `-C debug-assertions=on -C overflow-checks=on` | PASS |

## Bug found and fixed

One real divergence, visible only outside the release profile:

**`run(NULL, x)` terminated with `SIGABRT` in Rust but `SIGSEGV` in C.**
The translation formed a Rust reference from the incoming raw pointer
(`&mut *the_house`) and accessed fields through place expressions, both of which
make rustc emit a null/alignment validity check under `-C debug-assertions`;
that check `abort()`s. Fixed by reading and writing through raw-refs
(`&raw const` / `&raw mut`, which never dereference) plus `ptr::read`/
`ptr::write`, which lower to the same plain load/store the C emits. Details in
`ERRORS.md`.

## Harness bug found and fixed

`cargo test` does **not** rebuild a `cdylib`-only library, so the first version
of the suite was silently validating a **stale** `libdriver.so` — a deliberately
broken Rust build passed every test. Two changes make this impossible:

1. `crate-type = ["cdylib", "rlib"]` so the library is a build dependency of the
   integration tests (the tests still only ever `dlopen` the `.so`).
2. The harness resolves the **newest** candidate `.so` and hard-fails with a
   `STALE cdylib` message if it is older than `src/lib.rs`.

## Mutation testing (proof the suite actually discriminates)

The suite was validated by injecting deliberate faults into the Rust and
confirming each is caught:

| injected fault | tests failed |
|----------------|--------------|
| `%.1f` → `%.2f` in the format string | 28 |
| `bathrooms += 1.0` → `+= 1.0000000001` | 28 |
| `bathrooms += 1.0` → `+= 2.0` | 28 |
| `floors + 1` → `floors + 2` | 30 |
| `bedrooms + extra` → `bedrooms - extra` | 30 |
| `wrapping_add` → `saturating_add` (floors) | 3 |
| `wrapping_add` → `saturating_add` (bedrooms) | 27 |
| swap the `floors`/`bedrooms` printf arguments | 30 |
| `tmp <= INT_MAX` → `tmp < INT_MAX` | 6 |
| `tmp >= INT_MIN` → `tmp > INT_MIN` | 8 |
| drop the `endp != str` check | 3 |
| `strtol` base `10` → `0` | 7 |
| initial `bathrooms` `2.5` → `2.6` | 16 |
| error message text changed by one letter | 4 |
| call `run` once instead of twice in `driver` | 16 |

The only mutation *not* detected was deleting the `errno == 0` conjunct, which
is provably unobservable on an LP64 target — see the note in `ERRORS.md`.

## Notes on deliberate C behaviours replicated (not "fixed")

* **Trailing garbage is accepted.** `parse_val` checks `endp != str` but never
  `*endp == '\0'`, so `"12abc"` → 12, `"7.9"` → 7, `"0x10"` → 0.
* **Signed overflow wraps.** `floors++` and `bedrooms += extra_bedrooms` are UB
  in C; the un-optimised build wraps, so the Rust uses `wrapping_add`.
* **No null checks anywhere**, so `driver(NULL)`/`run(NULL, ..)` fault.
* **GCC rewrites the error `printf` into `puts`.** The Rust `.so` imports both
  `printf` and `puts` after LLVM's equivalent rewrite; the emitted bytes are
  identical, which the tests confirm exactly (18 bytes).
* Formatting and parsing are delegated to libc `printf`/`strtol` rather than
  reimplemented, so `%.1f` rounding (including half-to-even ties, subnormals,
  `±inf`, NaN, and ~310-digit expansions of `f64::MAX`) is identical by
  construction — and is still verified explicitly.
