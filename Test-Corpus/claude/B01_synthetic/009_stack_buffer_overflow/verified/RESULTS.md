# RESULTS.md — verification run log

Reproduce everything with:

```
./scripts/verify.sh
```

## Completion gate

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** | `0 of 5 C symbols missing from the Rust .so`; both objects `dlopen` with `RTLD_NOW` (every relocation resolved) and all five names `dlsym` successfully; Rust `DT_NEEDED` is `libgcc_s`/`libc`/`ld-linux` only |
| Phase B: every row in `CONFIGS.md` passes across randomized inputs | **PASS** | 48/48 rows checked off; `tests/ffi_diff.rs` (16), `tests/exe_diff.rs` (23), `tests/stdio_semantics.rs` (7) |
| Phase C: every row in `ERRORS.md` has a passing error-path differential test | **PASS** | 30/30 rows; `tests/error_paths.rs` (25) plus 4 in `tests/stdio_semantics.rs` |
| All of the above under every feature combination | **PASS** | there is exactly **one** combination — no `[features]` in `Cargo.toml`, no `#ifdef` in the C, no CMake options; `verify.sh` derives the powerset mechanically and runs it |

**82 tests, 0 failures**, stable across repeated runs of `scripts/verify.sh`.

## Test inventory

| file | tests | covers |
|------|-------|--------|
| `tests/smoke.rs` | 3 | both comparison channels work, **and can actually detect a difference** (guards against vacuous assertions) |
| `tests/symbol_parity.rs` | 5 | Phase D symbol diff, `RTLD_NOW` load, no `static` C functions exported, system-libraries-only |
| `tests/ffi_diff.rs` | 16 | Phase B through the `.so` exports, lowest-level entry points first |
| `tests/exe_diff.rs` | 23 | Phase B end-to-end, including the index classes that kill the process |
| `tests/error_paths.rs` | 25 | Phase C, one test per `ERRORS.md` row |
| `tests/stdio_semantics.rs` | 7 | C-stdio behaviors: SIGPIPE, tty vs pipe buffering, seekable stdin, C-consumer sharing |
| `tests/oob_band.rs` | 3 | the statistical envelope of the one unmatchable region |

Both channels load the Rust code as an **external consumer would** — `libloading` on
`target/release/libdriver.so`, never a direct Rust call — so the `#[no_mangle]` wrappers
are themselves under test. Each FFI call runs in a `fork`ed child with fd 0/1 redirected,
which is what makes it possible to observe a crash *and* the loss of the unflushed stdio
buffer.

## Divergences found and fixed

The translation was correct on all well-defined behavior from the start. Every defect
below was found by this verification and fixed in the Rust (never in the C).

| # | divergence | root cause | fix |
|---|-----------|------------|-----|
| 1 | index 16–19 / 26–27: C died with SIGSEGV and empty stdout, Rust exited 0 | the out-of-bounds store was silently skipped | modelled `bad()`'s gcc `-O0` frame from `objdump -d`; see the table in `src/imp.rs` |
| 2 | SIGPIPE: C killed by signal 13, Rust exited 0 | Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main` | `src/main.rs` restores `SIG_DFL` |
| 3 | on a **tty** C emitted 167/151/121 bytes before dying, Rust emitted 0 | `BufWriter` is unconditionally block-buffered; C line-buffers on a terminal | I/O layer rewritten onto glibc stdio |
| 4 | seekable stdin left at EOF instead of the logical offset | Rust's `io::Stdin` slurps 8 KiB into its own `BufReader` | same |
| 5 | a C consumer's `printf` did not interleave with `printLine` in call order | the exports used a private `BufWriter` and flushed eagerly | same |
| 6 | a C consumer's own `fgets` did not cooperate with `bad()`/`good()` | Rust read through its own `io::Stdin`, not the shared `FILE` | same |
| 7 | the fatal index set was hardwired to the executable's call chain, so the exported `bad()` diverged at 26/27 (Rust died, C did not) and 50/51 | indices ≥ 20 reach the *caller's* frame, whose layout the library cannot see | split into a caller-independent set `{18,19}` and a `Caller::CMain` set `{16,17,26,27}` |
| 8 | crash *timing* was wrong: Rust died before printing the ten values | in C the store does not fault; a later `ret` does | `Death` enum defers the fault to `bad`'s return or `main`'s return |
| 9 | `ERRORS.md` row 13 claimed `atoi("-999999999999")` stayed negative | it truncates to **+727379969** — the sign is not preserved | doc corrected and the value now asserted in `err_atoi_neg_truncation` |
| 10 | `scripts/verify.sh` printed PASS when `/tmp` was read-only | unchecked redirects into `/tmp` | honors `TMPDIR`, probes writability, and fails on `0 tests run` |
| 11 | `cargo test` failed to link | the `#[no_mangle] main` export collides with the unit-test harness entry point | `test = false` on `[lib]`; all tests are integration tests |

### Aggregate effect

A 940-case differential fuzz over the two executables (structural cases, every index in
`-5..45`, 400 random integers, 400 random byte strings):

| | divergences |
|---|---|
| before | **37 / 940** |
| after | **6 / 940** |

## The one irreducible region

The 6 residual cases are all `bad()` indices in `3613..4567`, and they are **not** Rust
bugs — C is nondeterministic there. Re-running each of the six inputs 30 times on both
binaries:

| input | C | Rust |
|-------|---|------|
| `12345678901234567\n` | 20 crash / 10 ok | 19 crash / 11 ok |
| `-202740845\n3737\n` | 4 crash / 26 ok | 4 crash / 26 ok |
| `28\n3613\n` | 6 crash / 24 ok | 5 crash / 25 ok |
| `3855\n3971\n` | 8 crash / 22 ok | 12 crash / 18 ok |
| `-7\n3848\n` | 12 crash / 18 ok | 11 crash / 19 ok |
| `-11\n3803\n` | 4 crash / 26 ok | 5 crash / 25 ok |

Whether `buffer[data] = 1` falls past the top of the `[stack]` mapping depends on where
stack ASLR put the stack on that particular execution, so **the C binary disagrees with
itself** run to run. The Rust distribution tracks C's closely; a single-shot comparison
lands on a mismatch only because both sides are flipping the same coin. No implementation
can be byte-identical here.

`src/imp.rs` reads the live `[stack]` bounds from `/proc/self/maps` so the boundary tracks
the environment automatically (it moves with the size of the environment block: with an
empty environment even index 500 sometimes faults, while under a typical inherited
environment index 1300 is still benign 12/12). Tests assert exact equality strictly
outside the band — the safe limit is *probed at run time* rather than hardcoded — and
`tests/oob_band.rs` asserts the statistical envelope so that deleting the emulation would
still be caught rather than silently passing.

See `CONFIGS.md` § "Known-unmatchable regions" for the two related cases: which fatal
signal a far write raises (`SIGSEGV` vs `SIGBUS`, also ASLR-dependent), and indices that
reach an arbitrary consumer's frame through the `.so` export (dependent on that
consumer's code generation, and observed both ways with the same C `.so`).
