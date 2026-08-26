use std::collections::LinkedList;

pub const DEFAULT_ARRAY_CAPACITY: usize = 16;

#[derive(Clone)]
pub struct GenericArray<T> {
    data: Vec<T>,
}

impl<T> GenericArray<T> {
    pub fn create(initial_capacity: usize) -> Self {
        let capacity = if initial_capacity > 0 {
            initial_capacity
        } else {
            DEFAULT_ARRAY_CAPACITY
        };
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn destroy(self) {}

    pub fn push(&mut self, value: T) -> i32 {
        self.data.push(value);
        0
    }

    pub fn get(&self, index: usize) -> T
    where
        T: Clone,
    {
        self.data[index].clone()
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data
    }
}

#[derive(Clone)]
pub struct GenericList<T> {
    data: LinkedList<T>,
}

impl<T> GenericList<T> {
    pub fn create() -> Self {
        Self {
            data: LinkedList::new(),
        }
    }

    pub fn destroy(self) {}

    pub fn append(&mut self, value: T) -> i32 {
        self.data.push_back(value);
        0
    }

    pub fn prepend(&mut self, value: T) -> i32 {
        self.data.push_front(value);
        0
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

pub type ArrayIntT = GenericArray<i32>;
pub type ArrayDoubleT = GenericArray<f64>;
pub type ArrayItemTT = GenericArray<crate::inventory::ItemT>;
pub type ArrayOrderTT = GenericArray<crate::inventory::OrderT>;

pub type ListIntT = GenericList<i32>;
pub type ListDoubleT = GenericList<f64>;
pub type ListItemTT = GenericList<crate::inventory::ItemT>;
pub type ListOrderTT = GenericList<crate::inventory::OrderT>;

pub fn array_int_create(initial_capacity: usize) -> ArrayIntT {
    GenericArray::create(initial_capacity)
}

pub fn array_int_destroy(arr: ArrayIntT) {
    arr.destroy();
}

pub fn array_int_push(arr: &mut ArrayIntT, value: i32) -> i32 {
    arr.push(value)
}

pub fn array_int_get(arr: &ArrayIntT, index: usize) -> i32 {
    arr.get(index)
}

pub fn array_int_size(arr: &ArrayIntT) -> usize {
    arr.size()
}

pub fn array_int_clear(arr: &mut ArrayIntT) {
    arr.clear();
}

pub fn array_double_create(initial_capacity: usize) -> ArrayDoubleT {
    GenericArray::create(initial_capacity)
}

pub fn array_double_destroy(arr: ArrayDoubleT) {
    arr.destroy();
}

pub fn array_double_push(arr: &mut ArrayDoubleT, value: f64) -> i32 {
    arr.push(value)
}

pub fn array_double_get(arr: &ArrayDoubleT, index: usize) -> f64 {
    arr.get(index)
}

pub fn array_double_size(arr: &ArrayDoubleT) -> usize {
    arr.size()
}

pub fn array_double_clear(arr: &mut ArrayDoubleT) {
    arr.clear();
}

pub fn array_item_t_create(initial_capacity: usize) -> ArrayItemTT {
    GenericArray::create(initial_capacity)
}

pub fn array_item_t_destroy(arr: ArrayItemTT) {
    arr.destroy();
}

pub fn array_item_t_push(arr: &mut ArrayItemTT, value: crate::inventory::ItemT) -> i32 {
    arr.push(value)
}

pub fn array_item_t_get(arr: &ArrayItemTT, index: usize) -> crate::inventory::ItemT {
    arr.get(index)
}

pub fn array_item_t_size(arr: &ArrayItemTT) -> usize {
    arr.size()
}

pub fn array_item_t_clear(arr: &mut ArrayItemTT) {
    arr.clear();
}

pub fn array_order_t_create(initial_capacity: usize) -> ArrayOrderTT {
    GenericArray::create(initial_capacity)
}

pub fn array_order_t_destroy(arr: ArrayOrderTT) {
    arr.destroy();
}

pub fn array_order_t_push(arr: &mut ArrayOrderTT, value: crate::inventory::OrderT) -> i32 {
    arr.push(value)
}

pub fn array_order_t_get(arr: &ArrayOrderTT, index: usize) -> crate::inventory::OrderT {
    arr.get(index)
}

pub fn array_order_t_size(arr: &ArrayOrderTT) -> usize {
    arr.size()
}

pub fn array_order_t_clear(arr: &mut ArrayOrderTT) {
    arr.clear();
}

pub fn list_int_create() -> ListIntT {
    GenericList::create()
}

pub fn list_int_destroy(list: ListIntT) {
    list.destroy();
}

pub fn list_int_append(list: &mut ListIntT, value: i32) -> i32 {
    list.append(value)
}

pub fn list_int_prepend(list: &mut ListIntT, value: i32) -> i32 {
    list.prepend(value)
}

pub fn list_int_size(list: &ListIntT) -> usize {
    list.size()
}

pub fn list_int_clear(list: &mut ListIntT) {
    list.clear();
}

pub fn list_double_create() -> ListDoubleT {
    GenericList::create()
}

pub fn list_double_destroy(list: ListDoubleT) {
    list.destroy();
}

pub fn list_double_append(list: &mut ListDoubleT, value: f64) -> i32 {
    list.append(value)
}

pub fn list_double_prepend(list: &mut ListDoubleT, value: f64) -> i32 {
    list.prepend(value)
}

pub fn list_double_size(list: &ListDoubleT) -> usize {
    list.size()
}

pub fn list_double_clear(list: &mut ListDoubleT) {
    list.clear();
}

pub fn list_item_t_create() -> ListItemTT {
    GenericList::create()
}

pub fn list_item_t_destroy(list: ListItemTT) {
    list.destroy();
}

pub fn list_item_t_append(list: &mut ListItemTT, value: crate::inventory::ItemT) -> i32 {
    list.append(value)
}

pub fn list_item_t_prepend(list: &mut ListItemTT, value: crate::inventory::ItemT) -> i32 {
    list.prepend(value)
}

pub fn list_item_t_size(list: &ListItemTT) -> usize {
    list.size()
}

pub fn list_item_t_clear(list: &mut ListItemTT) {
    list.clear();
}

pub fn list_order_t_create() -> ListOrderTT {
    GenericList::create()
}

pub fn list_order_t_destroy(list: ListOrderTT) {
    list.destroy();
}

pub fn list_order_t_append(list: &mut ListOrderTT, value: crate::inventory::OrderT) -> i32 {
    list.append(value)
}

pub fn list_order_t_prepend(list: &mut ListOrderTT, value: crate::inventory::OrderT) -> i32 {
    list.prepend(value)
}

pub fn list_order_t_size(list: &ListOrderTT) -> usize {
    list.size()
}

pub fn list_order_t_clear(list: &mut ListOrderTT) {
    list.clear();
}
