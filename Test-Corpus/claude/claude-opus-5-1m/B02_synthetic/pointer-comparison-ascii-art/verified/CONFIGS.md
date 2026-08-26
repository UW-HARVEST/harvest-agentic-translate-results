# CONFIGS.md — configuration-surface table (valid inputs)

The axes below are the ones the C code in `c_src/` actually branches on.  They
were derived from the two public headers plus every `if` / `switch` / loop bound
in `c_src/src/{shape,scene,main}.c`.

**Build-time axes** — none.  `Cargo.toml` has no `[features]`,
`c_src/CMakeLists.txt` has no `option()`, and the only `#if` in `c_src` are the
two include guards.  The single build configuration is therefore tested with all
three equivalent cargo invocations (see `SYMBOLS.md`).

**Run-time axes**

| axis | values the C code distinguishes |
|------|---------------------------------|
| A. shape type | `0..=9` (each has its own `init_*`, its own name, its own `width`/`height`, `height` ∈ {4,5,6,7}), plus the out-of-range values of `ERRORS.md` |
| B. shape-manager state | never initialised / initialised / cleaned up / initialised twice |
| C. scene name | `NULL` → `"Untitled Scene"`; `""`; 1 byte; 62 bytes; 63 bytes (the exact `strncpy` limit); 64 bytes; 200 bytes (truncated); embedded spaces/tabs; `%`/`\`/quote characters (they go through `printf("%s")`); high-bit / non-UTF-8 bytes |
| D. shape count in a scene | 0, 1, 2, many, 49, 50 (`MAX_SHAPES_IN_SCENE`), and the rejected 51 |
| E. shape multiset in a scene | all-distinct, with duplicates, permutation of another scene, prefix/superset of another scene |
| F. `scene_remove_shape` index | 0 (first), middle, `count-1` (last), and the rejected values |
| G. persisted file shape (`scene_save`/`scene_load`) | count 0 / 1 / many / 50 / >50; name with/without trailing newline; `\r\n` line endings; extra whitespace around the numbers; extra trailing junk; long (>63 byte) name line; invalid type numbers; count larger/smaller than the number of type lines |
| H. `scene_count` (application state) | 0, 1, 2, 9, 10 (`MAX_SCENES`), after deletions from front/middle/end |
| I. stdin token shape (`fgets` + `sscanf`/`scanf("%d")`) | `"12\n"`, leading spaces/tabs, `"+12"`, `"-5"`, trailing junk (`"12abc"`), two numbers on one line, a number split across lines, empty line, whitespace-only line, line longer than the 256/64 byte buffer, missing final newline, EOF |
| J. entry point | all 28 exported symbols: the 15 low level `shape_*` / `scene_*` functions **and** the 13 `main.c` level ones (`print_menu`, `view_all_shapes`, `create_new_scene`, `add_shape_to_scene`, `remove_shape_from_scene`, `view_scene`, `list_all_scenes`, `save_scene_to_file`, `load_scene_from_file`, `compare_shapes`, `compare_scenes`, `delete_scene`, `main`) |

Every row below is executed against **both** shared objects (`libcdriver.so`
and `libdriver.so`) through `dlopen`/`dlsym` only, and all observable results are
compared byte for byte: the return value, the `stdout` bytes, the `stderr` bytes,
the fields of the returned structs read back through the C layout
(`type`, `name`, `width`, `height`, `art[0..height]`, `shape_count`,
`shapes[0..shape_count]` identity), the contents of every file the call created,
and the process exit status.  Pointer *values* are normalised to first-use ids so
that pointer *identity* is still compared.

Rows marked "randomised" run N ≥ 32 pseudo-random instances (fixed seed
`0x5EED_2026`, so runs are reproducible) covering the value ranges of the axes
involved.

Test locations: `tests/configs_lib.rs` (rows 1-30 and 53-56) and
`tests/configs_app.rs` (rows 31-52).

| # | entry point(s) | configuration (options set + input shape) | ok |
|---|----------------|--------------------------------------------|----|
| 1 | `shape_manager_init` + `shape_get` | axis B: `shape_get(t)` for every `t` in `0..=9` before init → all `NULL`; after init → 10 distinct pointers | [x] |
| 2 | `shape_manager_init` + `shape_get` + struct read-back | axis A × all 10 types: `type`, `name`, `width`, `height`, `art[0..height]` byte-identical | [x] |
| 3 | `shape_print` | axis A: each of the 10 shapes (heights 4,5,6,7) printed to stdout | [x] |
| 4 | `shape_print` | the same shape printed twice in a row / all ten in a randomised order (randomised) | [x] |
| 5 | `shape_type_name` | axis A: all 10 valid types (`"Tree"`…`"Rainbow"`) | [x] |
| 6 | `shape_equals` | axis A × A: all 100 ordered pairs of singletons (1 on the diagonal, 0 elsewhere) | [x] |
| 7 | `shape_manager_cleanup` + `shape_manager_init` | axis B: init → cleanup → init again; identities are fresh, `shape_get` valid again, `shape_print` unchanged | [x] |
| 8 | `scene_create` + struct read-back | axis C: `NULL` name → `"Untitled Scene"`, `shape_count == 0` | [x] |
| 9 | `scene_create` + struct read-back | axis C: `""`, 1, 30, 62, 63, 64, 65, 200 byte names — full 64-byte `name` buffer compared (`strncpy` zero-padding included) | [x] |
| 10 | `scene_create` | axis C randomised: random length 0..200, random bytes (incl. `%`, `\`, `"`, tabs, high-bit) (randomised) | [x] |
| 11 | `scene_add_shape` + struct read-back | axis D: 0 → 1 → 2 → … adds; `shape_count` and `shapes[i]` identity after every add | [x] |
| 12 | `scene_add_shape` | axis D boundary: exactly 50 successful adds (`shape_count == 50`) | [x] |
| 13 | `scene_add_shape` | axis E: the same shape added repeatedly (duplicates allowed) | [x] |
| 14 | `scene_remove_shape` | axis F: remove index 0 from a 3-shape scene; the shift is compared | [x] |
| 15 | `scene_remove_shape` | axis F: remove the middle and the last index; empty the scene one by one | [x] |
| 16 | `scene_remove_shape` | axis D × F randomised: random scene of 1..50 shapes, random valid index, repeated until empty (randomised) | [x] |
| 17 | `scene_print` | axis D: 0, 1, 3, 50 shapes (the `"Contains %d shape(s)"` header and every sub-shape) | [x] |
| 18 | `scene_print` | axis C × D: name with `%`/`\`/spaces/high-bit bytes and 2 shapes | [x] |
| 19 | `scene_list_shapes` | axis D: 0, 1, 3, 50 shapes — `%p` identities normalised and compared | [x] |
| 20 | `scene_equals` | axis E: identical order, identical multiset in a different order (permutation) | [x] |
| 21 | `scene_equals` | axis E: duplicates on one side only, subset, superset, disjoint sets of equal size | [x] |
| 22 | `scene_equals` | axis D × E randomised: two random scenes (0..50 shapes each) (randomised) | [x] |
| 23 | `scene_equals` | axis D: both scenes empty → 1 | [x] |
| 24 | `scene_save` | axis D × C: 0/1/3/50 shapes, plain name — file bytes compared | [x] |
| 25 | `scene_save` | axis C: name that needs truncation, name with spaces; file bytes compared | [x] |
| 26 | `scene_load` | axis G: files written by `scene_save` (round trip, 0/1/3/50 shapes) | [x] |
| 27 | `scene_load` | axis G: hand-written files — `\r\n`, no trailing newline, extra blank lines, spaces around the numbers, extra junk after the last type | [x] |
| 28 | `scene_load` | axis G: name line longer than 63 bytes (`fgets` stops at 63, the rest becomes the count line) | [x] |
| 29 | `scene_load` | axis G randomised: random count 0..60, random type values (valid and invalid), random separators (randomised) | [x] |
| 30 | `scene_destroy` | axis D: destroy an empty scene / a scene with shapes; the singletons stay valid afterwards (`shape_print` still works) | [x] |
| 31 | `print_menu` | no state — the exact 17-line menu block | [x] |
| 32 | `view_all_shapes` | axis B: after `shape_manager_init` (all 10 shapes); and *before* init (10 × `"(null shape)"`) | [x] |
| 33 | `create_new_scene` | axis C × I: names `""`, `"A"`, 63 bytes, 200 bytes (`fgets` splits it — the remainder is read as the *next* line), `"  spaced  "`, non-UTF-8 bytes | [x] |
| 34 | `create_new_scene` | axis H: 1st … 10th scene (`(index %d)` counter) | [x] |
| 35 | `add_shape_to_scene` | axis A × H × I: valid scene index (0, last), every shape type, input given as `"0\n"`, `" 0 \n"`, `"+0\n"`, `"0 5\n"` (two numbers on one line), `"0"` (no newline, EOF) | [x] |
| 36 | `add_shape_to_scene` | axis D boundary: 50 successful adds then the 51st | [x] |
| 37 | `remove_shape_from_scene` | axis D × F: 1-based index 1 (first), middle, last of a 5-shape scene | [x] |
| 38 | `view_scene` | axis H × D: scene 0 of 1, last scene of many, empty and non-empty scenes | [x] |
| 39 | `list_all_scenes` | axis H: 1, 2, 10 scenes with different shape counts and names | [x] |
| 40 | `save_scene_to_file` | axis H × D × G: save scene 0 and the last scene; file name plain, with spaces, 255 bytes | [x] |
| 41 | `load_scene_from_file` | axis G × H: load a file saved by the same run; load twice (two scenes); load into slot 9 | [x] |
| 42 | `compare_shapes` | axis A × A: `(0,0)`, `(0,1)`, `(9,9)`, `(3,7)` — pointer print, `Comparison of pointers`, verdict | [x] |
| 43 | `compare_scenes` | axis E × H: two identical scenes, two permutations, two different scenes, an empty vs. an empty | [x] |
| 44 | `delete_scene` | axis H: delete index 0 of 3 (shift), the middle, the last, then all of them | [x] |
| 45 | `main` | axis I: `"12\n"` (exit path), `""` (EOF path), `"1\n12\n"`, no trailing newline | [x] |
| 46 | `main` | full session: create 2 scenes, add shapes, view, list, save, load, compare, remove, delete, exit | [x] |
| 47 | `main` | axis I: leading spaces / tabs / `"+12"` / `"12abc"` / a 300-byte line (the 256-byte `fgets` buffer splits it) | [x] |
| 48 | `main` | axis H boundary: 12 × "create scene" (the 11th and 12th are rejected) then list | [x] |
| 49 | `main` | axis D boundary: 52 × "add shape" to one scene (the 51st/52nd hit the full-scene path) then view | [x] |
| 50 | `main` | axis G: load a hand-written scene file (valid, `\r\n`, invalid types, count > 50) then view + list | [x] |
| 51 | `main` | randomised sessions: random menu choices `-2..=14` with random arguments, ≥ 64 sessions (randomised) | [x] |
| 52 | `main` | randomised sessions restricted to the "well formed" grammar (create/add/remove/save/load/compare/delete/exit with valid indices), ≥ 64 sessions (randomised) | [x] |
| 53 | `scene_create` + `scene_add_shape` + `scene_print` + `scene_list_shapes` + `scene_save` + `scene_equals` | axis J: both structs are **public**, so a caller may write to them - `shape_count` shrunk behind the library's back, `name[]` overwritten in place (shorter / non-UTF-8 / `%s`), `shapes[]` reordered by hand.  Every function must read the struct, not private state. | [x] |
| 54 | `shape_get` + `shape_print` + `scene_save` + `scene_list_shapes` | axis A × J: the caller patches the singleton's `height` (0, 1, 3, 7), its `name`, its `type` (which is what `scene_save` writes) and one `art` row | [x] |
| 55 | `scene_save` twice to the same path + `scene_load` | axis G: `fopen(…, "w")` must truncate, so the second (shorter) scene replaces the first one completely | [x] |
| 56 | `scene_load` + `scene_print` + `scene_list_shapes` + `scene_save` | axis C × G: a scene file whose name line is non-UTF-8 and contains `%s` and control bytes | [x] |

## Coverage summary

| test binary | cases actually executed | rows |
|-------------|------------------------|------|
| `tests/configs_lib.rs` | 34 differential cases; the randomised rows run 32-64 inputs each inside their case | 1-30, 53-56 |
| `tests/configs_app.rs` | 218 differential cases, each a fresh pair of child processes | 31-52 |
| `tests/binary_diff.rs` | 179 scenarios against the `driver` *executable* (51 curated + 128 randomised) | the same axes as seen by a user |

Every row is `[x]`: it was executed against both shared objects and all
observable output matched byte for byte.  `cargo test -- --nocapture` prints the
per-file case counts; `DIFF_DUMP=1 cargo test -- --nocapture` additionally dumps
the full C transcript of every case (that is how the tables were reviewed for
"the test really did what the row says").
