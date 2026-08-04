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

// ----- Internal helpers -----

const HEADER_SIZE: usize = std::mem::size_of::<usize>();
/// elem_size used by the dictionary's internal sparse_array. We use usize::MAX
/// so that the "vlen > elem_size" check never triggers — bucket entries are
/// variable in length (header + key + value).
const DICT_ELEM_SIZE: usize = usize::MAX;

/// Reads a usize at the given offset from a byte buffer (native-endian).
#[inline]
fn read_usize(buf: &[u8], offset: usize) -> usize {
    let mut bytes = [0u8; HEADER_SIZE];
    bytes.copy_from_slice(&buf[offset..offset + HEADER_SIZE]);
    usize::from_ne_bytes(bytes)
}

/// Reads a u64 at the given offset from a byte buffer (native-endian).
#[inline]
fn read_u64(buf: &[u8], offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&buf[offset..offset + 8]);
    u64::from_ne_bytes(bytes)
}

/// Returns the index in the bitmap word array for a given position.
#[inline]
fn charbit(position: u32) -> u32 {
    position >> 5
}

/// Returns the bit-mask within a word for the given position.
#[inline]
fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

/// Convert a position (the user-facing index) to an offset (the slot index
/// within the packed group buffer). Counts the number of 1 bits in the bitmap
/// from 0 .. position - 1.
fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut bitmap_iter: usize = 0;
    while pos >= BITCHUNK_SIZE as u32 {
        retval += bitmap[bitmap_iter].count_ones();
        bitmap_iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }
    let mask: u32 = if pos == 0 {
        0
    } else {
        (1u32 << pos).wrapping_sub(1)
    };
    retval + (bitmap[bitmap_iter] & mask).count_ones()
}

#[inline]
fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    bitmap[charbit(position) as usize] & modbit(position) != 0
}

#[inline]
fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position) as usize] |= modbit(position);
}

/// Walks `slot_idx` packed entries to determine the byte offset where slot
/// `slot_idx` begins in the group buffer.
fn get_byte_offset(group: &[u8], slot_idx: usize) -> usize {
    let mut byte_offset: usize = 0;
    for _ in 0..slot_idx {
        let vlen = read_usize(group, byte_offset);
        byte_offset += HEADER_SIZE + vlen;
    }
    byte_offset
}

/// Sets the value for slot `i` in a single sparse_array_group.
fn sparse_array_group_set(arr: &mut SparseArrayGroup, i: u32, val: &[u8], vlen: usize) -> i32 {
    if vlen > arr.elem_size {
        return 0;
    }
    let offset = position_to_offset(&arr.bitmap, i) as usize;
    let occupied = is_position_occupied(&arr.bitmap, i);
    let byte_offset = get_byte_offset(&arr.group, offset);

    if occupied {
        // Overwrite existing entry. Replace the existing slot with the new
        // header + value bytes, splicing in place.
        let existing_vlen = read_usize(&arr.group, byte_offset);
        let existing_total = HEADER_SIZE + existing_vlen;

        let mut new_bytes: Vec<u8> = Vec::with_capacity(HEADER_SIZE + vlen);
        new_bytes.extend_from_slice(&vlen.to_ne_bytes());
        new_bytes.extend_from_slice(&val[..vlen]);
        arr.group
            .splice(byte_offset..byte_offset + existing_total, new_bytes);
    } else {
        // Insert a new entry, shifting later entries.
        let mut new_bytes: Vec<u8> = Vec::with_capacity(HEADER_SIZE + vlen);
        new_bytes.extend_from_slice(&vlen.to_ne_bytes());
        new_bytes.extend_from_slice(&val[..vlen]);
        arr.group
            .splice(byte_offset..byte_offset, new_bytes);
        arr.count += 1;
        set_position(&mut arr.bitmap, i);
    }

    1
}

