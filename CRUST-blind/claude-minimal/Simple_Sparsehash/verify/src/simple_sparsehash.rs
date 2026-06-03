use std::mem;
use std::ptr;

/// The maximum size of each sparse_array_group.
pub const GROUP_SIZE: usize = 48;
/// The default size of the hash table. Used to init bucket_max.
pub const STARTING_SIZE: usize = 32;
/// The default 'should we resize' percentage, out of 100.
pub const RESIZE_PERCENT: usize = 80;
/// Number of bits per u32.
pub const BITCHUNK_SIZE: usize = std::mem::size_of::<u32>() * 8;
/// The minimum number of u32 entries required to hold GROUP_SIZE bits.
pub const BITMAP_SIZE: usize = (GROUP_SIZE - 1) / BITCHUNK_SIZE + 1;
/// Represents one stored key/value pair.
#[derive(Debug)]
pub struct SparseBucket {
    /// The key as an owned String.
    pub key: String,
    /// The length of the key.
    pub klen: usize,
    /// The value as a vector of bytes.
    pub val: Vec<u8>,
    /// The length of the value.
    pub vlen: usize,
    /// The hash of the key.
    pub hash: u64,
}
/// One group in a sparse array.
#[derive(Debug)]
pub struct SparseArrayGroup {
    /// The number of items currently in this group.
    pub count: u32,
    /// The maximum size of each element.
    pub elem_size: usize,
    /// The storage for the elements.
    pub group: Vec<u8>,
    /// A bitmap tracking which slots in `group` are occupied.
    pub bitmap: [u32; BITMAP_SIZE],
}
/// A sparse array consisting of one or more groups.
#[derive(Debug)]
pub struct SparseArray {
    /// The maximum number of items that can be stored.
    pub maximum: usize,
    /// The groups that hold the elements.
    pub groups: Vec<SparseArrayGroup>,
}
/// A sparse dictionary that maps keys to values.
#[derive(Debug)]
pub struct SparseDict {
    /// The current maximum number of buckets in the dictionary.
    pub bucket_max: usize,
    /// The number of buckets that are currently occupied.
    pub bucket_count: usize,
    /// An array of sparse arrays (buckets).
    pub buckets: Vec<SparseArray>,
}

// ---------- Internal helpers ----------

/// FNV-1a hash function.
fn hash_fnv1a(key: &[u8], klen: usize) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash = FNV_OFFSET_BIAS;
    let n = klen.min(key.len());
    for i in 0..n {
        hash ^= key[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Returns the index in the bitmap (in u32 chunks) for the given position.
fn charbit(position: u32) -> u32 {
    position >> 5
}

/// Returns the bit mask within a u32 chunk for the given position.
fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

/// Counts the number of set bits in `bitmap` strictly before `position`.
fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval = 0u32;
    let mut pos = position;
    let mut bitmap_iter = 0usize;
    while pos >= BITCHUNK_SIZE as u32 {
        retval = retval.wrapping_add(bitmap[bitmap_iter].count_ones());
        bitmap_iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }
    // pos is now < BITCHUNK_SIZE, so (1u32 << pos) is well-defined.
    let mask = (1u32 << pos).wrapping_sub(1);
    retval.wrapping_add((bitmap[bitmap_iter] & mask).count_ones())
}

/// Returns true if the slot at `position` is occupied.
fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    bitmap[charbit(position) as usize] & modbit(position) != 0
}

/// Marks the slot at `position` as occupied.
fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position) as usize] |= modbit(position);
}

/// Sets a value within a single sparse_array_group.
fn sparse_array_group_set(arr: &mut SparseArrayGroup, i: u32, val: &[u8], vlen: usize) -> i32 {
    if vlen > arr.elem_size {
        return 0;
    }
    let usize_size = mem::size_of::<usize>();
    let full_size = arr.elem_size + usize_size;
    let offset = position_to_offset(&arr.bitmap, i) as usize;

    if !is_position_occupied(&arr.bitmap, i) {
        let to_move_siz = (arr.count as usize - offset) * full_size;
        let new_total_size = (arr.count as usize + 1) * full_size;
        arr.group.resize(new_total_size, 0);
        if to_move_siz > 0 {
            let from_start = offset * full_size;
            let from_end = from_start + to_move_siz;
            arr.group
                .copy_within(from_start..from_end, from_start + full_size);
        }
        arr.count += 1;
        set_position(&mut arr.bitmap, i);
    }

    // Write the size in the first usize bytes of the slot.
    let dest_start = offset * full_size;
    let size_bytes = vlen.to_ne_bytes();
    arr.group[dest_start..dest_start + usize_size].copy_from_slice(&size_bytes);
    // Write the value bytes after the size.
    let val_start = dest_start + usize_size;
    arr.group[val_start..val_start + vlen].copy_from_slice(&val[..vlen]);
    1
}

