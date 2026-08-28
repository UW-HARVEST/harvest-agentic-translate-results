# Configuration surface

Rows are derived from public exports plus each branch, mode, threshold, and
input-shape distinction in `src/lib.c`. `BINARY` is mode `0`; string comparison
is selected by every mode `>= 1`. String ownership mode is stored separately as
`NONE` (`0`), `DEFAULT` (`1`), `STRDUP` (`2`), or `ARENA` (`3`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `stbds_arrgrowf` | null array; `addlen == 0`, `min_cap == 0`; early return leaves it null | [x] |
| C02 | `stbds_arrgrowf` | null array; requested capacity in `1..3`; minimum-four branch | [x] |
| C03 | `stbds_arrgrowf` | null array; explicit `min_cap >= 4`; exact requested-capacity branch; varied element widths | [x] |
| C04 | `stbds_arrgrowf` | existing array; request `<= capacity`; pointer/capacity unchanged | [x] |
| C05 | `stbds_arrgrowf` | existing array; `length + addlen` selects growth and doubling wins | [x] |
| C06 | `stbds_arrgrowf` | existing array; explicit `min_cap` exceeds double-capacity and wins | [x] |
| C07 | `stbds_arrfreef` | free arrays produced by C/Rust after varied growth histories | [x] |
| C08 | `stbds_rand_seed` | seed `0`, small, high-bit, and `SIZE_MAX`; observe through the next binary/string table | [x] |
| C09 | `stbds_hash_string` | empty NUL-terminated string with varied seeds | [x] |
| C10 | `stbds_hash_string` | one/many bytes, including bytes with the high bit set, with varied seeds | [x] |
| C11 | `stbds_hash_bytes` | length `0` (including null data pointer) with varied seeds | [x] |
| C12 | `stbds_hash_bytes` | tail lengths `1..7`, covering every fall-through case and high-bit bytes | [x] |
| C13 | `stbds_hash_bytes` | exactly one full `size_t` block (`8` bytes on this ABI) | [x] |
| C14 | `stbds_hash_bytes` | multiple full blocks plus tail lengths `0..7` | [x] |
| C15 | `stbds_hmput_default` | null map creates zeroed default entry | [x] |
| C16 | `stbds_hmput_default` | existing default-only map and populated map return unchanged | [x] |
| C17 | `stbds_hmget_key_ts` | null map; binary and string mode; result default entry and `temp == -1` | [x] |
| C18 | `stbds_hmget_key` | null map; wrapper stores `-1` in header temp | [x] |
| C19 | `stbds_hmget_key_ts` / `stbds_hmget_key` | default-only map with no hash table; missing binary/string key | [x] |
| C20 | `stbds_hmput_key` / get APIs | binary mode; new key and duplicate update lookup; key widths `1, 4, 8, 16` | [x] |
| C21 | `stbds_hmput_key` / get APIs | binary mode; many unique keys cross 8-slot growth thresholds and rehash | [x] |
| C22 | `stbds_hmput_key` / get APIs | string compare selected by mode `1`; default borrowed-key ownership; empty/one/many strings | [x] |
| C23 | `stbds_hmput_key` / get APIs | string compare selected by mode `>1`; same default ownership when map was implicitly created | [x] |
| C24 | `stbds_shmode_func` | explicit ownership mode `NONE` (`0`) followed by binary puts | [x] |
| C25 | `stbds_shmode_func` | explicit ownership mode `DEFAULT` (`1`) followed by borrowed string puts | [x] |
| C26 | `stbds_shmode_func` | explicit ownership mode `STRDUP` (`2`) followed by mutated/freed source strings | [x] |
| C27 | `stbds_shmode_func` | explicit ownership mode `ARENA` (`3`) followed by mutated/freed source strings | [x] |
| C28 | `stbds_hmdel_key` | null map, default-only map, and populated map with missing key | [x] |
| C29 | `stbds_hmdel_key` | binary map; delete last element and non-last element (move-last repair) | [x] |
| C30 | `stbds_hmdel_key` | string map in `DEFAULT`, `STRDUP`, and `ARENA`; delete last/non-last | [x] |
| C31 | `stbds_hmdel_key` / `stbds_hmput_key` | deletion creates tombstone; later insertion reuses it | [x] |
| C32 | `stbds_hmdel_key` | enough deletions to exceed tombstone threshold and rebuild same-size table | [x] |
| C33 | `stbds_hmdel_key` | grow beyond one bucket, then delete enough entries to cross shrink threshold | [x] |
| C34 | `stbds_hmfree_func` | null map and default-only map | [x] |
| C35 | `stbds_hmfree_func` | populated binary and borrowed-string maps | [x] |
| C36 | `stbds_hmfree_func` | populated `STRDUP` and `ARENA` maps release owned storage | [x] |
| C37 | `stbds_stralloc` | zeroed arena; empty and small strings allocate first 512-byte block | [x] |
| C38 | `stbds_stralloc` | repeated strings fit in remaining current block | [x] |
| C39 | `stbds_stralloc` | exhaustion allocates progressively larger normal blocks up to max growth branch | [x] |
| C40 | `stbds_stralloc` | string length exceeds current block size and uses dedicated oversize block, with/without existing storage | [x] |
| C41 | `stbds_strreset` | zeroed empty arena | [x] |
| C42 | `stbds_strreset` | arena with normal and dedicated oversize blocks; all fields become zero | [x] |
| C43 | `strkey` | negative, zero, positive, and integer-limit values; returned bytes and static-buffer replacement | [x] |
| C44 | `helxo` | printable positive `char`; duplicate `"jen"` replaces value and output order is preserved | [x] |
| C45 | `helxo` | NUL, newline, high-bit, and signed-negative `char`; raw stdout bytes | [x] |
| C46 | composed low-level map pipeline | seed, create/default, mixed put/get/delete/reinsert/free sequence in binary mode with randomized operations | [x] |
| C47 | composed low-level map pipeline | same randomized operation sequence for borrowed string mode | [x] |
| C48 | composed low-level map pipeline | same randomized operation sequence for `STRDUP` and `ARENA` ownership modes | [x] |

Cargo feature combinations: none are declared in `Cargo.toml`; the only build
configuration is the no-feature/default configuration.

Coverage is implemented in `tests/differential.rs`. The test comments name the
row ranges, and deterministic threshold/probe tests supplement the randomized
binary and string operation streams.
