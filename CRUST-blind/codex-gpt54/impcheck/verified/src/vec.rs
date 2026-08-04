use std::cell::RefCell;
use std::collections::HashMap;

pub struct IntVec<'a> {
    pub capacity: u64, 
    pub size: u64, 
    pub data: &'a [u8],
}

#[derive(Default)]
struct IntVecState {
    elems: Vec<i32>,
}

thread_local! {
    static INT_VECS: RefCell<HashMap<usize, IntVecState>> = RefCell::new(HashMap::new());
}

impl IntVec<'_> {
    fn id(&self) -> usize {
        self.data.as_ptr() as usize
    }

    pub fn new(capacity: u64) -> Self {
        let id_slice = Box::leak(vec![0_u8; 1].into_boxed_slice());
        let id = id_slice.as_ptr() as usize;
        INT_VECS.with(|store| {
            store.borrow_mut().insert(
                id,
                IntVecState {
                    elems: Vec::with_capacity(capacity as usize),
                },
            );
        });
        Self {
            capacity,
            size: 0,
            data: id_slice,
        }
    }

    pub fn vec_free(&mut self) {
        INT_VECS.with(|store| {
            store.borrow_mut().remove(&self.id());
        });
        self.capacity = 0;
        self.size = 0;
    }
    pub fn vec_clear(&mut self) {
        self.vec_reserve(0);
    }
    pub fn vec_reserve(&mut self, new_size: u64) {
        INT_VECS.with(|store| {
            if let Some(state) = store.borrow_mut().get_mut(&self.id()) {
                let new_cap = new_size as usize;
                if new_cap > state.elems.capacity() {
                    state.elems.reserve_exact(new_cap - state.elems.capacity());
                }
                if state.elems.len() > new_cap {
                    state.elems.truncate(new_cap);
                }
                if new_cap == 0 {
                    state.elems = Vec::new();
                }
                self.capacity = new_size;
                self.size = state.elems.len() as u64;
            }
        });
    }
    pub fn vec_push(&mut self, elem: i32) {
        INT_VECS.with(|store| {
            if let Some(state) = store.borrow_mut().get_mut(&self.id()) {
                if self.size == self.capacity {
                    let mut new_cap = (self.capacity as f64 * 1.3) as u64;
                    if new_cap < self.capacity + 1 {
                        new_cap = self.capacity + 1;
                    }
                    if new_cap as usize > state.elems.capacity() {
                        state
                            .elems
                            .reserve_exact(new_cap as usize - state.elems.capacity());
                    }
                    self.capacity = new_cap;
                }
                state.elems.push(elem);
                self.size = state.elems.len() as u64;
            }
        });
    }
}
