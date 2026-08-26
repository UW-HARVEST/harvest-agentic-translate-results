/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
//! generic_containers.rs
//!
//! Safe-Rust equivalent of the `DECLARE_ARRAY` / `DEFINE_ARRAY` and
//! `DECLARE_LIST` / `DEFINE_LIST` macro families from `generic_containers.h`.
//! The C code uses one monomorphized copy per element type; Rust generics
//! give the same observable behavior with a single implementation.

#![allow(dead_code)]

// ============================================================================
// GENERIC DYNAMIC ARRAY
// ============================================================================

/// Equivalent of `array_TYPE_t`.
pub struct Array<T> {
    data: Vec<T>,
    capacity: usize,
}

impl<T: Copy> Array<T> {
    /// `array_TYPE_create(initial_capacity)`
    pub fn create(initial_capacity: usize) -> Array<T> {
        let capacity = if initial_capacity > 0 {
            initial_capacity
        } else {
            16
        };
        Array {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// `array_TYPE_push(arr, value)`
    pub fn push(&mut self, value: T) -> i32 {
        if self.data.len() >= self.capacity {
            self.capacity *= 2;
            self.data.reserve(self.capacity - self.data.len());
        }
        self.data.push(value);
        0
    }

    /// `array_TYPE_get(arr, index)`
    pub fn get(&self, index: usize) -> T {
        self.data[index]
    }

    /// `arr->size`
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// `arr->capacity`
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// `array_TYPE_clear(arr)`
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Backing storage, mirroring direct `arr->data[i]` accesses in the C code.
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// `ARRAY_FOREACH(TYPE, var, arr)`
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }
}

// ============================================================================
// GENERIC LINKED LIST
// ============================================================================

/// Equivalent of `list_TYPE_t`. The C version is a singly linked list with
/// head/tail pointers; only append/prepend/size/clear and forward iteration
/// are ever observed, so a growable vector reproduces it exactly.
pub struct List<T> {
    data: Vec<T>,
}

impl<T: Copy> List<T> {
    /// `list_TYPE_create()`
    pub fn create() -> List<T> {
        List { data: Vec::new() }
    }

    /// `list_TYPE_append(list, value)`
    pub fn append(&mut self, value: T) -> i32 {
        self.data.push(value);
        0
    }

    /// `list_TYPE_prepend(list, value)`
    pub fn prepend(&mut self, value: T) -> i32 {
        self.data.insert(0, value);
        0
    }

    /// `list->size`
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// `list_TYPE_clear(list)`
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// `LIST_FOREACH(TYPE, var, list)`
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }
}
