# Verification report

Differential verification of the Rust translation in `translation/` against the
C ground truth in `c_src/`. The C is authoritative; every divergence found was
fixed on the Rust side (none were fixed in the C, and the C was never modified).

## How to reproduce

```sh
cd translation
scripts/run_all.sh          # builds C + Rust, checks symbol parity, runs everything
scripts/negative_control.sh # proves the suite can actually fail
```

Both `.so`s are loaded with `libloading` and driven **only** through their
exported `extern "C"` symbols — no Rust function is ever called directly, so the
`#[no_mangle]` export wrappers are themselves under test.

## Artifacts

| file | what it is |
|------|------------|
| `SYMBOLS.md` | every `nm -D` symbol of the C `.so`, matched against the Rust `.so` |
| `ERRORS.md` | the error-surface table: one row per distinct rejection in the C |
| `CONFIGS.md` | the configuration-surface table: one row per valid-input combination the C distinguishes |
| `tests/harness/mod.rs` | shared loader, `stderr` capture, observation helpers, seeded PRNG, allocation-failure injection |
| `tests/phase_b_configs.rs` | Phase B — 32 tests, one per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | Phase C — 27 tests, one per `ERRORS.md` row |
| `scripts/run_all.sh` | build + symbol diff + suite, for every feature combo × profile |
| `scripts/negative_control.sh` | mutation testing of the suite itself |

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` symbol diff between the C `.so` and the Rust
      `.so` is **empty** (7 exports each, same names). 0 undefined non-libc /
      non-runtime symbols; `ldd` resolves fully against `libc.so.6` +
      `libgcc_s.so.1`. No stubs, no `unimplemented!()`, no `todo!()`.
- [x] **Phase B** — all 32 `CONFIGS.md` rows pass, each across many randomized
      inputs from a fixed seed (`0x5EED_1234_ABCD_0001`).
- [x] **Phase C** — all 19 `ERRORS.md` rows (+7 `E12` sub-rows, +8 generic
      boundary rows) have a passing differential test asserting the *same*
      error code/sentinel and the same `stderr` bytes.
- [x] **Every feature combination** — `Cargo.toml` declares no `[features]`
      table and no optional dependencies, so the complete set is
      `<default>` / `--no-default-features` / `--all-features`, all three of
      which resolve to the same code. `scripts/run_all.sh` enumerates them from
      `Cargo.toml` and runs the whole suite for each, in both the `release` and
      the `debug` profile.

## What is compared

For every call, on both sides:

* the return value (`int`, or the pointer's null-ness);
* for `matrix_t*`: `width`, `height`, the row-array's null-ness, and **every
  cell** read back through `int**`;
* for `char*`: the full NUL-terminated byte string (then `free`d with the same
  `libc` `free`);
* for `write_to_file` / `driver`: the exact bytes of the file that was written
  (including `./matrix.txt` for `driver`);
* the **bytes written to `stderr`**, captured by redirecting fd 2 around each
  call. All three participants (test binary, C `.so`, Rust `.so`) share one
  `libc.so.6` and therefore one `stderr`, so this catches any difference in the
  diagnostic text, its formatting or its ordering.

Tests are serialised with an in-process mutex *and* an `flock`, because fd 2 and
`./matrix.txt` are process-global.

## Faithfully reproduced C bugs

These are defects in the original. They are reproduced verbatim rather than
"fixed", and the tests pin them down:

1. **`matrix_to_string` under-allocates.** `buffer_size = h*(w*10 + w) + h + 1`
   reserves 11 bytes per element plus **one** byte per row, but a row needs
   `strlen` per element plus `w-1` separators plus a newline. For `w >= 2` any
   row averaging more than 10 characters per element overruns the heap block.
   Consequence for testing: randomized rows keep `|value| <= 999_999_999`
   (≤ 10 chars) so the comparison observes defined behaviour, while the `w == 1`
   rows — where the sizing is *exactly* tight — sweep the full `i32` range
   including `INT_MIN`/`INT_MAX`. Documented in `CONFIGS.md`.
2. **`matrix_to_string` is quadratic.** It appends with `strcat`, rescanning the
   whole buffer on every element, so cost grows with the square of the output.
   This is why `ERRORS.md` row E18 is driven with a 250×250 result rather than
   something larger.
3. **`driver` leaks on one path.** When `matrix_to_string` returns `NULL`,
   `driver.c:56` calls plain `free(res)` instead of `free_matrix(res)`, leaking
   every row. Preserved in `src/driver.rs`.
4. **Unchecked `allocate_matrix` results.** Neither
   `initialize_matrix_from_string` nor `multiply_matrices` checks the pointer,
   so a failed allocation leads to a NULL dereference. Preserved; covered as
   undefined behaviour by G1/G2.
5. **`write_to_file` re-reads `errno`.** `strerror(errno)` is evaluated for the
   message, but the following `return errno;` reads `errno` again *after* the
   `fprintf` to `stderr`. Preserved in `src/write.rs`.
6. **Signed-overflow arithmetic.** `buffer_size` and the multiply accumulator
   overflow `int`. The C compiles to two's-complement wraparound at `-O0`; the
   Rust uses explicit `wrapping_*` everywhere so it matches, including in debug
   builds where Rust would otherwise panic on overflow.

## Build-profile note (not a translation difference)

An unchecked NULL dereference is undefined behaviour in C and compiles to a raw
fault. The shipped **release** Rust `cdylib` reproduces that exactly: `SIGSEGV`,
no output. A **debug** `cdylib` additionally carries rustc's `debug_assertions`,
which turn the same dereference into a `null pointer dereference occurred` panic
and `SIGABRT`. The two UB tests (G1, G2) therefore compare the exact signal and
`stderr` only when the artifact under test has assertions disabled
(`ub_strict()`, driven by `DIFFTEST_UB_STRICT`); otherwise they still require
both sides to terminate abnormally. Everything else is compared byte-for-byte in
both profiles.

`tests/harness/mod.rs` also refuses to run against a `.so` older than any
`src/*.rs`, so a stale artifact cannot masquerade as a passing run.

## Injecting allocation failures (E1, E5, E18)

Three `ERRORS.md` rows require a *specific* `malloc` to fail. `RLIMIT_AS` alone
cannot do it: a process always carries a pool of already-mapped-but-free heap —
measured at roughly 1 MiB even in a freshly started test binary — that requests
are carved out of without growing the address space. The harness therefore:

1. re-execs the test binary (`spawn_child`) instead of `fork`ing, so the child
   does not inherit whatever earlier tests allocated and freed;
2. calls `mallopt(M_MMAP_THRESHOLD, 32 MiB)` — glibc silently rejects anything
   above `DEFAULT_MMAP_THRESHOLD_MAX`, which is what made a first attempt at
   this fail — pinning all allocations to the `brk` heap so `free` cannot hand
   address space back with `munmap`;
3. pre-faults 1 MiB of stack, because once the address space is capped the
   kernel can no longer grow the stack and the process would die before
   reaching the code under test;
4. reserves the wanted budget, allocates until `malloc` fails, then releases the
   reservation — leaving *exactly* that budget allocatable
   (`constrain_heap_to`).

For E18 the budget window that starves `matrix_to_string` while still letting
`res` through was measured empirically as **384 KiB … 917 KiB**; the test uses
512 KiB, in the middle.

## Negative control

`scripts/negative_control.sh` builds mutated copies of the C library (in
`target/mutants`; `c_src` is never touched) and feeds each to the suite in place
of the Rust `.so`. Mutations: reworded diagnostics, transposed multiply indices,
`EINVAL`→`EPERM`, off-by-one `buffer_size`, an extra separator, a shifted row
index in a message, `abs()` on a negative height, and an `atoi` offset. **All are
caught**, which is what makes the all-green result above meaningful.

## Outcome

No behavioural divergence remains between the C `.so` and the Rust `.so` across
every configuration tested. The Rust source needed no changes: the four initial
test failures were faults in the *tests'* expectations about the C, and each was
corrected against measured C behaviour (see the "Result" sections of `ERRORS.md`
and `CONFIGS.md`).
