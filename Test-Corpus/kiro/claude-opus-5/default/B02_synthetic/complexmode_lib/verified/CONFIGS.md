# CONFIGS.md — Configuration-surface table (Phase B gate)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

| axis | where | values the C distinguishes |
|------|-------|-----------------------------|
| A1 `complexmode` `mode` | `switch (mode)` `lib.c:115` | `1`, `2`, `3`, `4`, `default` |
| A2 permission mask satisfied? | `check_permissions` `lib.c:48`, used at `lib.c:52` and `lib.c:154` | `(perms & required) == required` → true / false |
| A3 required mask | `lib.c:52` (`READ_PERM\|WRITE_PERM` = `0600`), `lib.c:154` (`0100`) | `0600`, `0100`, plus arbitrary caller-supplied masks incl. `0`, `-1`, `INT_MIN`, `INT_MAX` |
| A4 hard-wired `permissions` | `lib.c:103` `0644` | `0644 & 0600 == 0600` ⇒ A2 **true** at `lib.c:52`; `0644 & 0100 == 0` ⇒ A2 **false** at `lib.c:154`, so mode 4 always takes the `v1+v2+v3` else-branch |
| A5 `copy_and_sum` `count` shape | `lib.c:73,79,82` | `0` (empty), `1` (one), `2..N` (many), `3` (the value `complexmode` mode 3 hard-codes), negative / huge (malloc fails) |
| A6 integer magnitude | `lib.c:56,64,83,155,157` | small, `0`, negative, `INT_MAX`, `INT_MIN`, overflow-producing pairs |
| A7 string shape for `strcmp` | `lib.c:96,131,173` | equal, differing at first byte, differing late, prefix vs longer, empty vs empty, empty vs non-empty, high-bit (≥0x80) bytes, embedded byte values |
| A8 `create_result_string` output length | `malloc(64)` / `snprintf(...,64,...)` `lib.c:39,43` | fits, exactly fills, truncated; `op` empty / `NULL` / long |
| A9 `operation` buffer occupancy | `char operation[32]` `lib.c:34`, `strcpy` `lib.c:113,117,127,141,152` | `"none"`(4), `"addition"`(8), `"multiplication"`(14), `"array_sum"`(9), `"complex"`(7) — all fit; drives the `lib.c:173` `!= "none"` branch |
| A10 `"Operation performed"` trailer | `lib.c:173-175` | printed for modes 1-4, suppressed for `default` |
| A11 entry point level | `include/lib.h` exposes only `complexmode`; `src/lib.c` additionally gives external linkage to 6 lower-level functions | `create_result_string`, `check_permissions`, `safe_add`, `multiply_with_log`, `copy_and_sum`, `compare_operations` (low level) **and** `complexmode` (one-shot wrapper) |

There are **no** `#ifdef`s, no runtime option/flag setters, no global state, no
byte-order or element-type axes, and no Cargo features (`translation/Cargo.toml`
has no `[features]` section — the only build configuration is the default).
Everything observable is a pure function of the arguments plus the `stdout` text.

## Rows (cross-product, pruned to what the C distinguishes)

