pub struct IntVec<'a> {
    pub capacity: u64,
    pub size: u64,
    pub data: &'a [u8],
}
impl IntVec<'_> {
    pub fn vec_free(&mut self) {
        self.size = 0;
        self.capacity = 0;
        self.data = &[];
    }
    pub fn vec_clear(&mut self) {
        self.vec_reserve(0);
    }
    pub fn vec_reserve(&mut self, new_size: u64) {
        if new_size > self.capacity {
            let new_byte_size = (new_size as usize).saturating_mul(4);
            let mut new_buf: Vec<u8> = vec![0u8; new_byte_size];
            let copy_len = self.data.len().min(new_byte_size);
            new_buf[..copy_len].copy_from_slice(&self.data[..copy_len]);
            let leaked: &'static [u8] = Box::leak(new_buf.into_boxed_slice());
            self.data = leaked;
            self.capacity = new_size;
        }
        if self.size > new_size {
            self.size = new_size; // shrink
        }
    }
    pub fn vec_push(&mut self, elem: i32) {
        if self.size == self.capacity {
            let mut new_cap = ((self.capacity as f64) * 1.3) as u64;
            if new_cap < self.capacity + 1 {
                new_cap = self.capacity + 1;
            }
            self.vec_reserve(new_cap);
        }
        // Build a new buffer with the new element appended (safe but copies)
        let total_bytes = (self.capacity as usize).saturating_mul(4);
        let mut new_buf: Vec<u8> = vec![0u8; total_bytes];
        let copy_len = self.data.len().min(total_bytes);
        new_buf[..copy_len].copy_from_slice(&self.data[..copy_len]);
        let offset = (self.size as usize) * 4;
        if offset + 4 <= new_buf.len() {
            let bytes = elem.to_ne_bytes();
            new_buf[offset..offset + 4].copy_from_slice(&bytes);
        }
        let leaked: &'static [u8] = Box::leak(new_buf.into_boxed_slice());
        self.data = leaked;
        self.size += 1;
    }
}
