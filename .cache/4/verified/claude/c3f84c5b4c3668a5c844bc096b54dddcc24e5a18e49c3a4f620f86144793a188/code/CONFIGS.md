# CONFIGS.md — Configuration-surface table (Phase A)

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from the
branches `c_src/src/lib.c` actually takes, not from guesses about what matters.

## Axis inventory (from the source)

There are **no runtime options/modes/flags** in this library: no setters, no
context struct, no global state, no env lookup, no byte-order or format
selector. Every branch is driven purely by argument values. The branch inventory
is therefore exactly:

**`cleanup(int a, int b, int c, int d)`**
* `A1` — per-argument `switch (numbers[i])` path. 5 distinct paths:
  `P10` (`case 10`, **falls through** into `case 20`, net `+30`),
  `P20` (`case 20`, breaks, `+20`),
  `P30` (`case 30`, **falls through** into `case 40`, net `+70`),
  `P40` (`case 40`, breaks, `+40`),
  `PD`  (`default`, `+v`).
* `A2` — slot position `i ∈ {0,1,2,3}`: the loop is `for (i=0;i<4;i++)` over
  `numbers[] = {a,b,c,d}`, so A1 is chosen **independently per slot** → the
  cross-product the code distinguishes is `5^4 = 625`.
* `A3` — value class fed to the `PD` (`default`) path: `0`, small positive,
  small negative, off-by-one around every case label
  (`9,11,19,21,29,31,39,41`), `INT_MAX`, `INT_MIN`, random.
* `A4` — accumulator overflow class: none / wraps past `INT_MAX` / wraps below
  `INT_MIN`.
* `A5` — the fixed-behaviour tail every non-early-exit call performs:
  `malloc(50)` → `snprintf(dst,50,"Processed numbers: %s", TO_STRING(numbers))`
  → `printf("%s\n",dst)` → `cleanup_resources`. `TO_STRING(numbers)`
  stringizes the *token* `numbers`, so the literal text is
  `Processed numbers: numbers` (26 bytes + NUL, fits the 50-byte buffer untruncated).

**`print_result(const char *label, int result)`** — lowest-level entry point,
pure `printf("%s: %d\n", label, result)` passthrough.
* `B1` — label shape: `NULL`, `""`, 1 char, short ASCII, exactly-`BUFSIZ`-ish
  and oversized (4 KiB / 64 KiB), embedded conversion specifiers, embedded
  control bytes (`\n \t \r`), non-UTF-8 high bytes `0x80..0xFF`, embedded NUL
  (truncates at the NUL).
* `B2` — `result` shape: `0`, positive, negative, `INT_MAX`, `INT_MIN`, random.

**`cleanup_resources(char *dynamic_str)`** — lowest-level entry point.
* `C1` — pointer shape: `NULL` (guard rejects) / genuine libc-`malloc`'d
  non-NULL (guard passes, `free` runs).

**Observables compared for every row:** the `int` return value **and** the exact
stdout byte stream (captured by `dup2`-redirecting fd 1 around each call and
`fflush(NULL)`-ing; both `.so`s share the one glibc `stdout`, so buffering is
identical by construction).

## Table

