//! Translation of `generic_containers.h`.
//!
//! The C header uses token-pasting macros (`DECLARE_ARRAY`/`DEFINE_ARRAY`,
//! `DECLARE_LIST`/`DEFINE_LIST`) to stamp out a dynamic array and a singly
//! linked list for each element type. Rust generics express the same thing
//! once, so `Array<T>` stands in for every `array_TYPE_t` and `List<T>` for
//! every `list_TYPE_t`.
//!
//! `ARRAY_FOREACH` / `LIST_FOREACH` become plain `for` loops over `iter()`,
//! which visit elements in the same order (index 0..size for the array,
//! head->tail for the list).

/// Equivalent of `array_TYPE_t`.
///
/// The C version tracks `data`/`size`/`capacity` by hand; `Vec` tracks exactly
/// the same three things. `capacity` is never observable in the program output,
/// so growth policy differences are irrelevant, but `create` still honours the
/// C default of 16 for a zero initial capacity.
pub struct Array<T> {
    data: Vec<T>,
}

impl<T> Array<T> {
    /// `array_TYPE_create(initial_capacity)`
    pub fn create(initial_capacity: usize) -> Array<T> {
        let capacity = if initial_capacity > 0 {
            initial_capacity
        } else {
            16
        };
        Array {
            data: Vec::with_capacity(capacity),
        }
    }

    /// `array_TYPE_push(arr, value)`
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// `arr->size` / `array_TYPE_size(arr)`
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// `arr->data[index]` / `array_TYPE_get(arr, index)`
    #[allow(dead_code)]
    pub fn get(&self, index: usize) -> &T {
        &self.data[index]
    }

    /// `ARRAY_FOREACH(TYPE, var, arr)`
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// `array_TYPE_clear(arr)`
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// `array_TYPE_destroy(arr)`; Rust frees on drop, this is the explicit form.
    pub fn destroy(self) {}
}

/// Equivalent of `list_TYPE_t`, a singly linked list with head/tail/size.
///
/// The C list is only ever appended to and walked front-to-back in this
/// program. A `Vec` reproduces that observable behaviour (and `prepend`'s
/// insert-at-front semantics) without raw pointers.
pub struct List<T> {
    data: Vec<T>,
}

impl<T> List<T> {
    /// `list_TYPE_create()`
    pub fn create() -> List<T> {
        List { data: Vec::new() }
    }

    /// `list_TYPE_append(list, value)` - link onto the tail.
    pub fn append(&mut self, value: T) {
        self.data.push(value);
    }

    /// `list_TYPE_prepend(list, value)` - link onto the head.
    #[allow(dead_code)]
    pub fn prepend(&mut self, value: T) {
        self.data.insert(0, value);
    }

    /// `list->size` / `list_TYPE_size(list)`
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// `LIST_FOREACH(TYPE, var, list)` - head to tail.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// `list_TYPE_clear(list)`
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// `list_TYPE_destroy(list)`; Rust frees on drop, this is the explicit form.
    pub fn destroy(self) {}
}
