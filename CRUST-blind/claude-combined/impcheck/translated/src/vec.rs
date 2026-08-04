pub struct IntVec<'a> {
    pub capacity: u64,
    pub size: u64,
    pub data: &'a [u8],
}

impl<'a> IntVec<'a> {
    pub fn vec_init(capacity: u64) -> Self {
        let n = capacity as usize;
        let v: Vec<i32> = vec![0; n];
        // Convert Vec<i32> to a leaked &'static [u8]
        let boxed = v.into_boxed_slice();
        let raw = Box::into_raw(boxed); // *mut [i32]
        let len_bytes = n * std::mem::size_of::<i32>();
        let ptr = raw as *mut i32 as *const u8;
        let slice: &'static [u8] = unsafe { std::slice::from_raw_parts(ptr, len_bytes) };
        IntVec {
            capacity,
            size: 0,
            data: slice,
        }
    }

    fn data_as_mut_i32(&mut self) -> &mut [i32] {
        let cap = self.capacity as usize;
        let ptr = self.data.as_ptr() as *mut i32;
        unsafe { std::slice::from_raw_parts_mut(ptr, cap) }
    }

    fn data_as_i32(&self) -> &[i32] {
        let cap = self.capacity as usize;
        let ptr = self.data.as_ptr() as *const i32;
        unsafe { std::slice::from_raw_parts(ptr, cap) }
    }

    pub fn vec_free(&mut self) {
        if self.capacity > 0 {
            let cap = self.capacity as usize;
            let ptr = self.data.as_ptr() as *mut i32;
            // Reconstruct the Box and drop it
            unsafe {
                let slice = std::slice::from_raw_parts_mut(ptr, cap);
                let _: Box<[i32]> = Box::from_raw(slice as *mut [i32]);
            }
        }
        self.size = 0;
        self.capacity = 0;
        self.data = &[];
    }

    pub fn vec_clear(&mut self) {
        // C vec_clear calls vec_reserve(vec, 0) which since 0 < capacity does
        // not realloc; then "shrink" branch sets size = min(size, new_cap) = 0.
        self.vec_reserve(0);
    }

    pub fn vec_reserve(&mut self, new_cap: u64) {
        if new_cap > self.capacity {
            let new_n = new_cap as usize;
            let mut new_v: Vec<i32> = vec![0; new_n];
            let old_n = self.capacity as usize;
            if old_n > 0 {
                let old = self.data_as_i32();
                let copy_n = std::cmp::min(old_n, new_n);
                for i in 0..copy_n {
                    new_v[i] = old[i];
                }
                // free old
                let ptr = self.data.as_ptr() as *mut i32;
                unsafe {
                    let slice = std::slice::from_raw_parts_mut(ptr, old_n);
                    let _: Box<[i32]> = Box::from_raw(slice as *mut [i32]);
                }
            }
            let boxed = new_v.into_boxed_slice();
            let raw = Box::into_raw(boxed);
            let len_bytes = new_n * std::mem::size_of::<i32>();
            let ptr = raw as *mut i32 as *const u8;
            let slice: &'static [u8] = unsafe { std::slice::from_raw_parts(ptr, len_bytes) };
            self.data = slice;
            self.capacity = new_cap;
        }
        if self.size > new_cap {
            self.size = new_cap;
        }
    }

    pub fn vec_push(&mut self, elem: i32) {
        if self.size == self.capacity {
            // C: u64 new_cap = vec->capacity*1.3;
            //    if (new_cap < vec->capacity+1) new_cap = vec->capacity+1;
            let mut new_cap = ((self.capacity as f64) * 1.3) as u64;
            if new_cap < self.capacity + 1 {
                new_cap = self.capacity + 1;
            }
            self.vec_reserve(new_cap);
        }
        let idx = self.size as usize;
        let data = self.data_as_mut_i32();
        data[idx] = elem;
        self.size += 1;
    }

    pub fn get(&self, idx: usize) -> i32 {
        self.data_as_i32()[idx]
    }
}