/// Gets a value from within a single sparse_array_group.
fn sparse_array_group_get<'a>(
    arr: &'a SparseArrayGroup,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&arr.bitmap, i) {
        return None;
    }
    let usize_size = mem::size_of::<usize>();
    let full_size = arr.elem_size + usize_size;
    let offset = position_to_offset(&arr.bitmap, i) as usize;
    let item_start = offset * full_size;

    let mut size_bytes = [0u8; mem::size_of::<usize>()];
    size_bytes.copy_from_slice(&arr.group[item_start..item_start + usize_size]);
    let item_siz = usize::from_ne_bytes(size_bytes);

    if item_siz == 0 {
        return None;
    }

    if let Some(s) = outsize {
        *s = item_siz;
    }

    let val_start = item_start + usize_size;
    Some(&arr.group[val_start..val_start + item_siz])
}

// ---------- Sparse Array API ----------

/// Creates a new sparse array.
///
/// # Parameters
/// - `element_size`: Maximum size (in bytes) of each element.
/// - `maximum`: The maximum number of elements.
///
/// # Returns
/// An owned pointer (boxed) to a new `SparseArray` or `None` on failure.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    if maximum == 0 {
        return None;
    }
    let max_arr_size = (maximum as usize - 1) / GROUP_SIZE + 1;
    let mut groups = Vec::with_capacity(max_arr_size);
    for _ in 0..max_arr_size {
        groups.push(SparseArrayGroup {
            count: 0,
            elem_size: element_size,
            group: Vec::new(),
            bitmap: [0u32; BITMAP_SIZE],
        });
    }
    Some(Box::new(SparseArray {
        maximum: maximum as usize,
        groups,
    }))
}

/// Sets the element at index `i` to `val`.
///
/// # Parameters
/// - `arr`: The sparse array.
/// - `i`: The index at which to set the value.
/// - `val`: A slice of bytes holding the new element.
/// - `vlen`: The length of the value (in bytes).
///
/// # Returns
/// A nonzero integer on success and 0 on failure.
pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if (i as usize) > arr.maximum {
        return 0;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    let position = (i as u32) % (GROUP_SIZE as u32);
    sparse_array_group_set(&mut arr.groups[group_idx], position, val, vlen)
}

/// Retrieves the element at index `i`.
///
/// # Parameters
/// - `arr`: The sparse array.
/// - `i`: The index to retrieve.
/// - `outsize`: An optional mutable reference that will be set to the size (in bytes)
///   of the retrieved element.
///
/// # Returns
/// An optional slice reference to the element; `None` if the index is invalid.
pub fn sparse_array_get<'a>(
    arr: &'a SparseArray,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if (i as usize) > arr.maximum {
        return None;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    let position = (i as u32) % (GROUP_SIZE as u32);
    sparse_array_group_get(&arr.groups[group_idx], position, outsize)
}

/// Frees the sparse array.
///
/// # Parameters
/// - `arr`: The sparse array to free.
///
/// # Returns
/// A nonzero integer on success and 0 on failure.
pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    // The Box (and all owned data) is dropped automatically when this
    // function returns, freeing all memory.
    1
}

// ---------- Sparse Dictionary API ----------

/// Creates a new sparse dictionary.
///
/// # Returns
/// An owned pointer (boxed) to a new `SparseDict` or `None` on failure.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let arr = sparse_array_init(mem::size_of::<SparseBucket>(), STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*arr],
    }))
}

/// Quadratically probes for an index given the hash, probe count, and table maximum.
fn quadratic_probe(key_hash: u64, num_probes: u64, maximum: usize) -> u32 {
    (key_hash.wrapping_add(num_probes.wrapping_mul(num_probes)) & (maximum as u64 - 1)) as u32
}

/// Constructs a SparseBucket for the given key/value and writes its bytes
/// into the underlying sparse array.
fn create_and_insert_new_bucket(
    array: &mut SparseArray,
    i: u32,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
    key_hash: u64,
) -> bool {
    let key_bytes = key.as_bytes();
    let actual_klen = klen.min(key_bytes.len());
    let copied_key: String =
        String::from_utf8_lossy(&key_bytes[..actual_klen]).into_owned();
    let copied_val = value[..vlen].to_vec();
    let bucket = SparseBucket {
        key: copied_key,
        klen,
        val: copied_val,
        vlen,
        hash: key_hash,
    };
    let bucket_size = mem::size_of::<SparseBucket>();
    // SAFETY: We construct a byte slice over the local `bucket`. The bytes are
    // valid for the duration of the call to `sparse_array_set`. The underlying
    // `sparse_array_set` performs a bytewise copy of the data into the
    // sparse array. After the copy, we use `mem::forget` to prevent dropping
    // the local `bucket`, which would deallocate the heap memory backing its
    // String and Vec<u8> fields - that memory is now owned by the bytes
    // stored in the array.
    let bucket_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(&bucket as *const SparseBucket as *const u8, bucket_size)
    };
    let result = sparse_array_set(array, i, bucket_bytes, bucket_size);
    if result == 0 {
        // sparse_array_set failed; let `bucket` drop normally to free its
        // String/Vec<u8>.
        return false;
    }
    mem::forget(bucket);
    true
}

