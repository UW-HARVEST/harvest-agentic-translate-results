# Dynamic Symbol Surface

Reference library:

```text
c_src/build/libdriver_c.so
```

The library is built from the two implementation files corresponding to the
public headers (`scene.c` and `shape.c`). The interactive `main.c` consumer is
not part of the public library.

Extraction command:

```sh
nm -D --defined-only c_src/build/libdriver_c.so |
  awk '$2 ~ /^[TWDBR]$/ {print $3}' | sort
```

The disposition column records the initial Phase A diagnosis made before the
export layer was implemented. The Rust export column records the final state.

| C symbol | Rust export | Initial Phase A disposition |
|----------|-------------|---------------------|
| `scene_add_shape` | exported | Rust behavior existed only as a non-ABI `Scene` method; required the real C ABI implementation/export. |
| `scene_create` | exported | Rust behavior existed only as `Scene::new`; required the real C ABI implementation/export. |
| `scene_destroy` | exported | Rust relied on ownership drop; required the real C ABI implementation/export. |
| `scene_equals` | exported | Rust behavior existed only as a non-ABI function; required the real C ABI implementation/export. |
| `scene_list_shapes` | exported | Rust behavior existed only as a non-ABI function; required the real C ABI implementation/export. |
| `scene_load` | exported | Rust behavior existed only as a non-ABI function; required the real C ABI implementation/export. |
| `scene_print` | exported | Rust behavior existed only as a non-ABI function; required the real C ABI implementation/export. |
| `scene_remove_shape` | exported | Rust behavior existed only as a non-ABI `Scene` method; required the real C ABI implementation/export. |
| `scene_save` | exported | Rust behavior existed only as a non-ABI function; required the real C ABI implementation/export. |
| `shape_equals` | exported | Rust behavior existed only through internal pointer/type comparisons; required the real C ABI implementation/export. |
| `shape_get` | exported | Rust behavior existed only as a non-ABI `ShapeManager` method; required the real C ABI implementation/export. |
| `shape_manager_cleanup` | exported | Rust behavior existed only through ownership drop; required the real C ABI implementation/export. |
| `shape_manager_init` | exported | Rust behavior existed only as `ShapeManager::new`; required the real C ABI implementation/export. |
| `shape_print` | exported | Rust behavior existed only as a non-ABI function; required the real C ABI implementation/export. |
| `shape_type_name` | exported | Rust behavior existed only as a non-ABI function; required the real C ABI implementation/export. |

The initial Rust crate is binary-only and therefore has no Rust `.so` or C ABI
surface. No C module is absent from the translation; the missing work is the
ABI-compatible data representation and export layer.

Completion:

- [x] Every C symbol above is exported by the Rust `.so`.
- [x] Defined-symbol diff is empty.
- [x] Rust has no undefined non-libc project symbols.
