# Differential testing: mismatches found and fixed

The C in `c_src/` is ground truth. Both programs were built and run as
subprocesses on identical stdin, comparing stdout, stderr and exit status
(including the terminating signal).

Run commands:

| program | build | run |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` | `translation/target/release/driver` |

Test suite: `translation/tests/differential.rs` (24 tests, none ignored or
skipped). It builds the C binary itself if `c_src/build/driver` is absent, so
`cargo test` works from a clean checkout.

## What the program does

`main` calls `good()` then `bad()`. Reads happen in this order against a single
`stdin` stream, so the *first* line feeds `goodB2G()` and the *second* feeds
`bad()`:

1. `goodG2B()` — no input, `data = 7`, prints 10 values.
2. `goodB2G()` — `fgets(buf, 14, stdin)` + `atoi`, sink guarded by
   `data >= 0 && data < 10`.
3. `bad()` — `fgets(buf, 14, stdin)` + `atoi`, sink guarded only by
   `data >= 0`. This is the CWE-129 / CWE-787 out-of-bounds stack write.

`fgets` with size 14 stores at most 13 bytes, so any line longer than 13
characters is **split** between the `goodB2G` read and the `bad` read.

---

## Mismatch 1 — out-of-bounds indices that smash `bad()`'s frame

**Symptom.** For a second-line value of 16, 17, 18, 19, 26 or 27 the C died from
SIGSEGV producing *no output at all*, while the Rust printed the full happy-path
output and exited 0.

```
input "0\n16\n"   C: exit signal 11, stdout 0 bytes   Rust: exit 0, stdout 126 bytes
```

**Cause.** The original `store_one` modelled every out-of-bounds write below a
fixed threshold of 5010 as invisible. In the real compiled C, `buffer[16..=19]`
and `buffer[26..=27]` overlap the *live control data* of `bad()`'s frame — the
saved frame pointer and the return address. Storing `1` there makes `bad()`
return to address `1` and the process dies immediately. Because stdout is a
block-buffered pipe that is never flushed, everything printed so far is lost, so
both stdout and stderr come out empty.

Measured 100/100 reproducible for each of those six indices, and 0/100 for every
other index in 10..300, so this is a stable property of the layout and not noise.

**Fix.** `index_hits_live_control_slot()` in `src/main.rs` reproduces the fatal
window explicitly, calling `segfault()` (a volatile null write) so the process
dies from the same signal with the same empty streams.

---

## Mismatch 2 — the "far out of bounds" threshold was a wrong constant

**Symptom.** With the fix above, `cargo test --release` still failed
intermittently on index 4250: the C faulted, the Rust exited 0 (or vice versa),
roughly one run in three.

**Cause.** The Rust used a hard-coded `FAR_OOB_FATAL_INDEX`. There is no correct
constant, because how far `bad()`'s frame sits below the top of the stack
depends on:

* the combined size of `argv` and the environment, which the kernel copies onto
  the stack above `main` — a 60 KB environment variable moves the fault
  threshold by ~16000 index slots; and
* a per-exec random offset the kernel adds to the stack top.

Measured with a normal shell environment: index ≤ 2200 never faulted, ≥ 4500
always faulted, and everything in between was a coin flip. With `env -i` the
same band sat around index 400..2400 instead.

**Fix.** `store_one` now applies the rule the hardware actually applies. It
reads the `[stack]` mapping's end address from `/proc/self/maps` at runtime and
faults exactly when the target address `&buffer[0] + 4*idx` lands at or above
it. Because the Rust reads *its own* mapping, it automatically tracks the
environment size and its own randomisation, instead of guessing.

Verification that this tracks the C (crash counts out of 25 runs per value):

| index | small env (`env -i`) C / Rust | normal env C / Rust | 64 KB pad C / Rust |
|---|---|---|---|
| 1000 | 15 / 9 | 0 / 0 | 0 / 0 |
| 2000 | 23 / 17 | 0 / 0 | 0 / 0 |
| 17000 | 25 / 25 | 25 / 25 | 0 / 0 |
| 100000 | 25 / 25 | 25 / 25 | 25 / 25 |

---

## Mismatch 3 — `INT_MAX` as a `bad()` index: SIGSEGV vs SIGBUS

**Symptom.** Input `2147483647\n2147483647\n` gave C exit 135 (SIGBUS) and Rust
exit 139 (SIGSEGV).

**Cause.** Not a translation bug — the **C disagrees with itself**. For indices
above roughly 33,000,000 the target address is gigabytes past the stack and,
depending on ASLR, lands either on nothing (SIGSEGV) or inside a file-backed
mapping past its end (SIGBUS):

```
index 2147483647 over 40 runs of the C: 14x SIGBUS, 26x SIGSEGV
index 1073741824 over 40 runs of the C:  8x SIGBUS, 32x SIGSEGV
index   10000000 over 40 runs of the C:  0x SIGBUS, 40x SIGSEGV
```

**Resolution.** The Rust always raises SIGSEGV, C's majority outcome. The test
for these inputs asserts the property the ground truth actually holds to — both
programs die from a fatal signal with empty stdout and empty stderr — rather
than pinning a signal number the C itself does not pin.

---

## Irreducible non-determinism (documented, not "fixed")

Two input regions cannot be matched run-for-run by *any* implementation, because
the C's own output is not a function of its input:

1. **The stack-top band.** Indices within ~8 KB of the top of the stack mapping
   flip between "exit 0 with ten values printed" and "fatal signal, no output"
   from one exec to the next, driven by the kernel's random stack offset. Under
   the test harness's padded environment this band is roughly index
   18500..21000.
2. **Extreme indices** (above ~33,000,000): SIGSEGV vs SIGBUS, as above.

The test suite handles these honestly rather than by exclusion:

* `aslr_band_outcomes_are_from_the_same_two_shapes` asserts that every run of
  *either* program yields one of exactly two shapes — clean exit 0 with the
  byte-exact expected output, or death by signal with both streams empty.
* `extreme_indices_always_die_silently` asserts both die from a fatal signal
  with no output.

To keep every *other* out-of-bounds assertion genuinely deterministic, the
harness gives both programs the same 64 KB `DIFFTEST_STACK_PAD` environment
variable. That pushes the stack top far above `bad()`'s frame identically for
both, so indices up to 17000 are reliably invisible and indices from 22000 up
reliably fault. Without this, "which indices are safe" depends on whatever
environment the tests were launched from.

---

## Behaviours deliberately preserved (verified, no mismatch)

* **Read ordering.** First input line → `goodB2G`, second → `bad()`. Extra lines
  are ignored.
* **`fgets` 13-byte window.** `12345678901234567890\n` gives `goodB2G` the index
  `(int)1234567890123` and `bad()` the index `4567890`; the latter reliably
  faults. `fgets` does not skip leading whitespace or read across the newline.
* **`fgets` NULL path.** Empty stdin and closed stdin both print
  `fgets() failed.`, leaving `data == -1`, which then takes
  `ERROR: Array index is out-of-bounds` in `goodB2G` (its guard is a range
  check) but `ERROR: Array index is negative.` in `bad()`. Note the first has
  **no trailing period** and the second does — preserved exactly.
* **`atoi` semantics** (`(int)strtol(s, NULL, 10)`): leading whitespace skipped,
  optional sign, decimal only (`0x10` → 0), trailing junk ignored (`7abc` → 7),
  no digits → 0, saturation at `LONG_MIN`/`LONG_MAX` then truncation to `int`
  (`4294967296` → 0, `4294967297` → 1, `2147483648` → `INT_MIN`).
* **`-0`** parses to 0 and so takes the *accept* branch, not the negative one.
* **Embedded NUL** inside the 13-byte window terminates the string for `atoi`.
* **Buffered stdout.** Output is block-buffered and flushed only at normal exit,
  which is why a fatal signal discards all of it.
* **`argc`/`argv` unused**, so command line arguments change nothing.
* **Exit code 0** on every non-crashing path.
