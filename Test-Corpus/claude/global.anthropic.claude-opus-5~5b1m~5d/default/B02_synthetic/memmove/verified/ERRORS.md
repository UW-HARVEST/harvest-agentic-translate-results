# Differential verification log

The C in `c_src/` is the ground truth. Both programs are built and run as
subprocesses and compared on stdout, stderr and exit status:

```sh
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
#   -> c_src/build/driver

# Rust
cd translation && cargo build --release
#   -> translation/target/release/driver

# differential suite (builds the C itself if needed)
cd translation && cargo test
```

`translation/tests/differential.rs` holds the suite: 30 tests covering over
1500 input vectors, none `#[ignore]`d, skipped or disabled. It passes in both
the debug and the release profile. Beyond the suite, the two binaries were also
compared on ~12500 randomly generated inputs with no difference.

---

## Mismatches found and fixed

### 1. `compact_runs()` overflows `uint8_t buffer[256]`, and the overflow is observable

**Severity: this was the only behavioural difference, and it affects a large,
easily-reachable slice of the input space.**

`main.c` reserves `uint8_t buffer[256]`, but `compact_runs()` rewrites each run
as a 2-byte `value,count` pair. With `threshold == 1` (i.e. `flags & 0x02` and
`param1 == 1`) *every* run compacts, so a buffer of `n` runs grows to `2 * n`
bytes — up to 512 for a 256-byte input. The C therefore writes up to 256 bytes
past the end of its own array, into the rest of `main`'s stack frame.

The original Rust used a 4096-byte backing buffer and simply printed the clean
result, so it disagreed with the C for every input whose compacted length passed
280 bytes.

Reproducer (`L` distinct bytes, `flags = 2`, `param1 = 1`):

| input | C | previous Rust |
|---|---|---|
| `2 1 0 141 0 1 2 … 140` | `… 279 280 26 1` (exit 0) | `… 279 280 1 1` (exit 0) |
| `2 1 0 157 0 1 2 … 156` | *no output*, killed by SIGSEGV | full 314-byte line, exit 0 |
| `2 1 0 256 0 1 2 … 255` | *no output*, killed by SIGSEGV | full 512-byte line, exit 0 |

**Cause.** `objdump -d` on the CMake build (gcc, no optimisation) shows
`buffer` at `rbp-0x130` with `main`'s other locals at *higher* addresses, so the
overflow lands on them in a completely deterministic way:

| offset from `buffer` | slot | effect of the overflow |
|---|---|---|
| `256 .. 264` | `rbp-0x30` `size_t length` | dead after the call — invisible |
| `268 .. 272` | `rbp-0x24` `int param2` | dead after the call — invisible |
| `272 .. 276` | `rbp-0x20` `int param1` | dead after the call — invisible |
| `276 .. 280` | `rbp-0x1c` `uint32_t flags` | dead after the call — invisible |
| `280 .. 288` | `rbp-0x18` `size_t new_length` | **overwritten by `main`** right after `process_buffer()` returns, so these bytes print as the little-endian image of `new_length` |
| `288 .. 296` | `rbp-0x10` `size_t i` (print loop) | **live while printing**: byte `288 + k` prints as byte `k` of the loop counter, whose value at that moment is `288 + k` |
| `296 .. 304` | `rbp-0x08` `size_t i` (read loop) | dead — keeps the compacted data |
| `304 .. 312` | saved `rbp` | harmless: `main` does `leave; ret` immediately |
| `312 .. 320` | return address | **clobbered ⇒ `main` faults on `ret`** |

Two further details of the crash case:

* the crash happens *after* `printf`, but stdout is a fully buffered pipe and
  the line is at most ~2 KiB (`new_len <= 512`), well under `BUFSIZ`, so nothing
  has been flushed: stdout and stderr are both empty and the wait status is
  "killed by SIGSEGV".
* the trigger is the *highest index ever written*, not the final length. A run
  of singles followed by one long run grows the length past 312 and then shrinks
  it again; the return address stays clobbered. `2 1 0 256 <100 distinct><156
  zeros>` ends with `new_len == 202` and still dies by SIGSEGV.

**Fix.**

* `src/lib.rs` — all stores now go through a `Buffer` wrapper that records the
  highest index written (`memmove` ⇒ `memmove_within` / `memmove_from`,
  `buf[i] = v` ⇒ `set`). `process_buffer_tracked()` exposes it; the plain
  `process_buffer()` signature is unchanged.
* `src/main.rs` — models the frame layout above: a write reaching index 312 or
  beyond resets `SIGSEGV` to `SIG_DFL` and `raise()`s it *without* flushing
  stdout; otherwise bytes `280..288` print as `new_length` and bytes `288..296`
  print as the print-loop counter.

