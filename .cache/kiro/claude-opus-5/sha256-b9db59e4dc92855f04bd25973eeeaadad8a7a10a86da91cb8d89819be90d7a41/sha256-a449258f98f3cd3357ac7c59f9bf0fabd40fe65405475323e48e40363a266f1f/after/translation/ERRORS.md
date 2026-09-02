# Differential verification of the C → Rust translation

Ground truth is `c_src/` (never modified). The Rust crate in this directory is
compared against it by running both executables on identical stdin and diffing
stdout, stderr and exit status.

## How it was verified

```
# C reference
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
#   -> c_src/build/driver

# Rust
cd translation && cargo build --release
#   -> translation/target/release/driver

# Differential suite (spawns both binaries as subprocesses)
cd translation && cargo test
```

`tests/differential.rs` holds 23 tests that together compare **13,834 input
pairs**, asserting all three of stdout (byte for byte), stderr (byte for byte)
and exit status on every one. An additional standalone sweep compared **51,770**
further pairs (all 32 flag combinations × lengths 0–256 × a wide `param1`/`param2`
grid, plus 20,000 randomized inputs) with zero mismatches.

Result: **no mismatch exists for any input on which the C program's behaviour is
defined.** The single divergence found is described in Finding 1 and is caused by
undefined behaviour in the C.

## Finding 1 — the one real divergence: `compact_runs` writes past `buffer[256]`

**Status: present, deliberately not emulated.**

`main` declares `uint8_t buffer[256]`, and `process_buffer` is allowed to *grow*
the logical length. In `compact_runs`, a run whose length is `>= threshold` is
rewritten as a two-byte `value, count` pair, so when `threshold == 1` every
*singleton* run turns 1 byte into 2 and the length grows. `threshold` is 1
exactly when `param1 == 1`:

```c
uint8_t threshold = (param1 > 0 && param1 <= 255) ? (uint8_t)param1 : 3;
```

With `n` distinct input bytes the length becomes `2n`, and the C writes all `2n`
bytes through the 256-byte array. For `n > 128` that is a stack buffer overflow.

Reproducer:

```
printf '2 1 0 200 %s' "$(seq 0 199 | tr '\n' ' ')" | c_src/build/driver
```

### Why the divergence only starts at 280 and not at 256

`main`'s frame at `-O0` places `buffer` at `rbp-0x130`, so the bytes just past
the array land on locals that `main` has already consumed:

| `buffer` index | what actually lives there |
|---|---|
| 256–263 | `length` |
| 268–271 | `param2` |
| 272–275 | `param1` |
| 276–279 | `flags` |
| 280–287 | `new_length` — **re-assigned after `process_buffer` returns** |
| 296–303 | the print loop's `i` — **mutated while printing** |
| 312–319 | `main`'s return address |

Writes into 256–279 hit dead variables and read back unchanged, so both programs
still agree there. From 280 on, the C's own output overwrites what
`compact_runs` wrote. Measured boundaries (input `2 1 0 n <n distinct bytes>`,
grown length `2n`):

| grown length | C behaviour | agrees with Rust? |
|---|---|---|
| ≤ 280 (`n ≤ 140`) | correct output | **yes** |
| 282 – 312 (`n = 141..156`) | exit 0, but byte 280 shows the low byte of `new_length` and bytes 296+ show the print counter | no |
| ≥ 314 (`n ≥ 157`) | return address clobbered → SIGSEGV, exit 139, empty stdout | no |

The first mismatching byte is exactly as predicted by the table: at `n = 141`
the grown length is 282 and the C prints `26` at index 280, which is
`282 & 0xFF` — the low byte of `new_length` written back over the buffer.

### Why it is not emulated

Reproducing this would mean hard-coding one compiler's stack frame layout for
`main`, which is not a property of the program being translated:

- With `cmake -DCMAKE_BUILD_TYPE=Release` (`-O2`) the same input at `n = 141`
  **segfaults**, where the `-O0` build exits 0 with slightly wrong bytes. The
  layout, and therefore the "correct" answer, changes with the build flags.
- Producing the exit-139 case from safe Rust would mean detecting the condition
  and raising SIGSEGV artificially — emulating a symptom, not translating logic.

The Rust program instead backs the buffer with `2 * 256` bytes
(`BUFFER_CAPACITY = 512`), which bounds the growth, keeps every write in real
storage, and matches the C exactly across the whole region where the C's own
behaviour is well defined. Test `compact_growth_overflow_boundary_matches` pins
that region: it asserts agreement for every length 1..=140 inclusive.
`c_clobbers_live_locals()` in the test file identifies the undefined family so
the generated sweeps stay inside the defined domain; nothing is `#[ignore]`d.

## Findings 2–11 — C behaviour that a natural translation would get wrong

These produced no mismatch, because the translation already reproduces them.
They are recorded because each is a place where idiomatic Rust would diverge, so
they are what the tests are guarding.

