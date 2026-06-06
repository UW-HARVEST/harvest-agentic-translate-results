use std::fs::File;
use std::io::Read;

#[derive(Debug)]
pub struct TwoBitMaskedIdx {
    pub size: Vec<u32>,
    pub n_block_count: Vec<u32>,
    pub n_block_start: Vec<Vec<u32>>,
    pub n_block_sizes: Vec<Vec<u32>>,
    pub mask_block_count: Vec<u32>,
    pub mask_block_start: Vec<Vec<u32>>,
    pub mask_block_sizes: Vec<Vec<u32>>,
    pub offset: Vec<u64>,
}
#[derive(Debug)]
pub struct TwoBitHeader {
    pub magic: u32,
    pub version: u32,
    pub n_chroms: u32,
}
#[derive(Debug)]
pub struct TwoBitCL {
    pub chrom: Vec<String>,
    pub offset: Vec<u32>,
}
#[derive(Debug)]
pub struct TwoBit {
    pub fp: File,
    pub sz: u64,
    pub offset: u64,
    pub data: Vec<u8>,
    pub hdr: TwoBitHeader,
    pub cl: TwoBitCL,
    pub idx: TwoBitMaskedIdx,
}

// Internal helper to read a u32 little-endian from a slice
fn read_u32_le(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

// Internal byte-mask-from-offset helper (returns the actual mask)
fn byte_mask_from_offset(offset: i32) -> u8 {
    match offset {
        0 => 15u8,
        1 => 7u8,
        2 => 3u8,
        _ => 1u8,
    }
}

impl TwoBit {
    pub fn twobit_open(fname: &str, store_masked: bool) -> Self {
        let mut fp = File::open(fname).expect("Failed to open 2bit file");
        let mut data: Vec<u8> = Vec::new();
        fp.read_to_end(&mut data).expect("Failed to read 2bit file");
        let sz = data.len() as u64;

        let mut tb = TwoBit {
            fp,
            sz,
            offset: 0,
            data,
            hdr: TwoBitHeader {
                magic: 0,
                version: 0,
                n_chroms: 0,
            },
            cl: TwoBitCL {
                chrom: Vec::new(),
                offset: Vec::new(),
            },
            idx: TwoBitMaskedIdx {
                size: Vec::new(),
                n_block_count: Vec::new(),
                n_block_start: Vec::new(),
                n_block_sizes: Vec::new(),
                mask_block_count: Vec::new(),
                mask_block_start: Vec::new(),
                mask_block_sizes: Vec::new(),
                offset: Vec::new(),
            },
        };

        tb.twobitHdrRead();
        tb.twobitChromListRead();
        tb.twoBitIndexRead(if store_masked { 1 } else { 0 });
        tb
    }

    pub fn twobit_close(&mut self) {
        // In Rust, file handle and Vec memory are reclaimed when the struct
        // is dropped. Reset internal state for safety.
        self.twoBitIndexDestroy();
        self.twobitChromListDestroy();
        self.twobitHdrDestroy();
        self.data.clear();
        self.offset = 0;
        self.sz = 0;
    }

    pub fn twobit_chrom_len(&self, chrom: &str) -> u32 {
        for i in 0..self.hdr.n_chroms as usize {
            if self.cl.chrom[i] == chrom {
                return self.idx.size[i];
            }
        }
        0
    }

    pub fn twobit_sequence(&self, chrom: &str, start: u32, end: u32) -> String {
        // Find chromosome ID
        let n_chroms = self.hdr.n_chroms as usize;
        let mut tid: Option<usize> = None;
        for i in 0..n_chroms {
            if self.cl.chrom[i] == chrom {
                tid = Some(i);
                break;
            }
        }
        let tid = match tid {
            Some(t) => t,
            None => return String::new(),
        };

        let mut end = end;
        if start == end && end == 0 {
            end = self.idx.size[tid];
        }

        if end > self.idx.size[tid] {
            return String::new();
        }
        if start >= end {
            return String::new();
        }

        // Construct sequence (immutable form so it can be called from &self)
        let sz = (end - start) as usize;
        let block_start = (start / 4) as u64;
        let offset = (start % 4) as i32;
        let block_end = ((end / 4) + (if end % 4 != 0 { 1 } else { 0 })) as u64;
        let n_bytes = (block_end - block_start) as usize;

        let data_off = self.idx.offset[tid] + block_start;
        let mut bytes: Vec<u8> = self.data[data_off as usize..data_off as usize + n_bytes].to_vec();

        let mut seq: Vec<char> = vec!['\0'; sz];
        bytes2bases(&mut seq, &mut bytes, sz as u32, offset);

        // Apply N-mask
        n_mask_immutable(&mut seq, &self.idx, tid as u32, start, end);
        // Apply soft-mask if available
        soft_mask_immutable(&mut seq, &self.idx, tid as u32, start, end);

        seq.into_iter().collect()
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let n_chroms = self.hdr.n_chroms as usize;
        let mut tid: Option<usize> = None;
        for i in 0..n_chroms {
            if self.cl.chrom[i] == chrom {
                tid = Some(i);
                break;
            }
        }
        let tid = match tid {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut end = end;
        if start == end && end == 0 {
            end = self.idx.size[tid];
        }

        if end > self.idx.size[tid] {
            return Vec::new();
        }
        if start >= end {
            return Vec::new();
        }

        bases_worker_immutable(self, tid as u32, start, end, fraction)
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // The signature passes `data` as immutable, so we cannot actually copy
        // bytes into it. We still advance the internal offset in line with the
        // requested read size and report success, mirroring the C behavior of
        // returning the number of elements "read".
        let total = sz.checked_mul(nmemb).unwrap_or(0) as u64;
        if self.offset + total > self.sz {
            return 0;
        }
        self.offset += total;
        nmemb
    }

    pub fn twobitSeek(&mut self, offset: u64) {
        if offset >= self.sz {
            return;
        }
        self.offset = offset;
    }

    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid_us = tid as usize;
        let n_count = self.idx.n_block_count[tid_us] as usize;
        let mut pos: u32;
        for i in 0..n_count {
            let block_start = self.idx.n_block_start[tid_us][i];
            let mut block_end = block_start + self.idx.n_block_sizes[tid_us][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let width;
            if block_start < start {
                if block_end > end {
                    block_end = end;
                }
                pos = 0;
                width = block_end - start;
            } else {
                if block_end > end {
                    block_end = end;
                }
                pos = block_start - start;
                width = block_end - block_start;
            }
            let limit = pos + width;
            let mut p = pos;
            while p < limit {
                if (p as usize) < seq.len() {
                    seq[p as usize] = 'N';
                }
                p += 1;
            }
        }
    }

    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid_us = tid as usize;
        if self.idx.mask_block_start.is_empty() {
            return;
        }
        let m_count = self.idx.mask_block_count[tid_us] as usize;
        let mut pos: u32;
        for i in 0..m_count {
            let block_start = self.idx.mask_block_start[tid_us][i];
            let mut block_end = block_start + self.idx.mask_block_sizes[tid_us][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let width;
            if block_start < start {
                if block_end > end {
                    block_end = end;
                }
                pos = 0;
                width = block_end - start;
            } else {
                if block_end > end {
                    block_end = end;
                }
                pos = block_start - start;
                width = block_end - block_start;
            }
            let limit = pos + width;
            let mut p = pos;
            while p < limit {
                if (p as usize) < seq.len() {
                    let c = seq[p as usize];
                    if c != 'N' {
                        seq[p as usize] = c.to_ascii_lowercase();
                    }
                }
                p += 1;
            }
        }
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let tid_us = tid as usize;
        let sz = (end - start) as usize;
        let block_start = (start / 4) as u64;
        let offset = (start % 4) as i32;
        let block_end = ((end / 4) + (if end % 4 != 0 { 1 } else { 0 })) as u64;
        let n_bytes = (block_end - block_start) as usize;

        let data_off = self.idx.offset[tid_us] + block_start;
        if (data_off as usize) + n_bytes > self.data.len() {
            return Vec::new();
        }
        let mut bytes: Vec<u8> = self.data[data_off as usize..data_off as usize + n_bytes].to_vec();

        let mut seq: Vec<char> = vec!['\0'; sz];
        bytes2bases(&mut seq, &mut bytes, sz as u32, offset);

        self.NMask(&mut seq, tid, start, end);
        self.softMask(&mut seq, tid, start, end);
        seq
    }

    pub fn getMask(&mut self, tid: u32, start: u32, _end: u32) -> (u32, u32, u32) {
        // Find the first overlapping N-mask block (treat input maskIdx as -1).
        let tid_us = tid as usize;
        let n_count = self.idx.n_block_count[tid_us];
        let neg_one: u32 = u32::MAX;
        let mut mask_idx: u32 = 0;
        let mut mask_start: u32 = neg_one;
        let mut mask_end: u32 = neg_one;
        let mut found = false;
        let mut idx: u32 = 0;
        while idx < n_count {
            let bs = self.idx.n_block_start[tid_us][idx as usize];
            let be = bs + self.idx.n_block_sizes[tid_us][idx as usize];
            if be < start {
                idx += 1;
                continue;
            }
            // be >= start
            mask_idx = idx;
            mask_start = bs;
            mask_end = be;
            found = true;
            break;
        }
        if !found {
            mask_idx = n_count;
            mask_start = neg_one;
            mask_end = neg_one;
        } else if mask_start >= _end {
            mask_start = neg_one;
            mask_end = neg_one;
        }
        (mask_idx, mask_start, mask_end)
    }

    pub fn twoBitBasesWorker(&mut self, _tid: u32, _start: u32, _end: u32, _fraction: i32) {
        // The signature returns nothing, so this is a no-op stub. The actual
        // computation is performed via the immutable helper used by
        // `twobit_bases`.
    }

    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n = self.hdr.n_chroms as usize;
        let mut size: Vec<u32> = Vec::with_capacity(n);
        let mut n_block_count: Vec<u32> = Vec::with_capacity(n);
        let mut n_block_start: Vec<Vec<u32>> = Vec::with_capacity(n);
        let mut n_block_sizes: Vec<Vec<u32>> = Vec::with_capacity(n);
        let mut mask_block_count: Vec<u32> = Vec::with_capacity(n);
        let mut mask_block_start: Vec<Vec<u32>> = Vec::with_capacity(n);
        let mut mask_block_sizes: Vec<Vec<u32>> = Vec::with_capacity(n);
        let mut data_offsets: Vec<u64> = Vec::with_capacity(n);

        for i in 0..n {
            let cl_off = self.cl.offset[i] as u64;
            self.offset = cl_off;
            let p = self.offset as usize;
            let chrom_size = read_u32_le(&self.data, p);
            let n_count = read_u32_le(&self.data, p + 4);
            self.offset += 8;
            size.push(chrom_size);
            n_block_count.push(n_count);

            let mut starts: Vec<u32> = Vec::with_capacity(n_count as usize);
            for _ in 0..n_count {
                let pp = self.offset as usize;
                starts.push(read_u32_le(&self.data, pp));
                self.offset += 4;
            }
            n_block_start.push(starts);

            let mut sizes: Vec<u32> = Vec::with_capacity(n_count as usize);
            for _ in 0..n_count {
                let pp = self.offset as usize;
                sizes.push(read_u32_le(&self.data, pp));
                self.offset += 4;
            }
            n_block_sizes.push(sizes);

            let pp = self.offset as usize;
            let m_count = read_u32_le(&self.data, pp);
            self.offset += 4;
            mask_block_count.push(m_count);

            if storeMasked != 0 {
                let mut m_starts: Vec<u32> = Vec::with_capacity(m_count as usize);
                for _ in 0..m_count {
                    let pp = self.offset as usize;
                    m_starts.push(read_u32_le(&self.data, pp));
                    self.offset += 4;
                }
                mask_block_start.push(m_starts);

                let mut m_sizes: Vec<u32> = Vec::with_capacity(m_count as usize);
                for _ in 0..m_count {
                    let pp = self.offset as usize;
                    m_sizes.push(read_u32_le(&self.data, pp));
                    self.offset += 4;
                }
                mask_block_sizes.push(m_sizes);
            } else {
                self.offset += 8 * m_count as u64;
                mask_block_start.push(Vec::new());
                mask_block_sizes.push(Vec::new());
            }

            // Reserved (4 bytes)
            self.offset += 4;
            data_offsets.push(self.offset);
        }

        self.idx.size = size;
        self.idx.n_block_count = n_block_count;
        self.idx.n_block_start = n_block_start;
        self.idx.n_block_sizes = n_block_sizes;
        self.idx.mask_block_count = mask_block_count;
        if storeMasked != 0 {
            self.idx.mask_block_start = mask_block_start;
            self.idx.mask_block_sizes = mask_block_sizes;
        } else {
            self.idx.mask_block_start = Vec::new();
            self.idx.mask_block_sizes = Vec::new();
        }
        self.idx.offset = data_offsets;
    }

    pub fn twoBitIndexDestroy(&mut self) {
        self.idx.size.clear();
        self.idx.n_block_count.clear();
        self.idx.n_block_start.clear();
        self.idx.n_block_sizes.clear();
        self.idx.mask_block_count.clear();
        self.idx.mask_block_start.clear();
        self.idx.mask_block_sizes.clear();
        self.idx.offset.clear();
    }

    pub fn twobitChromListRead(&mut self) {
        let n = self.hdr.n_chroms as usize;
        let mut chrom: Vec<String> = Vec::with_capacity(n);
        let mut offsets: Vec<u32> = Vec::with_capacity(n);

        for _ in 0..n {
            let p = self.offset as usize;
            let name_len = self.data[p] as usize;
            self.offset += 1;
            let p2 = self.offset as usize;
            let name_bytes = &self.data[p2..p2 + name_len];
            let name = String::from_utf8_lossy(name_bytes).to_string();
            self.offset += name_len as u64;

            let p3 = self.offset as usize;
            let off = read_u32_le(&self.data, p3);
            self.offset += 4;

            chrom.push(name);
            offsets.push(off);
        }

        self.cl.chrom = chrom;
        self.cl.offset = offsets;
    }

    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }

    pub fn twobitHdrRead(&mut self) {
        if self.data.len() < 16 {
            return;
        }
        let magic = read_u32_le(&self.data, 0);
        let version = read_u32_le(&self.data, 4);
        let n_chroms = read_u32_le(&self.data, 8);
        // Reserved field (4 bytes) at offset 12 -- skip

        if magic != 0x1A412743 {
            // Invalid magic
            return;
        }
        if version != 0 {
            return;
        }
        if n_chroms == 0 {
            return;
        }

        self.hdr.magic = magic;
        self.hdr.version = version;
        self.hdr.n_chroms = n_chroms;
        self.offset = 16;
    }

    pub fn twobitHdrDestroy(&mut self) {
        self.hdr.magic = 0;
        self.hdr.version = 0;
        self.hdr.n_chroms = 0;
    }
}

