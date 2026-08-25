# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
configuration options or preprocessor definitions. Consequently there is one
valid build-time combination:

| # | Cargo invocation | C configuration | [x] |
|---|------------------|-----------------|-----|
| 1 | `--no-default-features` (empty feature set; also the default) | Default CMake configuration with PIC requested | [x] |

## Runtime and Input Configurations

There are no public runtime option setters. The axes below are the enum value,
manager lifecycle state, pointer identity/nullability, scene name length,
scene element count/order/duplication, removal position, and serialized file
shape that the C implementation branches on.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `shape_manager_init` | First initialization allocates and initializes all 10 singleton shapes. | [x] |
| 2 | `shape_manager_cleanup` | Cleanup after initialization frees all 10 singleton shapes and nulls every slot. | [x] |
| 3 | `shape_manager_cleanup` | Cleanup while every singleton slot is already null. | [x] |
| 4 | `shape_get` | Initialized manager; each valid type `0..=9` returns its corresponding singleton. | [x] |
| 5 | `shape_get` | After cleanup; each valid type `0..=9` returns the null slot. | [x] |
| 6 | `shape_type_name` | Each valid enum value `0..=9` selects its distinct switch arm/name. | [x] |
| 7 | `shape_print` | Each valid singleton type `0..=9`, covering every distinct width/height/art payload. | [x] |
| 8 | `shape_equals` | Two pointers to the same valid singleton. | [x] |
| 9 | `shape_equals` | Pointers to two different valid singletons. | [x] |
| 10 | `scene_create`, `scene_destroy` | Null name selects the literal `Untitled Scene`; destroy the result. | [x] |
| 11 | `scene_create`, `scene_destroy` | Empty name. | [x] |
| 12 | `scene_create`, `scene_destroy` | Nonempty name of length `1..62`. | [x] |
| 13 | `scene_create`, `scene_destroy` | Name exactly 63 bytes. | [x] |
| 14 | `scene_create`, `scene_destroy` | Name longer than 63 bytes is truncated and NUL-terminated. | [x] |
| 15 | `scene_add_shape` | Empty scene plus each valid shape type; count changes `0 -> 1`. | [x] |
| 16 | `scene_add_shape` | Partially populated scene at counts `1..48`; append preserves prior order. | [x] |
| 17 | `scene_add_shape` | Count 49; append reaches the capacity boundary 50. | [x] |
| 18 | `scene_remove_shape` | One-element scene; remove index 0 and become empty. | [x] |
| 19 | `scene_remove_shape` | Multi-element scene; remove first element and shift all following pointers. | [x] |
| 20 | `scene_remove_shape` | Multi-element scene; remove a middle element and shift the suffix. | [x] |
| 21 | `scene_remove_shape` | Multi-element scene; remove last element without shifting. | [x] |
| 22 | `scene_print` | Valid empty scene. | [x] |
| 23 | `scene_print` | Valid one-shape scene, randomized over all shape types and names. | [x] |
| 24 | `scene_print` | Valid many-shape scene with repeated and distinct types. | [x] |
| 25 | `scene_list_shapes` | Valid empty scene. | [x] |
| 26 | `scene_list_shapes` | Valid one-shape scene, randomized over all shape types and names. | [x] |
| 27 | `scene_list_shapes` | Valid many-shape scene with repeated and distinct types. | [x] |
| 28 | `scene_equals` | Two empty scenes (names are ignored). | [x] |
| 29 | `scene_equals` | Same nonempty scene pointer compared with itself. | [x] |
| 30 | `scene_equals` | Equal one-element scenes containing the same singleton. | [x] |
| 31 | `scene_equals` | Equal many-element scenes in the same order, including duplicates. | [x] |
| 32 | `scene_equals` | Equal many-element scenes in a different order (1:1 multiset correspondence). | [x] |
| 33 | `scene_save` | Writable path; empty scene and name lengths 0, short, and 63. | [x] |
| 34 | `scene_save` | Writable path; one shape, randomized over all valid types. | [x] |
| 35 | `scene_save` | Writable path; many shapes with duplicates and capacity-boundary count 50. | [x] |
| 36 | `scene_load` | Valid file with empty/short scene name and shape count 0. | [x] |
| 37 | `scene_load` | Valid file with a 63-byte name and one valid type. | [x] |
| 38 | `scene_load` | First name line exceeds 63 bytes; the next scan begins in the remaining name bytes. | [x] |
| 39 | `scene_load` | Valid file with many valid types, including duplicates and every enum value. | [x] |
| 40 | `scene_load` | Negative shape count; loop executes zero times and returns an empty scene. | [x] |
| 41 | `scene_load` | Positive count containing out-of-range negative and `>=10` type values; invalid types are skipped. | [x] |
| 42 | `scene_load` | Count above 50 with valid types; `scene_add_shape` failures after capacity are ignored and result stays at 50. | [x] |
| 43 | `scene_save`, `scene_load`, `scene_equals` | End-to-end round trip for randomized names and randomized shape multisets from empty through capacity 50. | [x] |
| 44 | `scene_add_shape`, `scene_remove_shape`, `scene_equals` | Randomized operation sequences preserve C pointer order/count and equality behavior. | [x] |
| 45 | All 15 public entry points | Repeated manager initialize/use/cleanup lifecycle with scenes destroyed before singleton cleanup. | [x] |
