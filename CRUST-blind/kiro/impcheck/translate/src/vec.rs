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
        self.vec_reserve(0);
    }
    pub fn vec_reserve(&mut self, new_cap: u64) {
        if new_cap > self.capacity {
            let new_len = new_cap as usize * std::mem::size_of::<i32>();
            let mut v = self.data.to_vec();
            v.resize(new_len, 0);
            self.data = Box::leak(v.into_boxed_slice());
            self.capacity = new_cap;
        }
        if self.size > new_cap {
            self.size = new_cap;
        }
    }
    pub fn vec_push(&mut self, elem: i32) {
        if self.size == self.capacity {
            let new_cap = (self.capacity as f64 * 1.3) as u64;
            let new_cap = if new_cap < self.capacity + 1 { self.capacity + 1 } else { new_cap };
            self.vec_reserve(new_cap);
        }
        let offset = self.size as usize * std::mem::size_of::<i32>();
        let bytes = elem.to_ne_bytes();
        // Safety: we need to write into the slice; use unsafe to get mutable access
        let ptr = self.data.as_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(offset), 4);
        }
        self.size += 1;
    }

    // Helper to read an i32 at a given index
    pub fn get_int(&self, idx: u64) -> i32 {
        let offset = idx as usize * std::mem::size_of::<i32>();
        let bytes: [u8; 4] = self.data[offset..offset+4].try_into().unwrap();
        i32::from_ne_bytes(bytes)
    }

    pub fn as_int_slice(&self) -> &[i32] {
        let len = self.size as usize;
        if len == 0 {
            return &[];
        }
        let ptr = self.data.as_ptr() as *const i32;
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }

    pub fn as_int_slice_mut(&mut self) -> &mut [i32] {
        let len = self.size as usize;
        if len == 0 {
            return &mut [];
        }
        let ptr = self.data.as_ptr() as *mut i32;
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }

    pub fn new(capacity: u64) -> Self {
        let byte_len = capacity as usize * std::mem::size_of::<i32>();
        let v = vec![0u8; byte_len];
        IntVec {
            capacity,
            size: 0,
            data: Box::leak(v.into_boxed_slice()),
        }
    }
}