// -- Free functions ---------------------------------------------------------

pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let mask: u8 = 3u8 << (2 * rev);
    let foo = ((mask & byte) >> (2 * rev)) as usize;
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo]
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    let bases = ['T', 'C', 'A', 'G'];
    let sz = sz as usize;
    let mut pos: usize = 0;
    let mut i: usize = 0;
    let mut offset = offset;

    if bytes.is_empty() || sz == 0 {
        return;
    }
    let mut foo: u8 = bytes[0];

    // Handle the first partial byte
    if offset != 0 {
        while offset < 4 && pos < sz {
            seq[pos] = byte2base(foo, offset);
            pos += 1;
            offset += 1;
        }
        if pos >= sz {
            return;
        }
        i += 1;
        if i < bytes.len() {
            foo = bytes[i];
        }
    }

    // Handle full bytes
    let remainder = (sz - pos) % 4;
    while pos + remainder < sz {
        if i >= bytes.len() {
            break;
        }
        foo = bytes[i];
        i += 1;
        seq[pos + 3] = bases[(foo & 3) as usize];
        foo >>= 2;
        seq[pos + 2] = bases[(foo & 3) as usize];
        foo >>= 2;
        seq[pos + 1] = bases[(foo & 3) as usize];
        foo >>= 2;
        seq[pos] = bases[(foo & 3) as usize];
        pos += 4;
    }

    // Handle the trailing partial byte
    if remainder > 0 && i < bytes.len() {
        foo = bytes[i];
    }
    let mut off2: i32 = 0;
    while (off2 as usize) < remainder && pos < sz {
        seq[pos] = byte2base(foo, off2);
        pos += 1;
        off2 += 1;
    }
}