Every row is exercised through **both** `.so`s via `libloading` and asserted
byte-for-byte equal. Randomized rows use a fixed-seed splitmix64 PRNG keyed by
the iteration index, for reproducibility.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| B1 | `cleanup` | **A1×A2 exhaustive**: all `5^4 = 625` per-slot switch-path patterns, using one representative value per path (`10/20/30/40/7`). Covers fallthrough in every slot position and every mixture. | [x] |
| B2 | `cleanup` | A1=all-`PD`, A3=`0` — `cleanup(0,0,0,0)`, the all-default zero shape. | [x] |
| B3 | `cleanup` | A1=all-`PD`, A3=small positive random, no overflow — 2000 randomized quadruples in `1..=1000`. | [x] |
| B4 | `cleanup` | A1=all-`PD`, A3=small negative random — 2000 randomized quadruples in `-1000..=-1`. | [x] |
| B5 | `cleanup` | A1=all-`PD`, A3=**off-by-one around every case label**: full `4`-fold cross-product of `{9,11,19,21,29,31,39,41}` (4096 calls) — one step past each of the 4 case labels in both directions. | [x] |
| B6 | `cleanup` | A1=mixed case/default, A3=mixed sign — 4000 randomized quadruples drawn from a pool that mixes `{10,20,30,40}` with random ±values, so cases and defaults interleave in every position. | [x] |
| B7 | `cleanup` | A3=`INT_MAX`/`INT_MIN`/`0`/`±1` extremes, A4=**overflow in both directions**: full `4`-fold cross-product of `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` (2401 calls) — drives the accumulator past both signed bounds. | [x] |
| B8 | `cleanup` | A1=all-`PD`, A3=**unrestricted random `i32`** over the whole 2^32 range — 4000 randomized quadruples, uniform full-width. | [x] |
| B9 | `cleanup` | A5: the `malloc`/`snprintf`/`printf` tail — assert the emitted line is exactly `Processed numbers: numbers\n` (the `TO_STRING` token-stringization, **not** the values), identically from both, on every one of the above rows. | [x] |
| B10 | `cleanup` | Repeated invocation / statelessness: the same argument quadruple called 100× in a row must give an identical (return, stdout) pair every time and must not drift (no leaked/retained state between calls) — checked for both. | [x] |
| B11 | `print_result` | B1=short ASCII label, B2=`0` / positive / negative — the ordinary shape. | [x] |
| B12 | `print_result` | B1=`""` empty and 1-char labels × B2=`{0, 1, -1, INT_MAX, INT_MIN}` cross-product. | [x] |
| B13 | `print_result` | B1=oversized labels (4 KiB and 64 KiB of `'A'`) × B2 random — crosses glibc's internal `BUFSIZ` boundary. | [x] |
| B14 | `print_result` | B1=label containing `%d %s %n %% %p` × B2 random — must be emitted literally (it is a `%s` argument, not a format). | [x] |
| B15 | `print_result` | B1=label with control bytes `\n \t \r \v \f` and **non-UTF-8** bytes `0x80..0xFF` (all 128 high bytes) × B2 random — a `str`-based translation would corrupt these. | [x] |
| B16 | `print_result` | B1=label with an **embedded NUL** mid-buffer × B2 random — `%s` must stop at the NUL and ignore the tail, in both. | [x] |
| B17 | `print_result` | B1×B2 **randomized cross-product**: 2000 iterations of random-length (0..=300) random-byte labels × full-width random `i32` results. | [x] |
| B18 | `cleanup_resources` | C1=`NULL` — guard rejects, silent no-op (also called 100× repeatedly). | [x] |
| B19 | `cleanup_resources` | C1=genuine libc-`malloc`'d non-NULL buffer (sizes `1, 50, 4096`) — guard passes, `free` runs, no output. Freed once per allocation, once through the C `.so` and once through the Rust `.so` with independent allocations. | [x] |
| B20 | composed pipeline | `cleanup(...)` → feed its return value into `print_result(label, ret)` → `cleanup_resources(NULL)`, run end-to-end as a real consumer would, with 1000 randomized quadruples and random labels; the **whole concatenated stdout stream** of the 3-call sequence is compared, so ordering/interleaving/buffering of the composed pipeline is covered (not just per-wrapper output). | [x] |
| B21 | all three | Interleaved/alternating call ordering across the two `.so`s in one process (C call, then Rust call, then C, …) over randomized inputs — catches any cross-library state or allocator interference. | [x] |

**Coverage of the 5-path × 4-slot cross-product is exhaustive (row B1, 625/625),
so no per-slot switch behaviour is left to sampling.**

## Notes on how the rows are driven

* Rows are executed by `tests/phase_b_valid_paths.rs`, one function per row, all
  invoked from a single `#[test]` entry point. That is deliberate: `capture()`
  redirects the process-global fd 1, and `libtest` writes its own
  `test NAME ... ok` progress lines straight to fd 1, so running rows as parallel
  `#[test]`s let libtest's progress text land inside a capture window and produce
  bogus "divergences". One test per binary removes the race without depending on
  the caller passing `--test-threads=1`. Every row still runs even if an earlier
  one fails, and all failures are reported together.
* Randomized rows derive their inputs from a fixed-seed splitmix64 keyed by the
  iteration index (`mix(seed, i)`), so `body(i)` is pure. This matters: the batch
  driver replays the same closure once per library, and a stateful RNG would feed
  the two libraries different inputs and make every comparison meaningless.
* Rows are compared **batched** (all N iterations in one capture per library) so
  the composed multi-call stdout stream is itself part of what is asserted; on
  any mismatch the driver replays indices one at a time to localise the first
  divergent input.
* Total differential calls: ~22,800 per library for Phase B and ~4,700 for
  Phase C, i.e. roughly 55,000 cross-library comparisons per run.

## Anti-vacuity result

`mutation_check.py` injects 16 deliberate bugs into `src/lib.rs` (broken
fallthrough at each case label, wrong case constants, saturating instead of
wrapping overflow, `TO_STRING` printing values instead of the token, truncated
`snprintf` size, 3-of-4 loop bound, inverted/removed NULL guard, swapped
`printf` arguments, label routed through a Rust `str`, `cleanup` returning 0,
missing output line, and two pure leaks) and requires the suite to fail on each.

**Result: 16/16 caught, 0 surviving.** Two of those (`M9`, `M16`) and `M15` were
originally MISSED and are what motivated rows E5/E5b/E6 in `ERRORS.md`.