**2. `scanf("%u")` and `scanf("%zu")` accept a minus sign.** glibc hands the
digits to `strtoul`, which wraps. `0 0 0 -1` therefore does not report a bad
length — it reports a huge one, and the message must print the wrapped value:

```
Error: length 18446744073709551615 exceeds maximum 256
```

A translation that rejected the `-` or clamped to 0 would print a different
message, or none. Rust models this as `magnitude.wrapping_neg()`.

**3. `strtoul`/`strtol` saturate, then the assignment truncates.** A 400-digit
number saturates to `ULONG_MAX`/`LONG_MAX` *before* being narrowed, so
`%u` yields `0xFFFFFFFF` and `%d` yields `-1` — not 0, and not the low digits.
Distinct from plain modular truncation: `4294967296` (no saturation) yields 0.

**4. `scanf` skips newlines.** Every numeric conversion consumes leading
whitespace, so the four scalars and all bytes may be laid out over any number of
lines, with tabs, CRs, vertical tabs or form feeds. A line-oriented
(`fgets`-style) reader would reject valid input.

**5. A NUL byte is a matching failure, not a terminator.** The program reads a
stream, so `3 2 1 4 10 \0 20 ...` fails at a byte read rather than truncating.

**6. `rotate_buffer`'s two branches rotate in *opposite* directions.** Despite
the comment "positive = right", the small-offset branch (`offset < len/2`)
rotates **left** by `offset`, while the large-offset branch rotates **right**.
On `[1..10]`, `param1 = 2` gives `3 4 5 6 7 8 9 10 1 2` (left by 2) but
`param1 = 6` gives `5 6 7 8 9 10 1 2 3 4` (right by 6). Preserved verbatim.

**7. `interleave_halves` destroys the last element of an odd-length buffer.**
The tail `buf[len - 1] = buf[half];` reads `buf[half]`, which the interleave loop
has already overwritten. `[1,2,3,4,5]` becomes `1 3 2 4 2` — the `5` is gone and
a duplicate `2` appears. Preserved verbatim.

**8. The run-length count is capped at 255 and the cap is reachable.** 256
identical bytes with `threshold = 1` produce `4 7 255 7 1`: the run is recorded
as 255, and the one byte the cap left behind becomes a second run.

**9. The `threshold` default applies to `param1` values outside `1..=255`.**
`param1` of `0`, `-5`, `256` and `1000` all fall back to 3 and give identical
output. Note this also means `param1 == 256` behaves as 3 rather than as 0 — no
truncation to `uint8_t` happens on the guard, only on the value.

**10. `remove_duplicates`' two branches are observationally equivalent.** The
`preserve_order` (`param2 != 0`) path and the swap-to-front path both return the
distinct values in first-appearance order, so `param2` cannot affect the output.
Confirmed by inverting the condition in the Rust: all 51,770 comparisons still
matched. Both branches are still translated separately, since only the returned
prefix is equivalent — the bytes past it differ, and relying on that would be
fragile.

**11. Only the low 5 bits of `flags` are examined.** `0xFFFFFFFF`, `0x8000_0000`
and `0xDEADBEEF` are all accepted and behave as their low 5 bits.

## Unreachable code (verified, not tested)

Two paths cannot be reached from `main` and so have no test:

- `interleave_halves`' `else` branch needs `half > 256`, i.e. a length above
  512. The maximum reachable length is 512 (256 doubled by `compact_runs`),
  giving `half == 256`, so the `half <= 256` branch always wins.
- `rotate_buffer`'s chunking loop can only iterate once. Rotation runs before
  `compact_runs` on the original length (≤ 256), so after normalisation
  `offset <= 255`, making `chunk == offset` and the loop a single pass.

`process_buffer`'s `buffer == NULL` guard is likewise unreachable and is
represented in Rust by the `buffer.is_empty()` check.

## Confirmation that the tests actually bite

Seven faults were injected into the Rust source one at a time; the suite was run
after each and the source restored. Five were caught, and the two that were not
are provably unobservable — a useful result in itself.

| injected fault | caught |
|---|---|
| `rotate_buffer` branch boundary `<` → `<=` | yes (3 divergences) |
| run-length cap 255 → 256 | yes (2) |
| exit status 1 → 0 on the length-exceeds error, stdout/stderr untouched | yes (2) |
| trailing `\n` removed from stdout | yes (16) |
| one character of the `param1` error message | yes (2) |
| `interleave_halves` odd tail reads `temp[half-1]` instead of `buf[half]` | yes (4) |
| `preserve = param2 != 0` → `== 0` | no — provably unobservable, see Finding 10 |
| `if remainder > 1` → `> 0` in `reverse_segments` | no — `0..remainder/2` is empty when `remainder == 1`, so the guard is redundant |

The exit-status fault is the important one: stdout and stderr were left
identical and only the return code changed, and the suite still failed. That is
the case a stdout-only comparison would have missed.