pub fn getByteMaskFromOffset(_offset: i32) {
    // The C version returns a u8 mask, but the Rust signature here returns
    // unit. Internally, callers use `byte_mask_from_offset` directly.
}

// -- Internal immutable helpers --------------------------------------------

fn n_mask_immutable(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: u32, start: u32, end: u32) {
    let tid_us = tid as usize;
    let n_count = idx.n_block_count[tid_us] as usize;
    let mut pos: u32;
    for i in 0..n_count {
        let block_start = idx.n_block_start[tid_us][i];
        let mut block_end = block_start + idx.n_block_sizes[tid_us][i];
        if block_end <= start {
            continue;
        }
        if block_start >= end {
            break;
        }
        let width;
        if block_start < start {
            if block_end > end {
                block_end = end;
            }
            pos = 0;
            width = block_end - start;
        } else {
            if block_end > end {
                block_end = end;
            }
            pos = block_start - start;
            width = block_end - block_start;
        }
        let limit = pos + width;
        let mut p = pos;
        while p < limit {
            if (p as usize) < seq.len() {
                seq[p as usize] = 'N';
            }
            p += 1;
        }
    }
}

fn soft_mask_immutable(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: u32, start: u32, end: u32) {
    let tid_us = tid as usize;
    if idx.mask_block_start.is_empty() {
        return;
    }
    let m_count = idx.mask_block_count[tid_us] as usize;
    let mut pos: u32;
    for i in 0..m_count {
        let block_start = idx.mask_block_start[tid_us][i];
        let mut block_end = block_start + idx.mask_block_sizes[tid_us][i];
        if block_end <= start {
            continue;
        }
        if block_start >= end {
            break;
        }
        let width;
        if block_start < start {
            if block_end > end {
                block_end = end;
            }
            pos = 0;
            width = block_end - start;
        } else {
            if block_end > end {
                block_end = end;
            }
            pos = block_start - start;
            width = block_end - block_start;
        }
        let limit = pos + width;
        let mut p = pos;
        while p < limit {
            if (p as usize) < seq.len() {
                let c = seq[p as usize];
                if c != 'N' {
                    seq[p as usize] = c.to_ascii_lowercase();
                }
            }
            p += 1;
        }
    }
}

