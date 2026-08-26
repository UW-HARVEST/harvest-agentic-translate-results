# Error Surface

The rows below come from rejection branches and boundary checks in
`c_src/src/shape.c` and `c_src/src/scene.c`. Allocation-failure rows are part of
the C surface even though deterministic fault injection is not exposed by the
API.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `shape_manager_init` | Any `malloc(sizeof(shape_t))` returns `NULL`. | Print `Error: Failed to allocate shape\n` to stderr and terminate with `exit(1)`. |
| [x] 2 | `shape_get` | `type < 0`. | Return `NULL`. |
| [x] 3 | `shape_get` | `type >= SHAPE_COUNT` (`10`). | Return `NULL`. |
| [x] 4 | `shape_print` | `shape == NULL`. | Print `(null shape)\n` and return. |
| [x] 5 | `shape_type_name` | `type` is not one of `0..=9`. | Return pointer to `"Unknown"`. |
| [x] 6 | `scene_create` | `malloc(sizeof(scene_t))` returns `NULL`. | Return `NULL`. |
| [x] 7 | `scene_destroy` | `scene == NULL`. | No-op and return. |
| [x] 8 | `scene_add_shape` | `scene == NULL`. | Return `-1`. |
| [x] 9 | `scene_add_shape` | `shape == NULL`. | Return `-1`. |
| [x] 10 | `scene_add_shape` | `scene->shape_count >= MAX_SHAPES_IN_SCENE` (`50`). | Print `Error: Scene is full\n` to stderr and return `-1`. |
| [x] 11 | `scene_remove_shape` | `scene == NULL`. | Return `-1`. |
| [x] 12 | `scene_remove_shape` | `index < 0`. | Return `-1`. |
| [x] 13 | `scene_remove_shape` | `index >= scene->shape_count`. | Return `-1`. |
| [x] 14 | `scene_print` | `scene == NULL`. | Print `(null scene)\n` and return. |
| [x] 15 | `scene_equals` | `s1 == NULL`. | Return `0`. |
| [x] 16 | `scene_equals` | `s2 == NULL`. | Return `0`. |
| [x] 17 | `scene_equals` | `s1->shape_count != s2->shape_count`. | Return `0`. |
| [x] 18 | `scene_equals` | A shape in `s1` has no unmatched pointer-identical shape in `s2`. | Return `0`. |
| [x] 19 | `scene_save` | `scene == NULL`. | Return `-1`. |
| [x] 20 | `scene_save` | `filename == NULL`. | Return `-1`. |
| [x] 21 | `scene_save` | `fopen(filename, "w") == NULL`. | Print the open-for-writing error to stderr and return `-1`. |
| [x] 22 | `scene_load` | `filename == NULL`. | Return `NULL`. |
| [x] 23 | `scene_load` | `fopen(filename, "r") == NULL`. | Print the open-for-reading error to stderr and return `NULL`. |
| [x] 24 | `scene_load` | Initial `fgets(name, 64, file) == NULL` (empty/unreadable file). | Close the file and return `NULL`. |
| [x] 25 | `scene_load` | Internal `scene_create(name)` allocation fails. | Close the file and return `NULL`. |
| [x] 26 | `scene_load` | `fscanf(file, "%d\n", &shape_count) != 1`. | Destroy the new scene, close the file, and return `NULL`. |
| [x] 27 | `scene_load` | Any of `shape_count` type reads has `fscanf(file, "%d\n", &type) != 1`. | Destroy the new scene, close the file, and return `NULL`. |
| [x] 28 | `scene_list_shapes` | `scene == NULL`. | Print `(null scene)\n` and return. |

Constants and representational boundaries mechanically found in the public
headers:

| constant | value | affected storage/check |
|----------|-------|------------------------|
| `MAX_SHAPE_WIDTH` | 80 | `shape_t.art` row width |
| `MAX_SHAPE_HEIGHT` | 30 | `shape_t.art` row count |
| `MAX_SHAPE_NAME` | 32 | `shape_t.name` |
| `SHAPE_COUNT` | 10 | enum validity and singleton array |
| `MAX_SHAPES_IN_SCENE` | 50 | scene capacity and equality match array |
| `MAX_SCENE_NAME` | 64 | scene name truncation/read width |
