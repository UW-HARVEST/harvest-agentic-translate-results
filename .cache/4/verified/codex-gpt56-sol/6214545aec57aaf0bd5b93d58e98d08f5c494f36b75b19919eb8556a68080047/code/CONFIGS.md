# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table and the C source has no conditional
compilation branches. There is exactly one valid build-time combination:

| # | Cargo feature combination | C configuration | Status |
|---|---------------------------|-----------------|-----|
| B01 | empty set (`--no-default-features`) | default/unconditional | [x] |

## Runtime Matrix

The sole public entry point is:

```c
int process_decisions(char *decision_string, size_t length,
                      int operation, int param);
```

All rows vary byte representation as well as logical value: true is `y` or
`Y`; false is `n`, `N`, or any other byte. Operations 0 and 1 consume only the
first three bytes. Operation 2 consumes at most 32 bytes. Operation 3 parses
and overwrites every input byte with C `bool` values (`0` or `1`).

### Operation 0: Permissions

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|-----|
| C001 | `process_decisions` op 0 | triple `FFF`; length >= 3, arbitrary ignored tail | [x] |
| C002 | `process_decisions` op 0 | triple `FFT`; length >= 3, arbitrary ignored tail | [x] |
| C003 | `process_decisions` op 0 | triple `FTF`; length >= 3, arbitrary ignored tail | [x] |
| C004 | `process_decisions` op 0 | triple `FTT`; length >= 3, arbitrary ignored tail | [x] |
| C005 | `process_decisions` op 0 | triple `TFF`; length >= 3, arbitrary ignored tail | [x] |
| C006 | `process_decisions` op 0 | triple `TFT`; length >= 3, arbitrary ignored tail | [x] |
| C007 | `process_decisions` op 0 | triple `TTF`; length >= 3, arbitrary ignored tail | [x] |
| C008 | `process_decisions` op 0 | triple `TTT`; length >= 3, arbitrary ignored tail | [x] |

### Operation 1: Conditions

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|-----|
| C009 | `process_decisions` op 1 | AND (`param=0`), triple `FFF`, length >= 3 | [x] |
| C010 | `process_decisions` op 1 | AND (`param=0`), triple `FFT`, length >= 3 | [x] |
| C011 | `process_decisions` op 1 | AND (`param=0`), triple `FTF`, length >= 3 | [x] |
| C012 | `process_decisions` op 1 | AND (`param=0`), triple `FTT`, length >= 3 | [x] |
| C013 | `process_decisions` op 1 | AND (`param=0`), triple `TFF`, length >= 3 | [x] |
| C014 | `process_decisions` op 1 | AND (`param=0`), triple `TFT`, length >= 3 | [x] |
| C015 | `process_decisions` op 1 | AND (`param=0`), triple `TTF`, length >= 3 | [x] |
| C016 | `process_decisions` op 1 | AND (`param=0`), triple `TTT`, length >= 3 | [x] |
| C017 | `process_decisions` op 1 | OR (`param=1`), triple `FFF`, length >= 3 | [x] |
| C018 | `process_decisions` op 1 | OR (`param=1`), triple `FFT`, length >= 3 | [x] |
| C019 | `process_decisions` op 1 | OR (`param=1`), triple `FTF`, length >= 3 | [x] |
| C020 | `process_decisions` op 1 | OR (`param=1`), triple `FTT`, length >= 3 | [x] |
| C021 | `process_decisions` op 1 | OR (`param=1`), triple `TFF`, length >= 3 | [x] |
| C022 | `process_decisions` op 1 | OR (`param=1`), triple `TFT`, length >= 3 | [x] |
| C023 | `process_decisions` op 1 | OR (`param=1`), triple `TTF`, length >= 3 | [x] |
| C024 | `process_decisions` op 1 | OR (`param=1`), triple `TTT`, length >= 3 | [x] |
| C025 | `process_decisions` op 1 | XOR (`param=2`), triple `FFF`, length >= 3 | [x] |
| C026 | `process_decisions` op 1 | XOR (`param=2`), triple `FFT`, length >= 3 | [x] |
| C027 | `process_decisions` op 1 | XOR (`param=2`), triple `FTF`, length >= 3 | [x] |
| C028 | `process_decisions` op 1 | XOR (`param=2`), triple `FTT`, length >= 3 | [x] |
| C029 | `process_decisions` op 1 | XOR (`param=2`), triple `TFF`, length >= 3 | [x] |
| C030 | `process_decisions` op 1 | XOR (`param=2`), triple `TFT`, length >= 3 | [x] |
| C031 | `process_decisions` op 1 | XOR (`param=2`), triple `TTF`, length >= 3 | [x] |
| C032 | `process_decisions` op 1 | XOR (`param=2`), triple `TTT`, length >= 3 | [x] |
| C033 | `process_decisions` op 1 | NAND (`param=3`), triple `FFF`, length >= 3 | [x] |
| C034 | `process_decisions` op 1 | NAND (`param=3`), triple `FFT`, length >= 3 | [x] |
| C035 | `process_decisions` op 1 | NAND (`param=3`), triple `FTF`, length >= 3 | [x] |
| C036 | `process_decisions` op 1 | NAND (`param=3`), triple `FTT`, length >= 3 | [x] |
| C037 | `process_decisions` op 1 | NAND (`param=3`), triple `TFF`, length >= 3 | [x] |
| C038 | `process_decisions` op 1 | NAND (`param=3`), triple `TFT`, length >= 3 | [x] |
| C039 | `process_decisions` op 1 | NAND (`param=3`), triple `TTF`, length >= 3 | [x] |
| C040 | `process_decisions` op 1 | NAND (`param=3`), triple `TTT`, length >= 3 | [x] |