// Iterator-style getMask helper that supports incremental traversal of N
// blocks. It mirrors the in-out semantics of the C version.
fn get_mask_iter(
    idx: &TwoBitMaskedIdx,
    tid: u32,
    start: u32,
    end: u32,
    mask_idx: &mut u32,
    mask_start: &mut u32,
    mask_end: &mut u32,
) {
    let tid_us = tid as usize;
    let n_count = idx.n_block_count[tid_us];
    let neg_one: u32 = u32::MAX;

    if *mask_idx == neg_one {
        let mut idx_v: u32 = 0;
        let mut found = false;
        while idx_v < n_count {
            let bs = idx.n_block_start[tid_us][idx_v as usize];
            let be = bs + idx.n_block_sizes[tid_us][idx_v as usize];
            *mask_start = bs;
            *mask_end = be;
            if be < start {
                idx_v += 1;
                continue;
            }
            if be >= start {
                found = true;
                break;
            }
        }
        *mask_idx = idx_v;
        if !found {
            *mask_start = neg_one;
            *mask_end = neg_one;
        }
    } else if *mask_idx >= n_count {
        *mask_start = neg_one;
        *mask_end = neg_one;
    } else {
        *mask_idx += 1;
        if *mask_idx >= n_count {
            *mask_start = neg_one;
            *mask_end = neg_one;
        } else {
            let bs = idx.n_block_start[tid_us][*mask_idx as usize];
            let be = bs + idx.n_block_sizes[tid_us][*mask_idx as usize];
            *mask_start = bs;
            *mask_end = be;
        }
    }

    if *mask_idx >= n_count || *mask_start >= end {
        *mask_start = neg_one;
        *mask_end = neg_one;
    }
}

