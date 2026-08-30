# Differential verification notes

Ground truth: `c_src/` (never modified). Subject: this crate's `driver` binary.
Both programs are compared by running them, not by linking: see
`tests/differential.rs` and `tests/common/mod.rs`.

## How to reproduce

```sh
cd c_src && cmake -S . -B build && cmake --build build   # produces c_src/build/driver
cd ../translation && cargo build --release               # produces target/release/driver
cargo test                                               # 23 differential tests
```

The test harness builds the C program itself if `c_src/build/driver` is absent,
configuring out-of-source into `translation/target/c_build` so that `c_src/`
stays untouched.

Run commands recorded in Phase A:

| program | command |
| --- | --- |
| C | `c_src/build/driver` (reads stdin, writes stdout/stderr) |
| Rust | `translation/target/release/driver` (same) |

## Mismatches found

### 1. `compact_runs` with threshold 1 overflows the C program's stack buffer

**Status: unresolved by design — the C behaviour here is undefined, not a
behaviour that can be replicated.**

`main.c` declares `uint8_t buffer[256]` and `process_buffer` is free to grow the
logical length. When flag `0x02` is set and `param1 == 1`, `compact_runs`
rewrites *every* run — including runs of length 1 — as the two-byte pair
`value, count`. Each single-element run therefore adds one byte to `len`, and a
buffer of `N` distinct bytes ends up `2 * N` long. For `N > 128` the C program
writes past `buffer[255]`.

Observed on this machine (`gcc 11.5.0`, x86-64):

| input | C stdout | C status | Rust stdout | Rust status |
| --- | --- | --- | --- | --- |
| `2 1 0 128` + 128 distinct bytes | `256 0 1 1 ...` | 0 | identical | 0 |
| `2 1 0 140` + 140 distinct bytes | `280 0 1 1 ...` | 0 | identical | 0 |
| `2 1 0 160` + 160 distinct bytes | *(empty)* | killed by SIGSEGV | `320 0 1 1 ...` | 0 |
| `2 1 0 256` + 256 distinct bytes | *(empty)* | killed by SIGSEGV | `512 0 1 1 ...` | 0 |

stdout is empty in the crashing cases because stdout is a fully buffered pipe
and the process dies before the flush at `return 0`.

**Cause.** Out-of-bounds stack writes clobber the saved return address of
`main`. Whether that is fatal depends purely on the stack frame layout the
compiler chose, so the boundary is not a property of the source:

| C build | last non-crashing result length | first crashing result length |
| --- | --- | --- |
| no `CMAKE_BUILD_TYPE` (`-O0`) | 312 | 314 |
| `CMAKE_BUILD_TYPE=Debug` (`-O0 -g`) | 312 | 314 |
| `CMAKE_BUILD_TYPE=Release` (`-O3`) | 280 | 282 |

Because the same C source produces two different boundaries from two different
compiler invocations, there is no boundary to translate. The Rust program
instead over-allocates the buffer (`BUFFER_CAPACITY = 1024`, zero filled) and
keeps running, which reproduces the C output exactly for every input where the C
program stays inside its own array — and, incidentally, for the padding region
up to length 312 on the `-O0` build.

**Test policy.** `tests/common/mod.rs::c_overflows` identifies this class
(`flags & 0x02 != 0 && threshold == 1 && length > 128`) and the differential
tests skip it. Nothing is `#[ignore]`d: the excluded inputs are excluded from
the *generated input set*, not from assertion. The growth path itself is still
covered up to and including `length == 128`, where the final length is exactly
256 and the C program is still in bounds
(`compact_runs_growth_path`).

Threshold 1 is the only way to reach this: `threshold` is `param1` when
`0 < param1 <= 255` and otherwise 3, so `param1 == 1` is required, and a run of
length 2 or more never grows.

## Behaviours checked and found already faithful

No other mismatch was found. The following are the traps a translation of this
program can plausibly get wrong; each was confirmed identical by running both
binaries, and each has a dedicated test.

- **`rotate_buffer` rotates in two different directions.** The
  `offset < len / 2` branch moves the prefix aside and shifts the remainder
  down, which is a *left* rotation; the `else` branch is a *right* rotation.
  `1 2 0 10 0 1 ... 9` gives `2 3 4 5 6 7 8 9 0 1` while `1 7 0 10 ...` gives
  `3 4 5 6 7 8 9 0 1 2`. Covered by `rotate_all_offsets` / `rotate_extremes`.
- **`rotate_buffer`'s chunk loop always runs once.** `chunk` is
  `min(offset, 256)` and `offset < len <= 256`, so `chunk == offset` and
  `i += chunk` exits the loop after the first iteration.
