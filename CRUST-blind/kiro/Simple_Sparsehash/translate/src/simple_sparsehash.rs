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
    pub key: String,
    pub klen: usize,
    pub val: Vec<u8>,
    pub vlen: usize,
    pub hash: u64,
}

/// One group in a sparse array.
#[derive(Debug)]
pub struct SparseArrayGroup {
    pub count: u32,
    pub elem_size: usize,
    pub group: Vec<u8>,
    pub bitmap: [u32; BITMAP_SIZE],
}

/// A sparse array consisting of one or more groups.
#[derive(Debug)]
pub struct SparseArray {
    pub maximum: usize,
    pub groups: Vec<SparseArrayGroup>,
}

/// A sparse dictionary that maps keys to values.
#[derive(Debug)]
pub struct SparseDict {
    pub bucket_max: usize,
    pub bucket_count: usize,
    pub buckets: Vec<SparseArray>,
}

// --- Internal helpers ---

fn hash_fnv1a(key: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash = FNV_OFFSET_BIAS;
    for &b in key {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn charbit(position: u32) -> usize {
    (position >> 5) as usize
}

fn modbit(position: u32) -> u32 {
    1 << (position & 31)
}

fn popcount_32(mut x: u32) -> u32 {
    let m1: u32 = 0x55555555;
    let m2: u32 = 0x33333333;
    let m4: u32 = 0x0f0f0f0f;
    x -= (x >> 1) & m1;
    x = (x & m2) + ((x >> 2) & m2);
    x = (x + (x >> 4)) & m4;
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
    retval + popcount_32(bitmap[bitmap_iter] & ((1u32 << pos) - 1))
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

// --- SparseArrayGroup internal operations ---

fn sparse_array_group_set(grp: &mut SparseArrayGroup, i: u32, val: &[u8], vlen: usize) -> bool {
    if vlen > grp.elem_size {
        return false;
    }
    let fes = full_elem_size(grp.elem_size);
    let offset = position_to_offset(&grp.bitmap, i) as usize;

    if !is_position_occupied(&grp.bitmap, i) {
        let to_move_siz = (grp.count as usize - offset) * fes;
        grp.group.resize(grp.group.len() + fes, 0);
        if to_move_siz > 0 {
            let src_start = offset * fes;
            grp.group.copy_within(src_start..src_start + to_move_siz, (offset + 1) * fes);
        }
        grp.count += 1;
        set_position(&mut grp.bitmap, i);
    }

    let dest_start = offset * fes;
    let size_bytes = vlen.to_ne_bytes();
    grp.group[dest_start..dest_start + std::mem::size_of::<usize>()].copy_from_slice(&size_bytes);
    let val_start = dest_start + std::mem::size_of::<usize>();
    for b in &mut grp.group[val_start..val_start + grp.elem_size] {
        *b = 0;
    }
    grp.group[val_start..val_start + vlen].copy_from_slice(&val[..vlen]);
    true
}

fn sparse_array_group_get(grp: &SparseArrayGroup, i: u32) -> Option<(usize, usize)> {
    if !is_position_occupied(&grp.bitmap, i) {
        return None;
    }
    let fes = full_elem_size(grp.elem_size);
    let offset = position_to_offset(&grp.bitmap, i) as usize;
    let dest_start = offset * fes;

    let mut size_bytes = [0u8; std::mem::size_of::<usize>()];
    size_bytes.copy_from_slice(&grp.group[dest_start..dest_start + std::mem::size_of::<usize>()]);
    let item_size = usize::from_ne_bytes(size_bytes);

    if item_size == 0 {
        return None;
    }

    let val_start = dest_start + std::mem::size_of::<usize>();
    Some((val_start, item_size))
}

// --- Public Sparse Array API ---

pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
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

pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if i as usize > arr.maximum {
        return 0;
    }
    let grp = &mut arr.groups[i as usize / GROUP_SIZE];
    let position = (i as usize % GROUP_SIZE) as u32;
    if sparse_array_group_set(grp, position, val, vlen) { 1 } else { 0 }
}

pub fn sparse_array_get<'a>(
    arr: &'a SparseArray,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if i as usize > arr.maximum {
        return None;
    }
    let grp = &arr.groups[i as usize / GROUP_SIZE];
    let position = (i as usize % GROUP_SIZE) as u32;
    match sparse_array_group_get(grp, position) {
        Some((val_start, item_size)) => {
            if let Some(out) = outsize {
                *out = item_size;
            }
            Some(&grp.group[val_start..val_start + grp.elem_size])
        }
        None => None,
    }
}

pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    1
}

// --- Sparse Dictionary ---

// Serialized bucket layout: hash(8) + klen(usize) + vlen(usize) + key_bytes + val_bytes
const BUCKET_ELEM_SIZE: usize = 512;
const BUCKET_HEADER_SIZE: usize = 8 + std::mem::size_of::<usize>() * 2;

fn serialize_bucket_into(buf: &mut [u8], bucket: &SparseBucket) {
    let sz = std::mem::size_of::<usize>();
    let mut off = 0;
    buf[off..off + 8].copy_from_slice(&bucket.hash.to_ne_bytes());
    off += 8;
    buf[off..off + sz].copy_from_slice(&bucket.klen.to_ne_bytes());
    off += sz;
    buf[off..off + sz].copy_from_slice(&bucket.vlen.to_ne_bytes());
    off += sz;
    buf[off..off + bucket.klen].copy_from_slice(bucket.key.as_bytes());
    off += bucket.klen;
    buf[off..off + bucket.vlen].copy_from_slice(&bucket.val);
}

