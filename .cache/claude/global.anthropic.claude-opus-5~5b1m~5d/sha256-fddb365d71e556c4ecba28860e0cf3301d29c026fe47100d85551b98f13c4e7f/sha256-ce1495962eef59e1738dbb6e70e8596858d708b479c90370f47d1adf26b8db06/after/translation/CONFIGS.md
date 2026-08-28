# CONFIGS.md — Phase A configuration surface table (valid inputs)

Mirror of `ERRORS.md` for *valid* inputs. Axes were derived mechanically from
the branches the C actually takes.

## Cargo feature axis

`translation/Cargo.toml` declares **no `[features]` table**, so the feature
cross-product is the single combination `{}` (= default = `--no-default-features`).
Phases B and C are nevertheless executed for every buildable configuration:

| cfg | how it is exercised | result |
|-----|---------------------|--------|
| default features, dev profile `.so`   | `cargo build && cargo test` | PASS |
| `--no-default-features`, dev profile  | `cargo build --no-default-features && cargo test --no-default-features` | PASS |
| `--all-features`, dev profile         | `cargo build --all-features && cargo test --all-features` | PASS |
| default features, **release** `.so` (`panic = "abort"`) | `cargo build --release && cargo test --release` | PASS |
| `--no-default-features`, release      | `… --release --no-default-features` | PASS |
| `--all-features`, release             | `… --release --all-features` | PASS |

All six are driven by `./run-diff-tests.sh`.

The harness loads `target/release/libconfusion_lib.so` when the test binary is
built with optimisations and `target/debug/...` otherwise, so the *shipped*
release object really is the one under test in the release rows. There is
deliberately **no cross-profile fallback**: `cargo test` does not build a
`cdylib`, so a fallback would silently test the other profile's object. Each
configuration therefore runs `cargo build` first. Overridable with
`RUST_LIB_PATH` (used by the mutation check).

## Runtime option / mode axes found in the C

| axis | source | distinct values the C branches on |
|------|--------|-----------------------------------|
| `confuse_types(operation)` | `switch`, `lib.c:150` | `0`, `1`, `2`, `3`, (fall-through) |
| `update_flags(param)` bit 0 → `flag1` | `lib.c:132` | 0, 1 |
| `update_flags(param)` bit 1 → `flag2` | `lib.c:133` | 0, 1 |
| `update_flags(param)` bit 2 → `flag3` | `lib.c:134` | 0, 1 |
| `update_flags(param)` bits 3..5 → `mode` | `lib.c:135` | 0..7 |
| `update_flags` call count → 5-bit `counter` | `lib.c:131` | 0..31 then wrap |
| `create_state(capacity)` | `malloc` + `snprintf` size, `lib.c:76,84` | 0, 1, `<len`, `==len+1`, `>len`, 128, huge, negative |
| `create_state(initial_val)` = the union payload | `lib.c:73` | any `int32` bit pattern; specially: float NaN / ±Inf / ±0 / subnormal / small / `*100` overflow |
| `process_buffer(target)` | `memchr`, `lib.c:110` | present-once / present-many / absent / first byte / last byte / `'\0'` / negative (high-bit) |
| `confusion(param3)` → search char | `lib.c:193` | `param3 % 10 ∈ -9..9` → char 39..57 |
| `confusion(param4)` → operation | `lib.c:197` | `param4 % 4 ∈ -3..3` |
| byte order / element width | fixed | little-endian x86-64, `char` **signed**, bit-fields LSB-first |

## Configuration rows

