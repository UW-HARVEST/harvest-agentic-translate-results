pub struct IntVec<'a> {
    pub capacity: u64,
    pub size: u64,
    pub data: &'a [u8],
}
impl IntVec<'_> {
    pub fn vec_free(&mut self) {
        self.size = 0;
        self.capacity = 0;
    }
    pub fn vec_clear(&mut self) {
        self.size = 0;
    }
    pub fn vec_reserve(&mut self, new_size: u64) {
        if new_size > self.capacity {
            self.capacity = new_size;
        }
        if self.size > new_size {
            self.size = new_size;
        }
    }
    pub fn vec_push(&mut self, _elem: i32) {
        if self.size == self.capacity {
            let mut new_cap = (self.capacity as f64 * 1.3) as u64;
            if new_cap < self.capacity + 1 {
                new_cap = self.capacity + 1;
            }
            self.vec_reserve(new_cap);
        }
        self.size += 1;
    }
}