/// Allocates a new, larger sparse array and re-inserts every existing bucket.
fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let new_buckets_box =
        match sparse_array_init(mem::size_of::<SparseBucket>(), new_bucket_max as u32) {
            Some(b) => b,
            None => return 0,
        };
    let mut new_buckets = *new_buckets_box;

    let mut buckets_rehashed = 0usize;
    let bucket_size = mem::size_of::<SparseBucket>();

    for i in 0..dict.bucket_max {
        let mut bucket_siz = 0usize;
        let cv = sparse_array_get(&dict.buckets[0], i as u32, Some(&mut bucket_siz));

        if let Some(bytes) = cv {
            if bucket_siz != 0 {
                // Read the bucket's hash without taking ownership.
                // SAFETY: The bytes were originally written from a valid
                // SparseBucket via `create_and_insert_new_bucket`, so
                // reinterpreting them is sound. We immediately `mem::forget`
                // the read-out bucket so the original heap memory remains
                // valid.
                let key_hash = unsafe {
                    let bucket: SparseBucket =
                        ptr::read_unaligned(bytes.as_ptr() as *const SparseBucket);
                    let h = bucket.hash;
                    mem::forget(bucket);
                    h
                };

                let mut num_probes = 0u64;
                let mut probed_val: u32;
                loop {
                    probed_val = quadratic_probe(key_hash, num_probes, new_bucket_max);
                    let mut current_sz = 0usize;
                    let cv2 = sparse_array_get(&new_buckets, probed_val, Some(&mut current_sz));
                    if cv2.is_none() || current_sz == 0 {
                        break;
                    }
                    if num_probes > dict.bucket_count as u64 {
                        return 0;
                    }
                    num_probes += 1;
                }

                // Copy the bucket bytes from the old array into the new
                // array. This is a bytewise move - the heap pointers inside
                // the SparseBucket remain valid and are now logically owned
                // by the new array's bytes.
                let success =
                    sparse_array_set(&mut new_buckets, probed_val, bytes, bucket_size);
                if success == 0 {
                    return 0;
                }
                buckets_rehashed += 1;
            }
        }

        if buckets_rehashed == dict.bucket_count {
            break;
        }
    }

    // Replace the old buckets with the new. The old SparseArray's `Vec<u8>`
    // is dropped, freeing its raw byte storage. The SparseBuckets that lived
    // in those bytes have been bytewise-copied to the new array, so their
    // String/Vec<u8> heap memory remains owned and reachable.
    dict.buckets[0] = new_buckets;
    dict.bucket_max = new_bucket_max;
    1
}

