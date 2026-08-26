# ERRORS.md — error-surface table

Every distinct way the C code in `c_src/` rejects, refuses or errors on input.
Derived mechanically by walking every `if (…) return`, every `!ptr` check, every
range check, every `< 0` / `>= MAX` comparison, every `fprintf(stderr, …)`,
every `printf("Invalid …")`, every `!= 1` scanf check and every `NULL`/`-1`
return statement in `c_src/src/{shape,scene,main}.c`.

`[x]` = a differential test constructs exactly that condition, calls **both**
shared objects through their exported symbols and asserts the same
error/sentinel result (return value *and* the produced `stdout`/`stderr`).

Test locations: `tests/errors_lib.rs` (rows 1-38, `shape.c` + `scene.c`) and
`tests/errors_app.rs` (rows 39-78, `main.c` + the generic FFI boundary rows).
Rows 79-81 (filesystem level `fopen` failures) are in `tests/errors_lib.rs`.

## `shape.c`

| # | function | trigger (exact invalid input/condition) | expected C result | ok |
|---|----------|------------------------------------------|-------------------|----|
| 1 | `shape_manager_init` | `malloc` returns `NULL` (`shape.c:177`) | `fprintf(stderr,"Error: Failed to allocate shape\n")` then `exit(1)` | [x] `tests/support/failmalloc.c` is `LD_PRELOAD`ed into the harness child and fails every `malloc(2444)` (after 0/1/5/9 successes): both print `Error: Failed to allocate shape` on stderr and `exit(1)` |
| 2 | `shape_get` | `type < 0` (e.g. `-1`, `INT_MIN`) | returns `NULL` | [x] |
| 3 | `shape_get` | `type >= SHAPE_COUNT` (`10`, `11`, `INT_MAX`) | returns `NULL` | [x] |
| 4 | `shape_get` | valid `type` but `shape_manager_init` never ran (`shapes[type]` still `NULL`) | returns `NULL` | [x] |
| 5 | `shape_get` | valid `type` after `shape_manager_cleanup` | returns `NULL` | [x] |
| 6 | `shape_print` | `shape == NULL` | prints `"(null shape)\n"`, returns | [x] |
| 7 | `shape_equals` | any two different pointers (incl. one `NULL`) | returns `0` | [x] |
| 8 | `shape_equals` | both `NULL` | returns `1` (pointer equality!) | [x] |
| 9 | `shape_type_name` | `type` not in `0..=9` (`-1`, `10`, `SHAPE_COUNT`, `INT_MIN`, `INT_MAX`) | returns `"Unknown"` | [x] |

## `scene.c`