/// Returns a slice covering the value bytes stored at slot `i`, or None if
/// the slot is unoccupied or has zero size.
fn sparse_array_group_get<'a>(
    arr: &'a SparseArrayGroup,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&arr.bitmap, i) {
        return None;
    }
    let offset = position_to_offset(&arr.bitmap, i) as usize;
    let byte_offset = get_byte_offset(&arr.group, offset);
    let vlen = read_usize(&arr.group, byte_offset);
    if vlen == 0 {
        return None;
    }
    if let Some(out) = outsize {
        *out = vlen;
    }
    Some(&arr.group[byte_offset + HEADER_SIZE..byte_offset + HEADER_SIZE + vlen])
}

/// Hash a byte slice using FNV-1a.
fn hash_fnv1a(key: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash: u64 = FNV_OFFSET_BIAS;
    for &b in key {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Quadratic probe step: (hash + n^2) & (max - 1). `max` must be a power of 2.
#[inline]
fn quadratic_probe(key_hash: u64, num_probes: u64, maximum: usize) -> u32 {
    let probed =
        key_hash.wrapping_add(num_probes.wrapping_mul(num_probes)) & (maximum as u64 - 1);
    probed as u32
}

/// Build the byte-serialized representation of a bucket.
/// Layout: [hash: 8][klen: 8][vlen: 8][key bytes][val bytes]
fn serialize_bucket(key: &str, klen: usize, value: &[u8], vlen: usize, hash: u64) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(24 + klen + vlen);
    buf.extend_from_slice(&hash.to_ne_bytes());
    buf.extend_from_slice(&(klen as u64).to_ne_bytes());
    buf.extend_from_slice(&(vlen as u64).to_ne_bytes());
    buf.extend_from_slice(&key.as_bytes()[..klen]);
    buf.extend_from_slice(&value[..vlen]);
    buf
}

// ----- Public API -----

pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let max_arr_size: usize = if maximum == 0 {
        0
    } else {
        ((maximum as usize) - 1) / GROUP_SIZE + 1
    };
    let mut groups: Vec<SparseArrayGroup> = Vec::with_capacity(max_arr_size);
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
    // Don't let users set outside the bounds of the array.
    if (i as usize) > arr.maximum {
        return 0;
    }
    let group_idx: usize = (i as usize) / GROUP_SIZE;
    let position: u32 = (i as usize % GROUP_SIZE) as u32;
    sparse_array_group_set(&mut arr.groups[group_idx], position, val, vlen)
}

pub fn sparse_array_get<'a>(
    arr: &'a SparseArray,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if (i as usize) > arr.maximum {
        return None;
    }
    let group_idx: usize = (i as usize) / GROUP_SIZE;
    let position: u32 = (i as usize % GROUP_SIZE) as u32;
    sparse_array_group_get(&arr.groups[group_idx], position, outsize)
}

pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    // Rust will drop everything here automatically. Always succeed.
    1
}

// ---------- Sparse Dictionary API ----------

pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let buckets = sparse_array_init(DICT_ELEM_SIZE, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*buckets],
    }))
}

/// Insert a freshly-built bucket into a sparse_array at the given position.
fn create_and_insert_new_bucket(
    array: &mut SparseArray,
    i: u32,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
    key_hash: u64,
) -> i32 {
    let bucket_bytes = serialize_bucket(key, klen, value, vlen, key_hash);
    let len = bucket_bytes.len();
    sparse_array_set(array, i, &bucket_bytes, len)
}

