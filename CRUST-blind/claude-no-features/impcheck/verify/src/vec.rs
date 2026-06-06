// Implementation of an int vector. The struct's `data` field is declared as
// a borrowed slice `&'a [u8]`, but the underlying C code stores `int` values.
// We treat the byte slice as a raw byte buffer of `capacity * 4` bytes
// containing `size` little-endian/native-endian i32 values. To make growth
// possible, we leak boxed buffers and transmute their lifetime to `'a`.

pub struct IntVec<'a> {
    pub capacity: u64,
    pub size: u64,
    pub data: &'a [u8],
}

const ELEM_SIZE: usize = std::mem::size_of::<i32>();

impl<'a> IntVec<'a> {
    pub fn vec_init(capacity: u64) -> Self {
        let buf: Vec<u8> = vec![0u8; (capacity as usize) * ELEM_SIZE];
        let leaked: &'static [u8] = Box::leak(buf.into_boxed_slice());
        // SAFETY: extend lifetime to 'a; we manage the buffer manually,
        // freeing it ourselves when we replace it.
        let data: &'a [u8] = unsafe { std::mem::transmute(leaked) };
        IntVec {
            capacity,
            size: 0,
            data,
        }
    }

    pub fn vec_free(&mut self) {
        // Reclaim the leaked buffer. The buffer was leaked from a Box, so
        // we can reconstruct it as long as we still know its length.
        if !self.data.is_empty() {
            let ptr = self.data.as_ptr() as *mut u8;
            let len = self.data.len();
            // SAFETY: The buffer was leaked from a Box<[u8]> of this length.
            unsafe {
                let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len));
            }
        }
        // Replace data with an empty static slice
        self.data = unsafe { std::mem::transmute::<&'static [u8], &'a [u8]>(&[][..]) };
        self.size = 0;
        self.capacity = 0;
    }

    pub fn vec_clear(&mut self) {
        self.vec_reserve(0);
    }

    pub fn vec_reserve(&mut self, new_cap: u64) {
        if new_cap > self.capacity {
            // Allocate a new buffer of new_cap elements, copy old data over.
            let new_size_bytes = (new_cap as usize) * ELEM_SIZE;
            let mut new_buf: Vec<u8> = vec![0u8; new_size_bytes];
            let copy_bytes = self.data.len().min(new_size_bytes);
            new_buf[..copy_bytes].copy_from_slice(&self.data[..copy_bytes]);

            // Free old buffer if we had one (leaked previously)
            if !self.data.is_empty() {
                let ptr = self.data.as_ptr() as *mut u8;
                let len = self.data.len();
                unsafe {
                    let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len));
                }
            }

            let leaked: &'static [u8] = Box::leak(new_buf.into_boxed_slice());
            self.data = unsafe { std::mem::transmute::<&'static [u8], &'a [u8]>(leaked) };
            self.capacity = new_cap;
        }
        if self.size > new_cap {
            self.size = new_cap;
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
        // Write the element into the byte buffer at position `size`.
        let offset = (self.size as usize) * ELEM_SIZE;
        let bytes = elem.to_ne_bytes();
        // SAFETY: We know the underlying buffer was allocated with capacity*ELEM_SIZE bytes
        // and we have unique access via &mut self.
        unsafe {
            let ptr = self.data.as_ptr() as *mut u8;
            for i in 0..ELEM_SIZE {
                *ptr.add(offset + i) = bytes[i];
            }
        }
        self.size += 1;
    }
}
