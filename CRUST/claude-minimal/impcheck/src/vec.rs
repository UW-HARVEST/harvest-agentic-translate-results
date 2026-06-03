pub struct IntVec {
    pub capacity: u64,
    pub size: u64,
    pub data: Vec<i32>,
}

impl IntVec {
    pub fn vec_init(capacity: u64) -> Self {
        let mut data = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            data.push(0);
        }
        IntVec {
            capacity,
            size: 0,
            data,
        }
    }

    pub fn vec_free(&mut self) {
        self.data.clear();
        self.size = 0;
        self.capacity = 0;
    }

    pub fn vec_clear(&mut self) {
        self.vec_reserve(0);
    }

    pub fn vec_reserve(&mut self, new_size: u64) {
        if new_size > self.capacity {
            self.data.resize(new_size as usize, 0);
            self.capacity = new_size;
        }
        if self.size > new_size {
            self.size = new_size;
        }
    }

    pub fn vec_push(&mut self, elem: i32) {
        if self.size == self.capacity {
            let mut new_cap = (self.capacity as f64 * 1.3) as u64;
            if new_cap < self.capacity + 1 {
                new_cap = self.capacity + 1;
            }
            self.vec_reserve(new_cap);
        }
        self.data[self.size as usize] = elem;
        self.size += 1;
    }
}