| # | function | trigger (exact invalid input/condition) | expected C result | ok |
|---|----------|------------------------------------------|-------------------|----|
| 10 | `scene_create` | `malloc` returns `NULL` (`scene.c:33`) | returns `NULL` | [x] same technique with `malloc(472)`: `scene_create` returns `NULL` in both (checked through `create_new_scene` and through `scene_load`) |
| 11 | `scene_create` | `name == NULL` | name becomes `"Untitled Scene"`, `shape_count = 0` | [x] |
| 12 | `scene_create` | `strlen(name) > MAX_SCENE_NAME-1` (64, 65, 200 bytes) | name truncated to 63 bytes, `name[63] = 0` | [x] |
| 13 | `scene_destroy` | `scene == NULL` | no-op (no crash) | [x] |
| 14 | `scene_add_shape` | `scene == NULL` | returns `-1`, nothing printed | [x] |
| 15 | `scene_add_shape` | `shape == NULL` | returns `-1`, nothing printed | [x] |
| 16 | `scene_add_shape` | both `NULL` | returns `-1` | [x] |
| 17 | `scene_add_shape` | `scene->shape_count >= MAX_SHAPES_IN_SCENE` (51st add) | `fprintf(stderr,"Error: Scene is full\n")`, returns `-1` | [x] |
| 18 | `scene_remove_shape` | `scene == NULL` | returns `-1` | [x] |
| 19 | `scene_remove_shape` | `index < 0` (`-1`, `INT_MIN`) | returns `-1` | [x] |
| 20 | `scene_remove_shape` | `index >= scene->shape_count` (incl. empty scene, `INT_MAX`) | returns `-1` | [x] |
| 21 | `scene_print` | `scene == NULL` | prints `"(null scene)\n"` | [x] |
| 22 | `scene_equals` | `s1 == NULL` | returns `0` | [x] |
| 23 | `scene_equals` | `s2 == NULL` | returns `0` | [x] |
| 24 | `scene_equals` | both `NULL` | returns `0` | [x] |
| 25 | `scene_equals` | `s1->shape_count != s2->shape_count` | returns `0` | [x] |
| 26 | `scene_equals` | equal counts, but a shape of `s1` has no unmatched partner in `s2` | returns `0` | [x] |
| 27 | `scene_save` | `scene == NULL` | returns `-1`, no file, no message | [x] |
| 28 | `scene_save` | `filename == NULL` | returns `-1`, no file, no message | [x] |
| 29 | `scene_save` | `fopen(filename,"w")` fails (`""`, `/nonexistent_dir/x`, a directory, unwritable path) | `fprintf(stderr,"Error: Could not open file '%s' for writing\n")`, returns `-1` | [x] |
| 30 | `scene_load` | `filename == NULL` | returns `NULL`, no message | [x] |
| 31 | `scene_load` | `fopen(filename,"r")` fails (missing file, `""`, a directory) | `fprintf(stderr,"Error: Could not open file '%s' for reading\n")`, returns `NULL` | [x] |
| 32 | `scene_load` | `fgets` returns `NULL` — completely empty file | `fclose`, returns `NULL`, no "Scene loaded" message | [x] |
| 33 | `scene_load` | first `fscanf("%d\n")` != 1 — no count line (`"Name\n"`), or a non-numeric count (`"Name\nxyz\n"`) | `scene_destroy` + `fclose`, returns `NULL` | [x] |
| 34 | `scene_load` | per-shape `fscanf("%d\n")` != 1 — fewer type lines than the count, or a non-numeric type | `scene_destroy` + `fclose`, returns `NULL` | [x] |
| 35 | `scene_load` | a type line outside `0..=9` (`-3`, `99`) | `shape_get` → `NULL` → shape silently skipped, load still succeeds | [x] |
| 36 | `scene_load` | count > 50 with that many valid type lines | first 50 added, then `"Error: Scene is full\n"` on stderr for each further shape; load succeeds | [x] |
| 37 | `scene_load` | negative count (`"N\n-5\n"`) | loop body never runs, load succeeds with 0 shapes | [x] |
| 38 | `scene_list_shapes` | `scene == NULL` | prints `"(null scene)\n"` | [x] |

## `main.c` (application level, all exported)

