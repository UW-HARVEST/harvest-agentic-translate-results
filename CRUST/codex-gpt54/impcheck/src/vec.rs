use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static REGISTRY: RefCell<HashMap<usize, Vec<i32>>> = RefCell::new(HashMap::new());
}

pub struct IntVec<'a> {
    pub capacity: u64,
    pub size: u64,
    pub data: &'a [u8],
}

impl IntVec<'_> {
    pub fn vec_free(&mut self) {
        if let Some(id) = self.id() {
            REGISTRY.with(|registry| {
                registry.borrow_mut().remove(&id);
            });
        }
        self.size = 0;
        self.capacity = 0;
    }

    pub fn vec_clear(&mut self) {
        self.vec_reserve(0);
    }

    pub fn vec_reserve(&mut self, new_size: u64) {
        let id = self.ensure_id();
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let values = registry.entry(id).or_default();
            if new_size as usize > values.capacity() {
                values.reserve(new_size as usize - values.capacity());
            }
            if values.len() > new_size as usize {
                values.truncate(new_size as usize);
            }
            self.capacity = values.capacity() as u64;
            self.size = values.len() as u64;
        });
    }

    pub fn vec_push(&mut self, elem: i32) {
        let id = self.ensure_id();
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let values = registry.entry(id).or_default();
            if values.len() == values.capacity() {
                let mut new_cap = ((values.capacity() as f64) * 1.3) as usize;
                if new_cap < values.capacity() + 1 {
                    new_cap = values.capacity() + 1;
                }
                values.reserve(new_cap - values.capacity());
            }
            values.push(elem);
            self.capacity = values.capacity() as u64;
            self.size = values.len() as u64;
        });
    }

    fn id(&self) -> Option<usize> {
        (!self.data.is_empty()).then_some(self.data.as_ptr() as usize)
    }

    fn ensure_id(&mut self) -> usize {
        if let Some(id) = self.id() {
            return id;
        }
        let token = Box::leak(vec![0u8].into_boxed_slice());
        self.data = token;
        token.as_ptr() as usize
    }
}
