# UB.md — the one input region where byte-identical behaviour is unachievable

This file records the single place where "the Rust must match the C exactly" runs
out of meaning, why, and what is asserted instead. It exists because the finding
came out of Phase B and would otherwise be invisible.

## The defect

`driver.c:42` is a CWE-121 stack buffer overflow. `bad()` checks only the *lower*
bound:

```c
void bad(int data) {
    int i;
    int buffer[10] = { 0 };
    if (data >= 0) {
        buffer[data] = 1;          /* <-- no upper bound check */
        for (i = 0; i < 10; i++) printIntLine(buffer[i]);
    } else {
        printLine("ERROR: Array index is negative.");
    }
}
```

`goodB2G()` is the same code with the check repaired (`data >= 0 && data < 10`),
so it has no such region.

## Where each index actually writes

From `objdump -d c_src/build/libdriver.so` (`gcc`, default CMake flags = `-O0`),
`bad` builds a frame with `sub $0x40,%rsp` and puts `buffer` at `-0x30(%rbp)`,
`i` at `-0x4(%rbp)` and `data` at `-0x34(%rbp)`. So `buffer[data]` targets
`rbp - 0x30 + 4*data`:

| `data`    | address       | what lives there              | consequence |
|-----------|---------------|-------------------------------|-------------|
| `0..=9`   | `-0x30..-0x08`| `buffer` itself               | in bounds |
| `10`      | `-0x08(%rbp)` | frame padding                 | benign |
| `11`      | `-0x04(%rbp)` | **the loop counter `i`**      | benign — the `for` statement re-initialises `i` to 0 immediately after |
| `12..=13` | `+0x00(%rbp)` | **saved `rbp`**               | caller's frame pointer destroyed; `leave` loads garbage; the *caller* dies later |
| `14..=15` | `+0x08(%rbp)` | **the return address**        | `ret` jumps to `0x1`; dies immediately |
| `>=16`    | `+0x10(%rbp)` and up | the caller's frame, its caller's, ... | corrupts whoever is up the stack |

Note the consequence for `data` in `0..=11`: **nothing observable happens**.
`buffer[10]` and `buffer[11]` miss `buffer`, so all ten printed values are `0`,
and `i` is overwritten only to be immediately re-assigned. Those two indices are
fully comparable and are covered by `CONFIGS.md` rows 18–19.

## Why `data >= 12` cannot be matched

From `data >= 12` the write lands **outside `bad`'s own frame**, in state owned by
the caller. Whether that is fatal is therefore a property of the *entire call
chain's* stack layout — the caller, its caller, and the compiler that built them —
not of the program under test. Measured over `data` in `10..=700` (unbuffered
stdout, one process per index):

```
bad(d)         C dies at: 12 13 14 15 202 203 204 205 208 209 222 223 240 241
                          246 247 342 343 344 345 550 551 558 559 566 567 574
                          575 642 643 654 655 658 659
bad(d)      Rust dies at: 134 135 322 323 324 325 328 329 342 343 360 361 366
                          367 462 463 464 465 670 671 678 679 686 687 694 695

driver(7,d)    C dies at: 12 13 14 15 20 21 22 23 210 211 212 213 216 217 230
                          231 248 249 254 255 350 351 352 353 558 559 566 567
                          574 575 582 583 650 651 662 663 666 667
driver(7,d) Rust dies at: 132 133 134 135 140 141 142 143 330 331 332 333 336
                          337 350 351 368 369 374 375 470 471 472 473 678 679
                          686 687 694 695
```

Two things follow:

1. The patterns differ **in both directions**. There are indices where the C dies
   and the Rust survives (`bad(12)`), and indices where the Rust dies and the C
   survives (`bad(134)`). Neither is "more correct".
2. The pattern is not even stable across call depth: the same index behaves
   differently under `bad(d)` and under `driver(7, d)`, because `bad`'s frame sits
   one level deeper in the second case. So there is no fixed set of indices a
   translation could be tuned to reproduce.

Reproducing this would mean reproducing gcc's `-O0` stack frame layout, and the
layout of every caller, inside Rust. That is not a translation of the program; it
is a translation of one particular compilation of it, and it would break the
moment the C is rebuilt at `-O2` or with `-fstack-protector`.

## What the Rust does instead, and what is asserted

`translation/src/lib.rs` gives `bad` a frame with explicit trailing slack:

```rust
#[repr(C)]
struct Frame { buffer: [c_int; 10], _slack: [c_int; 118] }
```

so indices `10..=127` are absorbed harmlessly and the printed output is
well-defined. Beyond that the write leaves the object and the process eventually
dies, as it does in C.

`tests/phase_b_ub.rs` therefore does **not** compare exit status for
`data >= 12`. It asserts the strongest property that is genuinely well defined,
and that a faithful translation must satisfy:

* `ub_01` — for `data` in `12..=15` (saved `rbp` and return address), both builds
  print the identical ten zeros before the damage takes effect.
* `ub_02` — for every `data` in `12..=400`, **what the library printed is
  byte-identical** between the two builds, and is always ten zeros.
* `ub_03` — the same region through `driver`, comparing the common output prefix,
  and requiring that whichever side survived produced the complete 34-line
  pipeline output.
* `ub_04` — for far-out-of-range indices (`100_000`, `1_000_000`, `100_000_000`,
  `INT_MAX`) **both** builds must die, and neither may have printed anything.

To make `ub_01`–`ub_03` meaningful, `examples/probe.rs` sets `stdout` unbuffered
(`setvbuf(stdout, NULL, _IONBF, 0)`) before the call. Without that, a call that
prints ten lines and *then* dies on `ret` loses all ten in the block-buffered
pipe, and the comparison would report a spurious difference that is an artifact
of stdio buffering rather than of the translation. This was observed directly:
`bad(14)` yields 0 bytes block-buffered but 20 bytes unbuffered, in both builds.

## Scope of the exclusion

Only `bad()` (and `driver()`'s `badData` argument) with `data >= 12`. Everything
else — all of `printLine`, all of `printIntLine`, all of `good`, `driver`'s
`goodData` argument over the full `int` range, and `bad`/`badData` over
`INT_MIN..=11` — is compared for byte-identical stdout **and** identical exit
status, with no exclusions. That is 62 tests across
`phase_b_configs`, `phase_c_errors`, `phase_d_symbols`, all passing in both the
`dev` and `release` profiles.
