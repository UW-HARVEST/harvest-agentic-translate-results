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
        // In this thin wrapper, reserve is a no-op since we can't reallocate a borrowed slice.
        // Real vector functionality is handled internally by each module.
        if self.size > new_size {
            self.size = new_size;
        }
    }
    pub fn vec_push(&mut self, _elem: i32) {
        // This thin wrapper cannot actually push to a borrowed slice.
        // Real vector functionality is handled internally by each module.
        self.size += 1;
    }
}