| # | function | trigger (exact invalid input/condition) | expected C result | ok |
|---|----------|------------------------------------------|-------------------|----|
| 39 | `create_new_scene` | `scene_count >= MAX_SCENES` (11th scene) | prints `"Error: Maximum scenes reached\n"` | [x] |
| 40 | `create_new_scene` | `fgets` returns `NULL` (stdin at EOF) | returns silently, no scene created | [x] |
| 41 | `create_new_scene` | `scene_create` returns `NULL` | prints `"Error creating scene\n"` | [x] with `malloc(472)` failing, both print `Error creating scene` and leave `scene_count` unchanged |
| 42 | `add_shape_to_scene` | `scene_count == 0` | prints `"No scenes available. Create a scene first.\n"` | [x] |
| 43 | `add_shape_to_scene` | 1st `scanf("%d")` != 1 (`"abc"`) | prints `"Invalid input\n"`, then `while(getchar()!='\n')` | [x] |
| 44 | `add_shape_to_scene` | `scene_idx < 0` | prints `"Invalid scene index\n"` | [x] |
| 45 | `add_shape_to_scene` | `scene_idx >= scene_count` | prints `"Invalid scene index\n"` | [x] |
| 46 | `add_shape_to_scene` | 2nd `scanf("%d")` != 1 | prints `"Invalid input\n"` + `getchar` drain | [x] |
| 47 | `add_shape_to_scene` | `shape_type < 0` or `>= SHAPE_COUNT` | prints `"Invalid shape type\n"` | [x] |
| 48 | `add_shape_to_scene` | `scene_add_shape != 0` (scene already holds 50 shapes) | `"Error: Scene is full"` on stderr **and** `"Error adding shape\n"` on stdout | [x] |
| 49 | `remove_shape_from_scene` | `scene_count == 0` | prints `"No scenes available\n"` | [x] |
| 50 | `remove_shape_from_scene` | `scanf` != 1 / bad scene index | `"Invalid input\n"` / `"Invalid scene index\n"` | [x] |
| 51 | `remove_shape_from_scene` | selected scene has `shape_count == 0` | lists shapes, then prints `"Scene is empty\n"` | [x] |
| 52 | `remove_shape_from_scene` | 2nd `scanf` != 1 | `"Invalid input\n"` + `getchar` drain | [x] |
| 53 | `remove_shape_from_scene` | `scene_remove_shape(scene, idx-1) != 0` (`idx` = 0, > count, `INT_MIN` → wrap) | prints `"Error removing shape\n"` | [x] |
| 54 | `view_scene` | `scene_count == 0` / `scanf` != 1 / index out of range | `"No scenes available\n"` / `"Invalid input\n"` / `"Invalid scene index\n"` | [x] |
| 55 | `list_all_scenes` | `scene_count == 0` | prints `"\n=== All Scenes ===\nNo scenes created yet\n"` | [x] |
| 56 | `save_scene_to_file` | `scene_count == 0` / `scanf` != 1 / index out of range | `"No scenes available\n"` / `"Invalid input\n"` / `"Invalid scene index\n"` | [x] |
| 57 | `save_scene_to_file` | filename `fgets` returns `NULL` (EOF) | returns silently | [x] |
| 58 | `load_scene_from_file` | `scene_count >= MAX_SCENES` | prints `"Error: Maximum scenes reached\n"` | [x] |
| 59 | `load_scene_from_file` | filename `fgets` returns `NULL` (EOF) | returns silently | [x] |
| 60 | `load_scene_from_file` | `scene_load` returns `NULL` | only `scene_load`'s stderr message, no `"Scene loaded (index …)"` | [x] |
| 61 | `compare_shapes` | 1st or 2nd `scanf` != 1 | `"Invalid input\n"` + `getchar` drain | [x] |
| 62 | `compare_shapes` | `type1` or `type2` outside `0..=9` (checked *after* both reads) | prints `"Invalid shape type\n"` | [x] |
| 63 | `compare_scenes` | `scene_count < 2` | prints `"Need at least 2 scenes to compare\n"` | [x] |
| 64 | `compare_scenes` | `scanf` != 1 (either read) | `"Invalid input\n"` + `getchar` drain | [x] |
| 65 | `compare_scenes` | `idx1` or `idx2` out of range | prints `"Invalid scene index\n"` | [x] |
| 66 | `delete_scene` | `scene_count == 0` | prints `"No scenes available\n"` | [x] |
| 67 | `delete_scene` | `scanf` != 1 | `"Invalid input\n"` + `getchar` drain | [x] |
| 68 | `delete_scene` | index out of range (`-1`, `scene_count`) | prints `"Invalid scene index\n"` | [x] |
| 69 | `main` | `fgets` returns `NULL` (EOF / empty stdin) | leaves the loop, cleans up, returns `0` | [x] |
| 70 | `main` | `sscanf(input,"%d")` != 1 (`"abc\n"`, `"\n"`, `"   \n"`, `"x1\n"`) | prints `"Invalid input\n"`, continues the loop | [x] |
| 71 | `main` | `choice` not in `1..=12` (`0`, `13`, `-5`, `99`, `2147483648` → clamped) | prints `"Invalid choice\n"` | [x] |

## Generic FFI boundary cases (not a single explicit check in the C source, but
## every C API has them)