fn deserialize_bucket(data: &[u8]) -> SparseBucket {
    let sz = std::mem::size_of::<usize>();
    let mut off = 0;
    let hash = u64::from_ne_bytes(data[off..off + 8].try_into().unwrap());
    off += 8;
    let klen = usize::from_ne_bytes(data[off..off + sz].try_into().unwrap());
    off += sz;
    let vlen = usize::from_ne_bytes(data[off..off + sz].try_into().unwrap());
    off += sz;
    let key = String::from_utf8_lossy(&data[off..off + klen]).to_string();
    off += klen;
    let val = data[off..off + vlen].to_vec();
    SparseBucket { key, klen, val, vlen, hash }
}

pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let buckets_arr = sparse_array_init(BUCKET_ELEM_SIZE, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*buckets_arr],
    }))
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
    let mut buf = vec![0u8; BUCKET_ELEM_SIZE];
    serialize_bucket_into(&mut buf, &bucket);
    sparse_array_set(array, i, &buf, BUCKET_ELEM_SIZE) != 0
}

fn rehash_and_grow_table(dict: &mut SparseDict) -> bool {
    let new_bucket_max = dict.bucket_max * 2;
    let new_buckets_arr = match sparse_array_init(BUCKET_ELEM_SIZE, new_bucket_max as u32) {
        Some(b) => b,
        None => return false,
    };
    let mut new_buckets = *new_buckets_arr;
    let mut buckets_rehashed = 0usize;

    for i in 0..dict.bucket_max {
        let mut bucket_siz = 0usize;
        let bucket_data = sparse_array_get(&dict.buckets[0], i as u32, Some(&mut bucket_siz));
        if bucket_siz != 0 {
            if let Some(data) = bucket_data {
                let bucket = deserialize_bucket(data);
                let key_hash = bucket.hash;
                let mut num_probes: usize = 0;
                loop {
                    let probed_val = ((key_hash as usize) + num_probes * num_probes) & (new_bucket_max - 1);
                    let mut current_siz = 0usize;
                    let current = sparse_array_get(&new_buckets, probed_val as u32, Some(&mut current_siz));
                    if current_siz == 0 && current.is_none() {
                        let mut buf = vec![0u8; BUCKET_ELEM_SIZE];
                        serialize_bucket_into(&mut buf, &bucket);
                        if sparse_array_set(&mut new_buckets, probed_val as u32, &buf, BUCKET_ELEM_SIZE) == 0 {
                            return false;
                        }
                        break;
                    }
                    if num_probes > dict.bucket_count {
                        return false;
                    }
                    num_probes += 1;
                }
                buckets_rehashed += 1;
            }
        }
        if buckets_rehashed == dict.bucket_count {
            break;
        }
    }

    dict.buckets = vec![new_buckets];
    dict.bucket_max = new_bucket_max;
    true
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
        let probed_val = ((key_hash as usize) + num_probes * num_probes) & (dict.bucket_max - 1);
        let mut current_value_siz = 0usize;
        let current_value = sparse_array_get(&dict.buckets[0], probed_val as u32, Some(&mut current_value_siz));

        if current_value_siz == 0 && current_value.is_none() {
            if create_and_insert_new_bucket(&mut dict.buckets[0], probed_val as u32, key, klen, value, vlen, key_hash) {
                break;
            } else {
                return 0;
            }
        } else if let Some(data) = current_value {
            let existing = deserialize_bucket(data);
            if existing.hash == key_hash && existing.klen == klen && existing.key == key[..klen] {
                if create_and_insert_new_bucket(&mut dict.buckets[0], probed_val as u32, key, klen, value, vlen, key_hash) {
                    return 1;
                } else {
                    return 0;
                }
            }
        }

        num_probes += 1;
        if num_probes > dict.bucket_count {
            println!("Could not find an open slot in the table.");
            return 0;
        }
    }

    dict.bucket_count += 1;

    if dict.bucket_count as f64 / dict.bucket_max as f64 >= RESIZE_PERCENT as f64 / 100.0 {
        return if rehash_and_grow_table(dict) { 1 } else { 0 };
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
        let probed_val = ((key_hash as usize) + num_probes * num_probes) & (dict.bucket_max - 1);
        let mut current_value_siz = 0usize;
        let current_value = sparse_array_get(&dict.buckets[0], probed_val as u32, Some(&mut current_value_siz));

        if current_value_siz != 0 {
            if let Some(data) = current_value {
                let sz = std::mem::size_of::<usize>();
                let hash = u64::from_ne_bytes(data[0..8].try_into().unwrap());
                let bklen = usize::from_ne_bytes(data[8..8 + sz].try_into().unwrap());
                let bvlen = usize::from_ne_bytes(data[8 + sz..8 + 2 * sz].try_into().unwrap());
                let key_start = BUCKET_HEADER_SIZE;
                let bkey = &data[key_start..key_start + bklen];

                if hash == key_hash && bklen == klen && bkey == key[..klen].as_bytes() {
                    if let Some(out) = outsize {
                        *out = bvlen;
                    }
                    let val_start = key_start + bklen;
                    return Some(&data[val_start..val_start + bvlen]);
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