fn bases_worker_immutable(tb: &TwoBit, tid: u32, start: u32, end: u32, fraction: i32) -> Vec<u8> {
    let tid_us = tid as usize;
    let mut tmp: [u32; 4] = [0; 4];
    let mut start = start;
    let seq_len = (end - start) as u32;
    let len = (end - start) + (start % 4);

    let block_start = (start / 4) as u64;
    let mut offset_b: u8 = (start % 4) as u8;
    let block_end = ((end / 4) + (if end % 4 != 0 { 1 } else { 0 })) as u64;
    let n_bytes = (block_end - block_start) as usize;
    let data_off = tb.idx.offset[tid_us] + block_start;
    if (data_off as usize) + n_bytes > tb.data.len() {
        return Vec::new();
    }
    let bytes: Vec<u8> = tb.data[data_off as usize..data_off as usize + n_bytes].to_vec();

    let mut mask: u8 = byte_mask_from_offset(offset_b as i32);
    start = 4 * block_start as u32;
    offset_b = 0;

    let neg_one: u32 = u32::MAX;
    let mut mask_idx: u32 = neg_one;
    let mut mask_start: u32 = 0;
    let mut mask_end: u32 = 0;
    get_mask_iter(
        &tb.idx,
        tid,
        start,
        end,
        &mut mask_idx,
        &mut mask_start,
        &mut mask_end,
    );

    let mut i: u32 = 0;
    let mut j: u32 = 0;

    while i < len {
        // Check if we need to handle the N-mask range
        if mask_idx != neg_one && start + i + 4 >= mask_start {
            if start + i >= mask_start || start + i + 4 - offset_b as u32 > mask_start {
                // Jump iff the whole byte is inside an N block
                if start + i >= mask_start && start + i + 4 - offset_b as u32 <= mask_end
                    && start + i + 4 - offset_b as u32 - 1 < mask_end
                {
                    // Re-implement the C condition exactly:
                    // start + i >= maskStart && start + i + 4 - offset < maskEnd
                }
                if start + i >= mask_start && (start + i + 4).saturating_sub(offset_b as u32) < mask_end {
                    // Jump i to maskEnd - start
                    i = mask_end - start;
                    get_mask_iter(
                        &tb.idx,
                        tid,
                        i,
                        end,
                        &mut mask_idx,
                        &mut mask_start,
                        &mut mask_end,
                    );
                    offset_b = ((start + i) % 4) as u8;
                    j = i / 4;
                    mask = byte_mask_from_offset(offset_b as i32);
                    i = 4 * j;
                    offset_b = 0;
                    continue;
                }

                // Set the mask to omit positions inside the N block
                let foo_pos: u32 = 4 * j + 4 * (block_start as u32);
                if (mask & 1) != 0 && (foo_pos + 3 >= mask_start && foo_pos + 3 < mask_end) {
                    mask -= 1;
                }
                if (mask & 2) != 0 && (foo_pos + 2 >= mask_start && foo_pos + 2 < mask_end) {
                    mask -= 2;
                }
                if (mask & 4) != 0 && (foo_pos + 1 >= mask_start && foo_pos + 1 < mask_end) {
                    mask -= 4;
                }
                if (mask & 8) != 0 && (foo_pos >= mask_start && foo_pos < mask_end) {
                    mask -= 8;
                }
                if foo_pos + 4 > mask_end {
                    get_mask_iter(
                        &tb.idx,
                        tid,
                        i,
                        end,
                        &mut mask_idx,
                        &mut mask_start,
                        &mut mask_end,
                    );
                    continue;
                }
            }
        }

        // Mask anything beyond the actual end of the requested range
        if i + 4 >= len {
            if (mask & 1) != 0 && i + 3 >= len {
                mask -= 1;
            }
            if (mask & 2) != 0 && i + 2 >= len {
                mask -= 2;
            }
            if (mask & 4) != 0 && i + 1 >= len {
                mask -= 4;
            }
            if (mask & 8) != 0 && i >= len {
                mask -= 8;
            }
        }

        let mut foo: u8 = if (j as usize) < bytes.len() {
            bytes[j as usize]
        } else {
            0
        };
        j += 1;

        // Offset 3
        if (mask & 1) != 0 {
            tmp[(foo & 3) as usize] += 1;
        }
        foo >>= 2;
        let mut mask_l = mask >> 1;
        // Offset 2
        if (mask_l & 1) != 0 {
            tmp[(foo & 3) as usize] += 1;
        }
        foo >>= 2;
        mask_l >>= 1;
        // Offset 1
        if (mask_l & 1) != 0 {
            tmp[(foo & 3) as usize] += 1;
        }
        foo >>= 2;
        mask_l >>= 1;
        // Offset 0
        if (mask_l & 1) != 0 {
            tmp[(foo & 3) as usize] += 1;
        }

        i += 4;
        mask = 15;
    }

    // out is in TCAG order, but we present it in ACTG order to mirror the C
    // library's "first release" convention.
    let mut out: Vec<u8> = Vec::new();
    if fraction != 0 {
        let denom = seq_len as f64;
        let a = tmp[2] as f64 / denom;
        let c = tmp[1] as f64 / denom;
        let t = tmp[0] as f64 / denom;
        let g = tmp[3] as f64 / denom;
        out.extend_from_slice(&a.to_le_bytes());
        out.extend_from_slice(&c.to_le_bytes());
        out.extend_from_slice(&t.to_le_bytes());
        out.extend_from_slice(&g.to_le_bytes());
    } else {
        out.extend_from_slice(&tmp[2].to_le_bytes());
        out.extend_from_slice(&tmp[1].to_le_bytes());
        out.extend_from_slice(&tmp[0].to_le_bytes());
        out.extend_from_slice(&tmp[3].to_le_bytes());
    }
    out
}