Every row is driven with **many randomized inputs** (seeded xorshift64\*,
fixed seed `0x243F6A8885A308D3`, ≥256 iterations per row) plus the hard boundary
values `{0, 1, -1, 2, -2, 3, 127, 128, 255, 256, 65535, 65536, INT_MAX, INT_MIN,
INT_MAX-1, INT_MIN+1}`. Both `.so`s are called through `dlopen`/`dlsym` and
compared on **return value + captured stdout bytes**.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `check_permissions` | `required == 0` (vacuous mask) × random `perms` | [x] |
| C2 | `check_permissions` | `required` = single bit `0400`/`0200`/`0100` × `perms` random (subset/superset/disjoint) | [x] |
| C3 | `check_permissions` | `required = 0600` (the `safe_add` mask) × `perms` ∈ {`0`,`0100`,`0200`,`0400`,`0600`,`0644`,`0777`, random} | [x] |
| C4 | `check_permissions` | `required` = random full-width int incl. negative / `INT_MIN` / `INT_MAX` × random `perms` (sign-extension of `&`) | [x] |
| C5 | `safe_add` | perms **satisfying** `0600` (`0600`,`0644`,`0777`,`-1`, random supersets) × random `(a,b)` incl. overflow pairs | [x] |
| C6 | `safe_add` | perms **not** satisfying `0600` × random `(a,b)` → must print refusal and return `0` | [x] |
| C7 | `safe_add` | perms satisfying × boundary `(a,b)` = `(INT_MAX,1)`, `(INT_MIN,-1)`, `(INT_MAX,INT_MAX)`, `(INT_MIN,INT_MIN)`, `(0,0)` (wraparound) | [x] |
| C8 | `create_result_string` | `op` = short ASCII (fits in 64) × random `val` incl. `INT_MIN`/`INT_MAX` (widest `%d`) | [x] |
| C9 | `create_result_string` | `op` = `""` (empty) × random `val` | [x] |
| C10 | `create_result_string` | `op` length swept `0..80` so the formatted string crosses the 64-byte boundary (truncation) × `val` = `INT_MIN` (longest suffix) | [x] |
| C11 | `create_result_string` | `op` containing high-bit / non-ASCII bytes × random `val` | [x] |
| C12 | `multiply_with_log` | random `(a,b)` — asserts both the return value **and** the full `log_msg` buffer contents byte-for-byte, then frees via the producing library | [x] |
| C13 | `multiply_with_log` | boundary `(a,b)` = `(INT_MAX,2)`, `(INT_MIN,-1)`, `(INT_MAX,INT_MAX)`, `(0,x)`, `(x,0)`, `(-1,INT_MIN)` (overflow in `a*b`, computed **twice** in C) | [x] |
| C14 | `copy_and_sum` | `count = 0`, `src` a valid non-empty buffer (empty shape) | [x] |
| C15 | `copy_and_sum` | `count = 1` (one element) × random value incl. `INT_MIN`/`INT_MAX` | [x] |
| C16 | `copy_and_sum` | `count = 3` (the shape `complexmode` mode 3 uses) × random triples | [x] |
| C17 | `copy_and_sum` | `count` random in `2..=1024` (many) × random buffer contents — exercises accumulator wraparound over many adds | [x] |
| C18 | `copy_and_sum` | `count` large enough that the sum wraps repeatedly: all-`INT_MAX`, all-`INT_MIN`, alternating extremes | [x] |
| C19 | `compare_operations` | equal strings (incl. `""` vs `""`) × random lengths | [x] |
| C20 | `compare_operations` | differ at byte 0 (both orders) — exact `strcmp` magnitude | [x] |
| C21 | `compare_operations` | differ at a late byte / common prefix × random | [x] |
| C22 | `compare_operations` | proper prefix vs longer string (both orders), `""` vs non-empty | [x] |
| C23 | `compare_operations` | random bytes incl. `0x80..0xFF` (signed-vs-unsigned `char` comparison) | [x] |
| C24 | `compare_operations` | the literal operation names the library itself uses: `none`/`addition`/`multiplication`/`array_sum`/`complex` in all pairings | [x] |
| C25 | `complexmode` | `mode = 1` (addition; perms `0644` ⇒ mask satisfied) × random `(v1,v2,v3)` — asserts return + 3 stdout lines | [x] |
| C26 | `complexmode` | `mode = 1` × boundary `(v1,v2)` overflow pairs | [x] |
| C27 | `complexmode` | `mode = 2` (multiplication + log string) × random `(v1,v2,v3)` — asserts the `Mode 2: Operation: multiply, Value: N` line | [x] |
| C28 | `complexmode` | `mode = 2` × boundary/overflow `(v1,v2)` incl. products that need all 11 `%d` digits | [x] |
| C29 | `complexmode` | `mode = 3` (array sum via `copy_and_sum(values,3)`) × random triples | [x] |
| C30 | `complexmode` | `mode = 3` × boundary triples (`INT_MAX,INT_MAX,INT_MAX`, `INT_MIN,INT_MIN,INT_MIN`, mixed) | [x] |
| C31 | `complexmode` | `mode = 4` — `check_permissions(0644,0100)` is **false**, so the `v1+v2+v3` else-branch; × random triples | [x] |
| C32 | `complexmode` | `mode = 4` × boundary triples (wraparound in the else-branch) | [x] |
| C33 | `complexmode` | `mode ∉ {1,2,3,4}`: `0`, `5`, `6`, `-1`, `100`, `INT_MIN`, `INT_MAX`, random — `default` arm, no trailer line | [x] |
| C34 | full pipeline: `complexmode` all 5 arms in sequence in one process | mode 1→2→3→4→default with the same values, comparing the **concatenated** stdout of the whole sequence (catches state/leak-order differences invisible per-call) | [x] |
| C35 | composed low-level pipeline | `check_permissions` → `safe_add` → `multiply_with_log` → `create_result_string` → `compare_operations` → `copy_and_sum` chained so each output feeds the next input, run per-library, comparing the whole transcript | [x] |

## Gate

- [x] Every row passes across its randomized inputs, C vs Rust, via `.so` exports.
- [x] Project builds **no** binary executable (`CMakeLists.txt` declares only
      `add_library(... SHARED src/lib.c)`; `Cargo.toml` has only a `[lib]` with
      `crate-type = ["cdylib"]`), so the "compare binary stdout" gate is N/A.
- [x] Only one build configuration exists (no Cargo features, no `#ifdef`s).
