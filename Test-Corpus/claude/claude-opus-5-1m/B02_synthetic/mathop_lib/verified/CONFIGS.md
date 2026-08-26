# CONFIGS.md — Phase A configuration-surface table

## Build-time configuration axes

| axis | source | values |
|------|--------|--------|
| Cargo features | `Cargo.toml` — **no `[features]` section** | exactly one: default (empty) |
| C preprocessor | `lib.c` / `lib.h` — no `#if`/`#ifdef` | none |
| CMake options | `c_src/CMakeLists.txt` — no `option()`/`add_definitions` | none |

→ **Total valid feature combinations: 1** (`--no-default-features`, which is
identical to the default build). Phases B–C therefore run once, under
`--no-default-features` and again under the plain default, to satisfy the
"every combination" requirement.

## Runtime configuration axes (derived from the branches the C takes)

| axis | where the C branches on it | distinct values the C distinguishes |
|------|---------------------------|-------------------------------------|
| A. `Operation` selector | `select_operation` `switch` (`lib.c:89-102`) | `1` add, `2` mul, `3` sub, `4` div, `5` mod, `default` (`0`, `6`, negative, `INT_MIN`, `INT_MAX`) |
| B. operand sign/magnitude shape | `a/b` in the five ops; `/` and `%` truncate toward zero | `0`, `+`, `−`, mixed signs, `±1`, `INT_MIN`, `INT_MAX`, overflow-producing pairs |
| C. divisor zero-ness | `if (b == 0)` in div/mod (`lib.c:75`, `82`) | `b == 0`, `b != 0` |
| D. history pointer state | `if (*history == NULL)` (`lib.c:122`) | `*history == NULL` (lazy alloc + count reset), `*history != NULL` (reuse) |
| E. history fill level | `if (*history_count < 10)` (`lib.c:127`) | `< 0`, `0`, `1..8`, `9` (last writable), `10`, `> 10` |
| F. `allocate_results` count | `calloc(count, 24)` (`lib.c:113`) | `0`, `1`, `10`, large-but-ok, `INT_MAX`, negative |
| G. `char` classification | `op_char && op_char >= '1' && op_char <= '5'` (`lib.c:53`) | `0`, `1..47`, `'1'..'5'`, `54..127`, `−128..−1` |
| H. priority overflow | `op * 10` (`lib.c:58`) | small, `> INT_MAX/10`, `< INT_MIN/10` |
| I. `mathop` op derivation | `(param3 % 5) + 1` (`lib.c:148`), `((param4 + 1) % 5) + 1` (`lib.c:156`) | each of `−3..5` reachable for op1; ditto op2; `param4 == INT_MAX` overflow |
| J. `mathop` static history | `static` locals (`lib.c:138-139`) | fresh (count 0), partially filled (2,4,6,8), saturated (10) |
| K. `mathop` validation char | `(char)(param1 % 128)` → axis G | valid `'1'..'5'`, invalid (dead-store branch) |
| L. `time_t` shift | `time(&t); t >> 29` (`lib.c:108`) | single runtime value; also `% 100` feeding the result |

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised through **both** `.so` exports with randomized
inputs (fixed-seed PCG, ≥ 200 samples/row unless the domain is smaller,
in which case the domain is swept **exhaustively**).

