// IntVec - a typed vector of i32 elements stored as raw byte buffer.
// The data field is &[u8] to mimic C's untyped buffer; element count is
// tracked in `size`, capacity in `capacity`.
pub struct IntVec<'a> {
    pub capacity: u64,
    pub size: u64,
    pub data: &'a [u8],
}

fn make_buffer<'a>(capacity: u64) -> &'a [u8] {
    let v: Vec<u8> = vec![0u8; (capacity as usize) * std::mem::size_of::<i32>()];
    let leaked: &'static [u8] = Box::leak(v.into_boxed_slice());
    leaked
}

impl IntVec<'_> {
    pub fn vec_free(&mut self) {
        // Drop the leaked memory by replacing with an empty buffer.
        // We cannot reclaim leaked memory but we reset the metadata.
        self.size = 0;
        self.capacity = 0;
    }

    pub fn vec_clear(&mut self) {
        self.vec_reserve(0);
    }

    pub fn vec_reserve(&mut self, new_cap: u64) {
        if new_cap > self.capacity {
            // Allocate a fresh buffer of new_cap*sizeof(int) and copy data.
            let new_buf_vec: Vec<u8> = {
                let mut v = vec![0u8; (new_cap as usize) * std::mem::size_of::<i32>()];
                let copy_len =
                    (self.size as usize).min(self.capacity as usize) * std::mem::size_of::<i32>();
                if copy_len > 0 && self.data.len() >= copy_len {
                    v[..copy_len].copy_from_slice(&self.data[..copy_len]);
                }
                v
            };
            let leaked: &'static [u8] = Box::leak(new_buf_vec.into_boxed_slice());
            self.data = leaked;
            self.capacity = new_cap;
        }
        if self.size > new_cap {
            self.size = new_cap;
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
        let idx = self.size as usize;
        let bytes = elem.to_ne_bytes();
        // Need to write into data slice. Since data is &[u8], we use raw write.
        let ptr = self.data.as_ptr() as *mut u8;
        unsafe {
            for (i, &b) in bytes.iter().enumerate() {
                std::ptr::write(ptr.add(idx * 4 + i), b);
            }
        }
        self.size += 1;
    }
}

impl<'a> IntVec<'a> {
    pub fn vec_init(capacity: u64) -> Self {
        Self {
            capacity,
            size: 0,
            data: make_buffer(capacity),
        }
    }
}