Verified deterministic: 75 runs over 25 distinct crashing inputs produced
"killed by SIGSEGV / empty stdout / empty stderr" every time.

Covered by `overflow_below_the_aliased_locals`, `overflow_into_aliased_locals`,
`overflow_clobbers_the_return_address`,
`transient_overflow_still_clobbers_the_return_address`,
`overflow_combined_with_other_flags` and `maximum_length_sweep`.

---

## C behaviour deliberately reproduced (checked, no change needed)

These all looked like candidate mismatches on review and were confirmed
identical by test:

* **`scanf` crosses newlines.** All five conversions are `scanf`, not `fgets`,
  so the fields may be separated by any run of `isspace()` characters — spaces,
  tabs, `\n`, `\r`, `\v`, `\f` — in any layout. `src/scan.rs` skips exactly the
  C locale whitespace set. (`whitespace_and_layout`)
* **`%u` accepts a sign and wraps.** glibc converts `%u`/`%zu` with
  `strtoul`, so `-1` for `flags` yields `4294967295`, and `-1` for `length`
  yields `18446744073709551615`, which then trips the `length > 256` check with
  that exact number in the message. (`length_exceeds_maximum`,
  `high_flag_bits_are_ignored`)
* **Out-of-range integers saturate, then truncate.** `%d` saturates at
  `LONG_MAX`/`LONG_MIN` before being truncated to `int`, so `2147483648`
  becomes `-1` rather than a matching failure. (`numeric_conversion_edges`)
* **A partly-consumed token breaks the *next* field.** `%u` on `0x10` consumes
  only `0`; the leftover `x10` makes the following conversion fail. So
  `0x10 0 0 0` reports `Error reading param1`, not a flags error.
  (`param1_read_failure`, `malformed_byte_tokens`)
* **Order of validation.** `length > 256` is rejected *before* any byte is
  read, and the per-byte error names the failing index (`Error reading byte
  %zu`). (`length_exceeds_maximum`, `byte_read_failure`)
* **Output format.** `printf("%zu", n)` then `printf(" %u", b)` per byte then a
  single `putchar('\n')` — no space before the length, exactly one space before
  each byte, one trailing newline, and just `0\n` when `length == 0`.
  (`zero_length`)
* **All error paths return 1** and write to stderr; stdout stays empty.
* **`param1 % (int)length` twice.** `process_buffer()` reduces the offset and
  `rotate_buffer()` reduces it again; C's `%` truncates toward zero, so a
  negative `param1` gives a negative intermediate that is then normalised with
  `+= len`. Reproduced with `wrapping_rem`. (`rotate_branches`)
* **`run_len` capped at 255.** 256 identical bytes emit `value, 255` and the one
  leftover byte is reprocessed. (`compact_runs_branches`)
* **High flag bits are inert.** Only bits 0..4 are tested, so `0xFFFFFFFF`
  behaves like `0x1F`. (`high_flag_bits_are_ignored`)
* **Guards use the *current* length.** `new_len >= 2`, `new_len >= 4` and
  `seg_size <= new_len` are re-evaluated after compaction/dedup shrank the
  buffer. (`length_guards_after_the_length_changed`)
* **`param1` is shared.** The same value is the rotation offset, the run
  threshold *and* the segment size, so `param1 == 1` means "rotate by 1",
  "threshold 1" (the growth case) and "segment size 1" (a no-op).

## Statically unreachable C branches

Kept in the Rust translation for fidelity, but no input can reach them:

* `process_buffer`: `buffer == NULL` — `main` always passes its array.
* `rotate_buffer`: `len <= 1`, and `offset == 0` after normalisation — the
  caller already reduced the offset mod `length` and checked it is non-zero.
* `rotate_buffer` small-offset loop: `chunk == offset` because
  `offset < len/2 <= 128 < 256`, so the loop always runs exactly once.
* `interleave_halves`: `len < 2` (guarded by the caller) and the `half > 256`
  in-place branch — `new_len <= 512` bounds `half` at 256.
* `reverse_segments`: `len < seg_size` — guarded by `seg_size <= new_len`.

## Assumption

The stack-frame model reproduces the executable produced by the documented
build, `cd c_src/build && cmake .. && cmake --build .` (gcc, no optimisation
flags, no stack protector — the crash is a plain SIGSEGV, with no
"stack smashing detected" message). Optimising the C would keep `new_length`
and `i` in registers and relocate the array, changing what the out-of-bounds
writes alias; the behaviour of that program is a different question. All
in-bounds behaviour (every input whose compacted length stays within 256 bytes)
is independent of this assumption.
