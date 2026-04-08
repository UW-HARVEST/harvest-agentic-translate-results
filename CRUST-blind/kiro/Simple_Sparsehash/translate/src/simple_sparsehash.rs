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

// ---- Helper functions (private) ----

fn hash_fnv1a(key: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash = FNV_OFFSET_BIAS;
    // C code uses `uint8_t i` which wraps at 256, limiting iterations
    let iterations = key.len() as u8;
    for i in 0..iterations {
        hash ^= key[i as usize] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn charbit(position: u32) -> usize {
    (position >> 5) as usize
}

fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

fn popcount_32(mut x: u32) -> u32 {
    let m1: u32 = 0x55555555;
    let m2: u32 = 0x33333333;
    let m4: u32 = 0x0f0f0f0f;
    x -= (x >> 1) & m1;
    x = (x & m2) + ((x >> 2) & m2);
    x = (x.wrapping_add(x >> 4)) & m4;
    x = x.wrapping_add(x >> 8);
    (x.wrapping_add(x >> 16)) & 0x3f
}

fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut bitmap_iter: usize = 0;
    while pos >= BITCHUNK_SIZE as u32 {
        retval += popcount_32(bitmap[bitmap_iter]);
        bitmap_iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }
    retval + popcount_32(bitmap[bitmap_iter] & ((1u32 << pos).wrapping_sub(1)))
}

fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    (bitmap[charbit(position)] & modbit(position)) != 0
}

fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position)] |= modbit(position);
}

fn full_elem_size(elem_size: usize) -> usize {
    elem_size + std::mem::size_of::<usize>()
}

// ---- Sparse Array Group (private) ----

fn sparse_array_group_set(arr: &mut SparseArrayGroup, i: u32, val: &[u8], vlen: usize) -> bool {
    if vlen > arr.elem_size {
        return false;
    }
    let fes = full_elem_size(arr.elem_size);
    let offset = position_to_offset(&arr.bitmap, i) as usize;

    if !is_position_occupied(&arr.bitmap, i) {
        let to_move_siz = (arr.count as usize - offset) * fes;
        let new_len = (arr.count as usize + 1) * fes;
        arr.group.resize(new_len, 0);
        if to_move_siz > 0 {
            let src = offset * fes;
            arr.group.copy_within(src..src + to_move_siz, (offset + 1) * fes);
        }
        arr.count += 1;
        set_position(&mut arr.bitmap, i);
    }

    let dest = offset * fes;
    let size_bytes = vlen.to_ne_bytes();
    arr.group[dest..dest + std::mem::size_of::<usize>()].copy_from_slice(&size_bytes);
    let data_start = dest + std::mem::size_of::<usize>();
    arr.group[data_start..data_start + vlen].copy_from_slice(&val[..vlen]);
    true
}

fn sparse_array_group_get<'a>(arr: &'a SparseArrayGroup, i: u32, outsize: Option<&mut usize>) -> Option<&'a [u8]> {
    if !is_position_occupied(&arr.bitmap, i) {
        return None;
    }
    let fes = full_elem_size(arr.elem_size);
    let offset = position_to_offset(&arr.bitmap, i) as usize;
    let base = offset * fes;
    let size_of_usize = std::mem::size_of::<usize>();

    let mut item_siz_bytes = [0u8; std::mem::size_of::<usize>()];
    item_siz_bytes.copy_from_slice(&arr.group[base..base + size_of_usize]);
    let item_siz = usize::from_ne_bytes(item_siz_bytes);

    if item_siz == 0 {
        return None;
    }

    if let Some(out) = outsize {
        *out = item_siz;
    }

    let data_start = base + size_of_usize;
    Some(&arr.group[data_start..data_start + item_siz])
}

// ---- Sparse Array public API ----

pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let max_arr_size = (maximum as usize).saturating_sub(1) / GROUP_SIZE + 1;
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

pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if i as usize > arr.maximum {
        return 0;
    }
    let group_idx = i as usize / GROUP_SIZE;
    let position = (i as usize % GROUP_SIZE) as u32;
    if sparse_array_group_set(&mut arr.groups[group_idx], position, val, vlen) { 1 } else { 0 }
}

pub fn sparse_array_get<'a>(
    arr: &'a SparseArray,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if i as usize > arr.maximum {
        return None;
    }
    let group_idx = i as usize / GROUP_SIZE;
    let position = (i as usize % GROUP_SIZE) as u32;
    sparse_array_group_get(&arr.groups[group_idx], position, outsize)
}

pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    // Rust drops automatically
    1
}

// ---- SparseBucket serialization helpers ----

fn bucket_to_bytes(b: &SparseBucket) -> Vec<u8> {
    let mut buf = Vec::new();
    let klen_bytes = b.klen.to_ne_bytes();
    let vlen_bytes = b.vlen.to_ne_bytes();
    let hash_bytes = b.hash.to_ne_bytes();
    // Layout: klen | vlen | hash | key bytes | val bytes
    buf.extend_from_slice(&klen_bytes);
    buf.extend_from_slice(&vlen_bytes);
    buf.extend_from_slice(&hash_bytes);
    buf.extend_from_slice(b.key.as_bytes());
    buf.extend_from_slice(&b.val);
    buf
}

fn bucket_from_bytes(data: &[u8]) -> SparseBucket {
    let sz = std::mem::size_of::<usize>();
    let mut klen_bytes = [0u8; std::mem::size_of::<usize>()];
    klen_bytes.copy_from_slice(&data[0..sz]);
    let klen = usize::from_ne_bytes(klen_bytes);

    let mut vlen_bytes = [0u8; std::mem::size_of::<usize>()];
    vlen_bytes.copy_from_slice(&data[sz..2 * sz]);
    let vlen = usize::from_ne_bytes(vlen_bytes);

    let mut hash_bytes = [0u8; 8];
    hash_bytes.copy_from_slice(&data[2 * sz..2 * sz + 8]);
    let hash = u64::from_ne_bytes(hash_bytes);

    let key_start = 2 * sz + 8;
    let key = String::from_utf8_lossy(&data[key_start..key_start + klen]).into_owned();
    let val = data[key_start + klen..key_start + klen + vlen].to_vec();

    SparseBucket { key, klen, val, vlen, hash }
}

fn bucket_serialized_size() -> usize {
    // This is the "element_size" we pass to sparse_array_init
    // Must be large enough to hold any serialized bucket
    // We use a generous fixed size
    std::mem::size_of::<usize>() * 2 + 8 + 256 // klen + vlen + hash + key + val
}

// ---- Sparse Dictionary public API ----

pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let elem_size = bucket_serialized_size();
    let buckets = sparse_array_init(elem_size, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*buckets],
    }))
}

fn quadratic_probe(key_hash: u64, num_probes: usize, maximum: usize) -> usize {
    ((key_hash as usize) + num_probes * num_probes) & (maximum - 1)
}

fn dict_buckets(dict: &SparseDict) -> &SparseArray {
    &dict.buckets[0]
}

fn dict_buckets_mut(dict: &mut SparseDict) -> &mut SparseArray {
    &mut dict.buckets[0]
}

fn create_and_insert_new_bucket(
    array: &mut SparseArray,
    i: u32,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
    key_hash: u64,
) -> bool {
    let bucket = SparseBucket {
        key: key[..klen].to_string(),
        klen,
        val: value[..vlen].to_vec(),
        vlen,
        hash: key_hash,
    };
    let serialized = bucket_to_bytes(&bucket);
    sparse_array_set(array, i, &serialized, serialized.len()) != 0
}

fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let elem_size = bucket_serialized_size();
    let mut new_buckets = match sparse_array_init(elem_size, new_bucket_max as u32) {
        Some(b) => *b,
        None => return 0,
    };

    let mut buckets_rehashed = 0usize;
    for i in 0..dict.bucket_max {
        let mut bucket_siz: usize = 0;
        let bucket_data = sparse_array_get(dict_buckets(dict), i as u32, Some(&mut bucket_siz));
        if bucket_siz != 0 {
            if let Some(data) = bucket_data {
                let bucket = bucket_from_bytes(data);
                let key_hash = bucket.hash;
                let mut num_probes: usize = 0;
                loop {
                    let probed_val = quadratic_probe(key_hash, num_probes, new_bucket_max);
                    let mut current_siz: usize = 0;
                    let current = sparse_array_get(&new_buckets, probed_val as u32, Some(&mut current_siz));
                    if current_siz == 0 && current.is_none() {
                        break;
                    }
                    if num_probes > dict.bucket_count {
                        return 0;
                    }
                    num_probes += 1;
                }
                let probed_val = quadratic_probe(key_hash, num_probes, new_bucket_max);
                let serialized = bucket_to_bytes(&bucket);
                if sparse_array_set(&mut new_buckets, probed_val as u32, &serialized, serialized.len()) == 0 {
                    return 0;
                }
                buckets_rehashed += 1;
            }
        }
        if buckets_rehashed == dict.bucket_count {
            break;
        }
    }

    dict.buckets[0] = new_buckets;
    dict.bucket_max = new_bucket_max;
    1
}

pub fn sparse_dict_set(
    dict: &mut SparseDict,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
) -> i32 {
    let key_hash = hash_fnv1a(&key.as_bytes()[..klen]);
    let mut num_probes: usize = 0;

    loop {
        let mut current_value_siz: usize = 0;
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let current_value = sparse_array_get(dict_buckets(dict), probed_val as u32, Some(&mut current_value_siz));

        if current_value_siz == 0 && current_value.is_none() {
            if create_and_insert_new_bucket(dict_buckets_mut(dict), probed_val as u32, key, klen, value, vlen, key_hash) {
                break;
            } else {
                return 0;
            }
        } else if let Some(data) = current_value {
            let existing = bucket_from_bytes(data);
            if existing.hash == key_hash && existing.klen == klen && existing.key[..klen] == key[..klen] {
                if create_and_insert_new_bucket(dict_buckets_mut(dict), probed_val as u32, key, klen, value, vlen, key_hash) {
                    return 1;
                } else {
                    return 0;
                }
            }
        }

        num_probes += 1;
        if num_probes > dict.bucket_count {
            return 0;
        }
    }

    dict.bucket_count += 1;

    if dict.bucket_count as f64 / dict.bucket_max as f64 >= RESIZE_PERCENT as f64 / 100.0 {
        return rehash_and_grow_table(dict);
    }

    1
}

pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let key_hash = hash_fnv1a(&key.as_bytes()[..klen]);
    let mut num_probes: usize = 0;

    loop {
        let mut current_value_siz: usize = 0;
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let current_value = sparse_array_get(dict_buckets(dict), probed_val as u32, Some(&mut current_value_siz));

        if current_value_siz != 0 {
            if let Some(data) = current_value {
                let existing = bucket_from_bytes(data);
                if existing.hash == key_hash && existing.klen == klen && existing.key[..klen] == key[..klen] {
                    // We need to return a reference to the value bytes within the stored data.
                    // The layout is: klen(usize) | vlen(usize) | hash(8) | key(klen) | val(vlen)
                    let sz = std::mem::size_of::<usize>();
                    let val_start = 2 * sz + 8 + existing.klen;
                    let val_end = val_start + existing.vlen;
                    if let Some(out) = outsize {
                        *out = existing.vlen;
                    }
                    return Some(&data[val_start..val_end]);
                }
            }
        } else {
            return None;
        }

        num_probes += 1;
        if num_probes > dict.bucket_count {
            return None;
        }
    }
}

pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    1
}