### Lowest level: leaf predicates and arithmetic

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `is_valid_operation` | exhaustive sweep of **all 256** `char` bit patterns (`−128..127`), covering G: zero / below range / in range / above range / negative | [x] |
| C2 | `get_operation_priority` | ops `−16..16` swept exhaustively (covers A incl. `default` values) | [x] |
| C3 | `get_operation_priority` | H: overflow region — `INT_MIN`, `INT_MAX`, `±(INT_MAX/10)`, `±(INT_MAX/10 ± 1)`, plus randomized full-range `i32` | [x] |
| C4 | `add_operation` | B: randomized full-range pairs + boundary matrix `{0,±1,±2,INT_MIN,INT_MAX,INT_MIN+1,INT_MAX−1}²` (includes wrapping overflow) | [x] |
| C5 | `multiply_operation` | B: same matrix + randomized, incl. overflow (`INT_MAX*INT_MAX`, `INT_MIN*−1`) | [x] |
| C6 | `subtract_operation` | B: same matrix + randomized, incl. `INT_MIN − 1` wrap | [x] |
| C7 | `divide_operation` | B×C, `b != 0`: same matrix + randomized; sign combinations verify truncation toward zero (`−7/2 == −3`); `INT_MIN/−1` excluded (UB, ERRORS.md E21) | [x] |
| C8 | `modulo_operation` | B×C, `b != 0`: same matrix + randomized; verifies remainder keeps the dividend's sign (`−3%5 == −3`) | [x] |
| C9 | all five ops | `unused_param` varied (`0`, `±1`, `INT_MIN`, `INT_MAX`, random) to prove it is ignored identically | [x] |
| C10 | `select_operation` | A: op swept `−16..16` plus `INT_MIN`, `INT_MAX`, randomized; the **returned function pointer is identified** against that library's own 5 exported op symbols, so C-vs-Rust selection identity is compared, not just the numeric result | [x] |
| C11 | `select_operation` → returned `MathOperation` | A×B: the pointer returned by each library is **invoked** with the boundary matrix + randomized operands; both libraries' dispatched results compared | [x] |
| C12 | `get_computation_timestamp` | L: no inputs; value compared between libraries (and `>> 29` semantics cross-checked against a fresh `time()` reading) | [x] |

### Allocation

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C13 | `allocate_results` | F: `count = 1`, `10`, `2`, `100`, `1000` → non-NULL, block is **fully zeroed** for `count*24` bytes (verified through the FFI pointer), and each block is `free()`d | [x] |
| C14 | `allocate_results` | F: `count = 0` → both non-NULL (glibc degenerate request) | [x] |
| C15 | `allocate_results` | F: `count < 0` (`−1`, `−10`, `INT_MIN`) → both NULL (see E7) | [x] |
| C16 | `allocate_results` + `ComputationResult` layout | struct ABI shape: `size == 24`, `align == 8`, offsets `value = 0`, `timestamp = 8`, `status = 16` — probed by writing through one library's pointer and reading it back through the other library's `perform_computation_with_history` records | [x] |

### `perform_computation_with_history` (low-level composed entry point)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C17 | `perform_computation_with_history` | D=`*history == NULL`, E=count `0`: lazy allocation path; record `{value, timestamp, STATUS_SUCCESS}` and `*history_count == 1` compared field-by-field | [x] |
| C18 | `perform_computation_with_history` | D=`*history == NULL`, E=count `7` (non-zero) → count **reset to 0** then record written at slot 0 (E10) | [x] |
| C19 | `perform_computation_with_history` | D=`*history != NULL` (caller-owned buffer), E=`0..9` sweep × A=`1..5` → record lands at the indexed slot, neighbours untouched | [x] |
| C20 | `perform_computation_with_history` | D=`*history != NULL`, E=`9` (last writable) → write at 9, count becomes `10` | [x] |
| C21 | `perform_computation_with_history` | D=`*history != NULL`, E=`10` and `11` (full) → result returned, **buffer unmodified**, count unchanged (E11) | [x] |
| C22 | `perform_computation_with_history` | D=`*history != NULL`, E=negative (`−1`, `−3`) with a padded buffer so the OOB write lands in owned memory → identical negative-index write and increment (E12) | [x] |
| C23 | `perform_computation_with_history` | A=`default` (op `0`, `6`, `−1`, `INT_MIN`, `INT_MAX`) × randomized operands → add semantics in both (E13) | [x] |
| C24 | `perform_computation_with_history` | A=`4`/`5` × C=`b == 0` → recorded `value == 0` (E14) | [x] |
| C25 | `perform_computation_with_history` | full sequence: 12 successive calls on one buffer (crosses the 10-slot limit) with randomized ops/operands → whole 10-record buffer + final count compared byte-for-byte | [x] |
| C26 | `perform_computation_with_history` | interop: buffer allocated by **C's** `allocate_results` then driven by **Rust's** function, and vice versa (proves identical struct layout/ownership across the ABI) | [x] |

