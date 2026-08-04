/// Reallocates a vector of bytes to a new size.
/// In C, this was 'void* xrealloc(void* ptr, size_t size)'.
pub fn xrealloc(mut data: Vec<u8>, new_size: usize) -> Vec<u8> {
    data.resize(new_size, 0);
    data
}
/// Allocates a new vector of bytes of the specified size.
/// In C, this was 'void* xmalloc(size_t size)'.
pub fn xmalloc(size: usize) -> Vec<u8> {
    vec![0u8; size]
}