| # | condition | expected C result | ok |
|---|-----------|-------------------|----|
| 72 | out-of-range "enum" values across the FFI boundary: `shape_get`/`shape_type_name` called with `INT_MIN`, `-1`, `10`, `11`, `INT_MAX` (C enums accept any `int`) | `NULL` / `"Unknown"`, no crash | [x] |
| 73 | `NULL` for every pointer parameter of every exported function | see rows 6, 13-16, 18, 21-24, 27-31, 38 | [x] |
| 74 | zero-length string arguments: `scene_create("")`, `scene_save(s,"")`, `scene_load("")` | empty name / `fopen` failure paths | [x] |
| 75 | oversized string arguments: 200-byte scene name, 300-byte filename | truncation at 63 / `fopen` failure | [x] |
| 76 | one step past the documented range: `scene_remove_shape(s, count)`, `scene_add_shape` on a 50-shape scene, 11th scene, 51st shape | `-1` / `"Error: Scene is full"` / `"Error: Maximum scenes reached"` | [x] |
| 77 | `shape_manager_cleanup` called twice / without `init` | `free(NULL)` — no-op, `shape_get` still `NULL` | [x] |
| 78 | `shape_manager_init` called twice (re-allocates all singletons, old ones leak) | new pointer identities; `shape_equals` against a pre-init pointer is `0` | [x] |

| 79 | `scene_save` | `fopen(…, "w")` fails with `EACCES` — an existing file with mode `0444` | `fprintf(stderr,"Error: Could not open file '%s' for writing\n")`, returns `-1`, the file is left untouched | [x] |
| 80 | `scene_load` | `fopen(…, "r")` fails with `EACCES` — an existing file with mode `0000` | `fprintf(stderr,"Error: Could not open file '%s' for reading\n")`, returns `NULL` | [x] |
| 81 | `scene_save` / `scene_load` | 300-byte file name component → `ENAMETOOLONG` | the same two messages, `-1` / `NULL` | [x] |

### Note on the allocator-failure rows (1, 10, 41)

They are reached with a surgical `malloc` interposer, `tests/support/failmalloc.c`
(test scaffolding outside `c_src`), which is `LD_PRELOAD`ed into the harness child
and fails allocations of one exact size only:

* `FAILMALLOC_SIZE=2444` (`sizeof(shape_t)`) → `shape_manager_init`
  prints `Error: Failed to allocate shape` on stderr and calls `exit(1)`;
* `FAILMALLOC_SIZE=472` (`sizeof(scene_t)`) → `scene_create` returns `NULL`,
  so `create_new_scene` prints `Error creating scene` and `scene_load` returns
  `NULL` silently.

`FAILMALLOC_AFTER` selects which of those allocations fails, so the failure can
be placed at the first, a middle and the last iteration of
`shape_manager_init`'s loop.  A disarmed run (interposer loaded, no size
configured) is compared as well, to prove the scaffolding itself changes nothing.

### Known limitation of the `driver` *executable*

The safe/idiomatic translation that `src/main.rs` is built from cannot observe an
allocation failure at all: Rust's global allocator aborts the process instead of
returning `NULL`.  Rows 1, 10 and 41 are therefore verified against the C ABI
library surface (`src/capi/`, which uses `malloc` exactly like the C code), not
against the executable.  Every other row is verified for both
(`tests/binary_diff.rs` runs the two executables against each other).

## Coverage summary

| test binary | cases | rows |
|-------------|-------|------|
| `tests/errors_lib.rs` | 25 differential cases, each covering several rows and many inputs | 1-38, 72-81 |
| `tests/errors_app.rs` | 126 differential cases (each its own pair of child processes) | 1, 10, 39-78 |
| `tests/binary_diff.rs` | 179 executable level scenarios (51 curated + 128 randomised) | the same error paths as seen by a user of the program |

All rows are `[x]`: every one has a differential test that constructs the exact
condition, runs **both** shared objects through their exported symbols, and
compares the return value/sentinel, `stdout`, `stderr`, the created files and the
exit status.