### `mathop` (public one-shot wrapper, stateful)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C27 | `mathop` | J=fresh statics, K=valid validation char (`param1 % 128 ∈ '1'..'5'`), I=op1 each of `1..5` | [x] |
| C28 | `mathop` | K=invalid validation char (dead-store branch, E15), incl. `param1 % 128 == 0` and `param1 = INT_MIN` | [x] |
| C29 | `mathop` | I: `param3` chosen so `selected_op` takes each reachable value `−3,−2,−1,0,1,2,3,4,5` (negative-priority rows E16/E17) | [x] |
| C30 | `mathop` | I: `param4` chosen so `second_op` takes each reachable value `−3..5`, including `param4 == INT_MAX` overflow (E18) | [x] |
| C31 | `mathop` | C: `param2 == 0` while `selected_op ∈ {4,5}` (div/mod guard) and `param4 == 0` while `second_op ∈ {4,5}` | [x] |
| C32 | `mathop` | J: lockstep call sequence of 12 calls, covering counts `2,4,6,8,10,10,…` — return value **and all four `printf` lines** (stdout captured via `dup2`) compared per call (E19) | [x] |
| C33 | `mathop` | randomized full-range `(param1,param2,param3,param4)` — 400 lockstep quadruples, return value + stdout compared (UB pairs from E21 filtered by the generator) | [x] |
| C34 | `mathop` | boundary quadruples: every combination of `{INT_MIN, INT_MIN+1, −1, 0, 1, INT_MAX−1, INT_MAX, 127, 128, −128}` over the 4 params (pruned by the UB guard) | [x] |

### Timestamp / clock axis (axis L) — driven with an `LD_PRELOAD` fixture

Wall-clock time pins `time() >> 29` to a single value (currently `3`), which
hides the whole timestamp code path. `tests/support/faketime.c` interposes
libc's `time()` for **both** libraries at once, so these rows drive real
alternatives. (Mutation testing confirms they have teeth: without them, changing
`% 100` to `% 10`, or the arithmetic shift to a logical one, is invisible.)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C35 | `get_computation_timestamp` | L: `time()` = `0`, `1`, `2^29-1`, `2^29` → `ts` = `0,0,0,1` (shift boundary) | [x] |
| C36 | `get_computation_timestamp` | L: `time()` = `10*2^29`, `45*2^29`, `137*2^29` → `ts` = `10`, `45`, `137` (distinguishes `% 100` from `% 10`) | [x] |
| C37 | `get_computation_timestamp` | L: **negative** `time()` (`-1`, `-2^29`, `-2^29-1`, `-137*2^29`) → arithmetic (sign-propagating) shift, negative `ts` | [x] |
| C38 | `get_computation_timestamp` | L: `time()` = `i64::MAX` / `i64::MIN` → `ts` = `±1.7e10`, exercising the narrowing `(int)(ts % 100)` and `printf("%ld")` of a large value | [x] |
| C39 | `perform_computation_with_history` | L × A: the `timestamp` field written into records for each faked clock and each op `1..5` | [x] |
| C40 | `mathop` | L × I × randomized quadruples: return value **and** all four printed lines under every faked clock (negative `time_modifier` included) | [x] |

## Row → test mapping

| rows | test |
|------|------|
| C1–C12 | `tests/phase_b_pure.rs` (`c1_*` … `c12_*`) |
| C13–C26 | `tests/phase_b_history.rs` (`c13_*` … `c26_*`) |
| C27–C34 | `tests/phase_b_mathop.rs::phase_b_mathop_all` (sections labelled `C27` … `C34`) |
| C35–C40 | `tests/phase_b_faketime.rs::axis_l_timestamp_values` |

## Verification status

* Feature combinations: **1** (`--no-default-features` == default). Every row was
  run under that combination in **both** the `dev` and `release` profiles
  (`release` also flips `panic = "abort"`), via `./run_verification.sh`.
* Randomized rows use fixed seeds (splitmix64), so failures are reproducible.
* The suite additionally passes against the C reference rebuilt at `-O2` and
  `-O3` (`C_SO_PATH=… cargo test`), i.e. the Rust does not depend on the exact
  code gcc emits at `-O0`.
* `./mutation_check.sh` injects 27 deliberate bugs into `src/lib.rs`; the suite
  catches all 27, and the 2 provably-equivalent control mutants correctly
  survive.