### Operation 2: Flags

Rows are the reachable ordered branches in `configure_flags`; randomized
cases vary count and the positions of true/false values while preserving the
listed branch preconditions.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|-----|
| C041 | `process_decisions` op 2 | consumed count 1..32, all false | [x] |
| C042 | `process_decisions` op 2 | consumed count 1..32, all true | [x] |
| C043 | `process_decisions` op 2 | count 2..32, exactly one true (all positions) | [x] |
| C044 | `process_decisions` op 2 | count 2..32, exactly one false (all positions) | [x] |
| C045 | `process_decisions` op 2 | count >= 4, alternating, starts true, not caught by prior count branches | [x] |
| C046 | `process_decisions` op 2 | count >= 4, alternating, starts false, not caught by prior count branches | [x] |
| C047 | `process_decisions` op 2 | non-alternating, maximum consecutive true run >= 3, not all/one-away | [x] |
| C048 | `process_decisions` op 2 | non-alternating, maximum consecutive true run <= 2, fallback true-count result | [x] |
| C049 | `process_decisions` op 2 | length exactly 32 (last valid flag-bit boundary), varied patterns | [x] |
| C050 | `process_decisions` op 2 | length > 32; first 32 bytes determine result and suffix is ignored | [x] |

### Operation 3: Sequence Validation

Only valid sequence outcomes are listed here. The long-sequence
`transitions < 3` return-40 branch is unreachable after enforcing the earlier
maximum-run-of-three rule: at most three runs can hold at most nine elements.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|-----|
| C051 | `process_decisions` op 3 | short length 1, starts true, zero transitions | [x] |
| C052 | `process_decisions` op 3 | short length 2, starts true/ends false, every adjacent value differs | [x] |
| C053 | `process_decisions` op 3 | short length 3, starts true/ends false, one transition | [x] |
| C054 | `process_decisions` op 3 | medium length 4..10, runs <= 3, `transitions < length/3` | [x] |
| C055 | `process_decisions` op 3 | medium length 4..10, runs <= 3, `transitions > length/2` | [x] |
| C056 | `process_decisions` op 3 | medium length 4..10, runs <= 3, transitions between those bounds | [x] |
| C057 | `process_decisions` op 3 | long length > 10 (including oversized 1025+), runs <= 3, `transitions > length-3` | [x] |
| C058 | `process_decisions` op 3 | long length > 10 (including oversized 1025+), runs <= 3, transitions at most `length-3` | [x] |