- **`compact_runs` caps a run at 255 but still advances `read` by the capped
  value.** 256 identical bytes with any threshold `<= 255` yields
  `3 7 255 7`, not `2 7 255`: the 256th byte survives as a leftover run.
  Covered by `compact_runs_caps_at_255`.
- **`interleave_halves` reads bytes it has already overwritten.** The loop
  writes `buf[i*2+1] = buf[half+i]` in place, so once `i*2+1 >= half` it is
  reading its own output. The odd-length fixup `buf[len-1] = buf[half]` reads an
  already-clobbered slot too. `8 0 0 7 1 2 3 4 5 6 7` yields
  `1 4 2 5 3 6 5` — note the duplicated `5` and the lost `7`. Covered by
  `interleave_length_guard_and_parities`.
- **`interleave_halves`' `half > 256` branch is unreachable.** It needs
  `new_len >= 514`, beyond anything `process_buffer` can produce from a
  256-byte input. Kept in the translation for fidelity; no test can reach it.
- **Order of the flag stages, and the length guards between them.** `0x08`
  requires `new_len >= 2` and `0x10` requires `new_len >= 4`, both evaluated
  against the length *after* compaction and de-duplication, and `0x10`
  additionally needs `seg_size <= new_len`. Covered by
  `every_flag_combination_on_representative_buffers`.
- **`param1` is overloaded three ways** — rotation offset, run threshold and
  segment size — so a single value drives all three stages when several flags
  are set. Covered by the same test.
- **`scanf` ignores line structure.** `3\n1\n0\n4\n1\n2\n3\n4\n` and
  `3 1 0 4 1 2 3 4` are equivalent, and a missing trailing newline changes
  nothing. Covered by `whitespace_and_stream_shape`.
- **glibc `%u` / `%zu` accept a minus sign and wrap.** `0 0 0 -1` is read as
  `length == 18446744073709551615` and reported as
  `Error: length 18446744073709551615 exceeds maximum 256`, exit 1 — not as a
  read failure. Covered by `length_exceeds_maximum`.
- **Out-of-range conversions clamp before they narrow.** `%u` goes through
  `strtoul` and saturates at `ULONG_MAX` before truncation to `unsigned int`,
  so `99999999999999999999999999` for `flags` becomes `0xFFFFFFFF`; `%d` goes
  through `strtol`, saturates at `LONG_MAX`/`LONG_MIN`, and then narrows to
  `int`, so an absurdly large `param1` becomes `-1`. Covered by
  `integer_conversion_edges`.
- **Partial numeric parses.** `0x10` is read as `0` (base 10 stops at `x`) and
  the leftover `x10` fails the *next* conversion, so the reported error is
  `Error reading param1`, not `Error reading flags`. Likewise `1.5` yields `1`
  and then fails on `.`. Covered by `error_reading_param1` /
  `error_reading_param2`.
- **Error precedence.** The length bound is checked before any buffer byte is
  read, so `0 0 0 300 1 2 3` reports the length error and never reports a byte
  error. Each of the six `fprintf(stderr, ...)` sites, its exact wording and
  its exit code 1 has a test.
- **`length == 0` short-circuits `process_buffer` before the flag dispatch**,
  printing `0\n` for every flag/parameter combination. Covered by
  `zero_length_returns_zero_for_every_flag_combination`.
- **Output formatting.** `printf("%zu", n)` then `printf(" %u", byte)` per
  element then a single `"\n"`, which means length `0` prints `0\n` with no
  trailing space. Asserted byte for byte everywhere.

## Coverage evidence

Beyond the 23 committed tests, the two programs were compared over
approximately 600,000 additional inputs while investigating:

- 432,128 inputs: flags `0..32` × 32 `param1` values × 2 `param2` values × 96
  buffer patterns (all binary patterns up to length 6, plus 3-symbol and
  longer patterns) — 0 mismatches.
- 140,768 inputs: 8 structured pattern families (all-identical, all-distinct,
  alternating, block runs, 255-long runs, single-run-plus-tail) at lengths
  1–4 and 127–256 × 23 `param1` values × flags `0..32` — 0 mismatches.
- ~30,000 random inputs plus 79 hand-written malformed-input cases — 0
  mismatches outside the overflow class above.

The harness itself was mutation-tested: reverting the `interleave_halves`
clobbering read, changing the `compact_runs` cap from 255 to 254, and changing
one `exit(1)` to `exit(0)` each caused a test failure, confirming the suite
checks stdout, stderr and exit status rather than passing vacuously.