/// Re-insert all existing buckets into a new, larger sparse_array.
fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let mut new_buckets: SparseArray = match sparse_array_init(DICT_ELEM_SIZE, new_bucket_max as u32)
    {
        Some(b) => *b,
        None => return 0,
    };

    let bucket_count = dict.bucket_count;
    let bucket_max = dict.bucket_max;
    let mut buckets_rehashed: usize = 0;

    for i in 0..bucket_max as u32 {
        // Read out the bucket data into a Vec so we can mutate new_buckets later.
        let bytes_copy: Option<Vec<u8>> = {
            let mut sz: usize = 0;
            sparse_array_get(&dict.buckets[0], i, Some(&mut sz)).map(|b| {
                if sz == 0 {
                    Vec::new()
                } else {
                    b.to_vec()
                }
            })
        };

        if let Some(bytes) = bytes_copy {
            if !bytes.is_empty() {
                let hash = read_u64(&bytes, 0);
                let mut num_probes: u64 = 0;
                let probed_val: u32;
                loop {
                    let probed = quadratic_probe(hash, num_probes, new_bucket_max);
                    if sparse_array_get(&new_buckets, probed, None).is_none() {
                        probed_val = probed;
                        break;
                    }
                    if num_probes > bucket_count as u64 {
                        return 0;
                    }
                    num_probes += 1;
                }
                let blen = bytes.len();
                if sparse_array_set(&mut new_buckets, probed_val, &bytes, blen) == 0 {
                    return 0;
                }
                buckets_rehashed += 1;
            }
        }

        if buckets_rehashed == bucket_count {
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
    let key_bytes = &key.as_bytes()[..klen];
    let key_hash = hash_fnv1a(key_bytes);
    let mut num_probes: u64 = 0;
    let bucket_max = dict.bucket_max;

    let probed_val: u32 = loop {
        let probed = quadratic_probe(key_hash, num_probes, bucket_max);

        // Determine what's at `probed` without holding a borrow on dict for too
        // long, since we may need to mutate dict afterwards.
        let action: SetAction = {
            let bucket_data = sparse_array_get(&dict.buckets[0], probed, None);
            match bucket_data {
                None => SetAction::Empty,
                Some(bytes) => {
                    let existing_hash = read_u64(bytes, 0);
                    let existing_klen = read_u64(bytes, 8) as usize;
                    if existing_hash == key_hash
                        && existing_klen == klen
                        && &bytes[24..24 + existing_klen] == key_bytes
                    {
                        SetAction::Overwrite
                    } else {
                        SetAction::ProbeAgain
                    }
                }
            }
        };

        match action {
            SetAction::Empty => {
                if create_and_insert_new_bucket(
                    &mut dict.buckets[0],
                    probed,
                    key,
                    klen,
                    value,
                    vlen,
                    key_hash,
                ) == 0
                {
                    return 0;
                }
                break probed;
            }
            SetAction::Overwrite => {
                if create_and_insert_new_bucket(
                    &mut dict.buckets[0],
                    probed,
                    key,
                    klen,
                    value,
                    vlen,
                    key_hash,
                ) == 0
                {
                    return 0;
                }
                // Don't increment count, don't resize.
                return 1;
            }
            SetAction::ProbeAgain => {
                num_probes += 1;
                if num_probes > dict.bucket_count as u64 {
                    return 0;
                }
            }
        }
    };

    let _ = probed_val; // suppress unused warning (kept for clarity)
    dict.bucket_count += 1;

    // Resize check: bucket_count / bucket_max >= 0.8?
    if dict.bucket_count * 100 >= dict.bucket_max * RESIZE_PERCENT {
        return rehash_and_grow_table(dict);
    }

    1
}

enum SetAction {
    Empty,
    Overwrite,
    ProbeAgain,
}

pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let key_bytes = &key.as_bytes()[..klen];
    let key_hash = hash_fnv1a(key_bytes);
    let mut num_probes: u64 = 0;
    let bucket_max = dict.bucket_max;

    loop {
        let probed = quadratic_probe(key_hash, num_probes, bucket_max);
        match sparse_array_get(&dict.buckets[0], probed, None) {
            Some(bytes) => {
                let existing_hash = read_u64(bytes, 0);
                let existing_klen = read_u64(bytes, 8) as usize;
                let existing_vlen = read_u64(bytes, 16) as usize;
                if existing_hash == key_hash
                    && existing_klen == klen
                    && &bytes[24..24 + existing_klen] == key_bytes
                {
                    if let Some(out) = outsize {
                        *out = existing_vlen;
                    }
                    return Some(&bytes[24 + existing_klen..24 + existing_klen + existing_vlen]);
                }
            }
            None => return None,
        }

        num_probes += 1;
        if num_probes > dict.bucket_count as u64 {
            return None;
        }
    }
}

pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    // Rust will drop everything here automatically.
    1
}