/// Inserts a key/value pair into the dictionary.
///
/// # Parameters
/// - `dict`: The sparse dictionary.
/// - `key`: The key as a string slice.
/// - `klen`: The length of the key.
/// - `value`: A slice of bytes for the value.
/// - `vlen`: The length of the value.
///
/// # Returns
/// A nonzero integer on success and 0 on failure.
pub fn sparse_dict_set(
    dict: &mut SparseDict,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
) -> i32 {
    let key_hash = hash_fnv1a(key.as_bytes(), klen);
    let mut num_probes = 0u64;

    enum Action {
        Insert(u32),
        Replace(u32),
    }

    let action: Action;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);

        // Determine the action without holding a long-lived borrow.
        let outcome: Option<Action> = {
            let mut current_sz = 0usize;
            let cv = sparse_array_get(&dict.buckets[0], probed_val, Some(&mut current_sz));
            match cv {
                None => Some(Action::Insert(probed_val)),
                Some(_) if current_sz == 0 => Some(Action::Insert(probed_val)),
                Some(bytes) => {
                    // SAFETY: bytes were written from a valid SparseBucket;
                    // we read it unaligned, inspect, and forget so the
                    // original heap memory stays alive.
                    let matches = unsafe {
                        let bucket: SparseBucket =
                            ptr::read_unaligned(bytes.as_ptr() as *const SparseBucket);
                        let key_b = key.as_bytes();
                        let cmp_len = klen.min(key_b.len());
                        let m = bucket.hash == key_hash
                            && bucket.klen == klen
                            && bucket.key.as_bytes() == &key_b[..cmp_len];
                        mem::forget(bucket);
                        m
                    };
                    if matches {
                        Some(Action::Replace(probed_val))
                    } else {
                        None
                    }
                }
            }
        };

        match outcome {
            Some(a) => {
                action = a;
                break;
            }
            None => {
                num_probes += 1;
                if num_probes > dict.bucket_count as u64 {
                    return 0;
                }
            }
        }
    }

    match action {
        Action::Insert(probed_val) => {
            if !create_and_insert_new_bucket(
                &mut dict.buckets[0],
                probed_val,
                key,
                klen,
                value,
                vlen,
                key_hash,
            ) {
                return 0;
            }
            dict.bucket_count += 1;
            if (dict.bucket_count as f64) / (dict.bucket_max as f64)
                >= (RESIZE_PERCENT as f64) / 100.0
            {
                return rehash_and_grow_table(dict);
            }
            1
        }
        Action::Replace(probed_val) => {
            // Take ownership of the existing bucket and drop it to free its
            // String and Vec<u8>. After this, the bytes at `probed_val` are
            // logically dead and we can safely overwrite them.
            // SAFETY: bytes at probed_val were written from a valid
            // SparseBucket; reading once and dropping is sound.
            unsafe {
                let bytes = sparse_array_get(&dict.buckets[0], probed_val, None).unwrap();
                let _old: SparseBucket =
                    ptr::read_unaligned(bytes.as_ptr() as *const SparseBucket);
                // _old is dropped here, freeing its String and Vec<u8>.
            }
            if !create_and_insert_new_bucket(
                &mut dict.buckets[0],
                probed_val,
                key,
                klen,
                value,
                vlen,
                key_hash,
            ) {
                return 0;
            }
            1
        }
    }
}

/// Retrieves the value associated with a key.
///
/// # Parameters
/// - `dict`: The sparse dictionary.
/// - `key`: The key as a string slice.
/// - `klen`: The length of the key.
/// - `outsize`: An optional mutable reference that will be set to the length of the value.
///
/// # Returns
/// An optional slice reference to the value; `None` if the key is not found.
pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    mut outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let key_hash = hash_fnv1a(key.as_bytes(), klen);
    let mut num_probes = 0u64;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let mut current_sz = 0usize;

        match sparse_array_get(&dict.buckets[0], probed_val, Some(&mut current_sz)) {
            Some(bytes) if current_sz != 0 => {
                // SAFETY: bytes hold a valid SparseBucket. Read it unaligned,
                // extract the val pointer/length, and forget the local copy
                // so the heap memory remains owned by the dict.
                let (matches, val_ptr, vlen) = unsafe {
                    let bucket: SparseBucket =
                        ptr::read_unaligned(bytes.as_ptr() as *const SparseBucket);
                    let key_b = key.as_bytes();
                    let cmp_len = klen.min(key_b.len());
                    let m = bucket.hash == key_hash
                        && bucket.klen == klen
                        && bucket.key.as_bytes() == &key_b[..cmp_len];
                    let p = bucket.val.as_ptr();
                    let l = bucket.vlen;
                    mem::forget(bucket);
                    (m, p, l)
                };

                if matches {
                    if let Some(s) = outsize.as_deref_mut() {
                        *s = vlen;
                    }
                    // SAFETY: val_ptr points to heap memory owned by the
                    // SparseBucket stored in the dict. It remains valid for
                    // as long as the dict is borrowed (lifetime 'a).
                    let result = unsafe { std::slice::from_raw_parts(val_ptr, vlen) };
                    return Some(result);
                }
            }
            _ => return None,
        }

        num_probes += 1;
        if num_probes > dict.bucket_count as u64 {
            return None;
        }
    }
}

/// Frees the sparse dictionary.
///
/// # Parameters
/// - `dict`: The sparse dictionary to free.
///
/// # Returns
/// A nonzero integer on success and 0 on failure.
pub fn sparse_dict_free(dict: Box<SparseDict>) -> i32 {
    let dict = *dict;
    // For each occupied slot, take ownership of its SparseBucket and drop it.
    // This frees the String and Vec<u8> backing each bucket. The Vec<u8> raw
    // byte storage of the SparseArrayGroups is freed when `dict` is dropped
    // at the end of this function.
    for i in 0..dict.bucket_max {
        let mut current_sz = 0usize;
        let cv = sparse_array_get(&dict.buckets[0], i as u32, Some(&mut current_sz));
        if let Some(bytes) = cv {
            if current_sz != 0 {
                // SAFETY: bytes hold a valid SparseBucket; reading it once
                // and letting it drop is sound and frees the owned heap data.
                unsafe {
                    let _bucket: SparseBucket =
                        ptr::read_unaligned(bytes.as_ptr() as *const SparseBucket);
                    // _bucket dropped here.
                }
            }
        }
    }
    1
}