Each row is run against **many randomized inputs** (fixed-seed SplitMix64) and
compared C-vs-Rust on: return value, every observable byte of the returned
`ProcessState` (flags word, union word, capacity, buffer contents), and the
full captured **stdout**.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C1  | `create_state` + raw struct read + `destroy_state` | `capacity = 128`, `initial_val` random over the whole `int32` range; assert flags word `== 0x00007B05`, union word, capacity, buffer string | [x] |
| C2  | `create_state` | `initial_val` boundary set `{0, 1, -1, INT_MAX, INT_MIN, 1078530011}`, `capacity = 128` | [x] |
| C3  | `create_state` | `capacity ∈ {1,2,…,20}` (spans `snprintf` truncation, exact fit, slack) × random `initial_val` | [x] |
| C4  | `create_state` | `capacity` large-but-servable `{64, 128, 256, 4096, 65536, 1<<20}` × random `initial_val` | [x] |
| C5  | `create_state`+`destroy_state` | alloc/free round-trip repeated many times, no leak/crash divergence | [x] |
| C6  | `process_buffer` | fresh `create_state(random, 128)` buffer, `target` = each of the 10 digits `'0'..'9'` | [x] |
| C7  | `process_buffer` | same buffer, `target` = each literal char of `"State:Mode:"` (`':'` occurs twice → multi-match loop) | [x] |
| C8  | `process_buffer` | `target` absent from the buffer (`'z'`, `'~'`, `' '`) → 0 matches | [x] |
| C9  | `process_buffer` | `target` = first byte `'S'` and = last byte of the string (match at position 0 / at `len-1`) | [x] |
| C10 | `process_buffer` | random `target` over the **full** `char` range `-128..127` × random buffer | [x] |
| C11 | `process_buffer` | truncated buffers (`capacity ∈ 1..20`) × every digit target — short/one-char/empty-string shapes | [x] |
| C12 | `process_buffer` | called repeatedly on the same state (idempotence: no state mutation) | [x] |
| C13 | `update_flags` | `param` = all 64 values `0..63` (full cross-product of `flag1×flag2×flag3×mode`) | [x] |
| C14 | `update_flags` | `param` random over full `int32` (incl. negatives → arithmetic `>>`) | [x] |
| C15 | `update_flags` | called 40× in a row on one state → counter `1..31,0,1,…` wrap-around, stdout of every call compared | [x] |
| C16 | `update_flags` then raw struct read | `param` random; assert the whole 32-bit flags word (proves `status`/`reserved` bits are preserved) | [x] |
| C17 | `confuse_types` | `operation = 0` (writes `1078530011`) × random `initial_val`; then re-read via op 1/2/3 | [x] |
| C18 | `confuse_types` | `operation = 1`, `initial_val` random over full `int32` → arbitrary `float` bit patterns (`%f` print + `cvttss2si`) | [x] |
| C19 | `confuse_types` | `operation = 1`, `initial_val` = curated float bit patterns: `+0.0`, `-0.0`, `+Inf`, `-Inf`, qNaN, sNaN, `-NaN`, subnormals, `FLT_MAX`, `1.0`, `-1.0`, `1e7`, `2.2e7` (`*100` straddles `INT_MAX`) | [x] |
| C20 | `confuse_types` | `operation = 2` × random `initial_val` (`%u` print + `& 0xFF`) | [x] |
| C21 | `confuse_types` | `operation = 3` × random `initial_val` (4 signed bytes printed + `bytes[0]+bytes[1]`) | [x] |
| C22 | `confuse_types` | ops applied **in sequence** `0→1→2→3` on one state (op 0 mutates the union that ops 1–3 then read) | [x] |
| C23 | `confuse_types` | random permutations of `operation ∈ {0,1,2,3}` sequences of length 6 on one state | [x] |
| C24 | full low-level pipeline | `create_state` → `update_flags` → `process_buffer` → `confuse_types` → raw struct → `destroy_state`, all four params random (replicates `confusion` by hand) | [x] |
| C25 | full low-level pipeline, reordered | `create_state` → `confuse_types` → `process_buffer` → `update_flags` → `destroy_state` (order the convenience wrapper never produces) | [x] |
| C26 | full low-level pipeline, N×`update_flags` | `create_state` → `update_flags`×k (k random 1..40) → `process_buffer` → `confuse_types` (counter ≠ 1, unlike `confusion`) | [x] |
| C27 | `confusion` | all four params random over the full `int32` range | [x] |
| C28 | `confusion` | `param4 % 4` forced to each of `0,1,2,3` × random `param1..param3` | [x] |
| C29 | `confusion` | `param4 % 4` forced to each of `-1,-2,-3` (negative operation → fall-through) | [x] |
| C30 | `confusion` | `param3 % 10` forced to each of `0..9` and `-1..-9` (digit vs non-digit search char) | [x] |
| C31 | `confusion` | `param2` = all `0..63` (full flag/mode cross-product through the wrapper) | [x] |
| C32 | `confusion` | boundary params: every combination drawn from `{0,1,-1,INT_MAX,INT_MIN,7,8,-8,1078530011}`⁴ (sampled) | [x] |
| C33 | `confusion` | `param1` = the curated float bit-pattern set of C19 × `param4 % 4 == 1` (overflowing `cvttss2si` inside the wrapper) | [x] |
| C34 | `confusion` | repeated invocation (state independence between calls) | [x] |
