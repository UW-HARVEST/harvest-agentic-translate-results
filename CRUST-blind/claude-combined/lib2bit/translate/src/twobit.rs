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

impl TwoBit {
    pub fn twobit_open(fname: &str, store_masked: bool) -> Self {
        let mut fp = File::open(fname).expect("failed to open 2bit file");
        let mut data: Vec<u8> = Vec::new();
        fp.read_to_end(&mut data).expect("failed to read 2bit file");
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
        if tb.hdr.magic != 0x1A412743 {
            panic!("Invalid 2bit file magic: 0x{:x}", tb.hdr.magic);
        }
        if tb.hdr.version != 0 {
            panic!("Unsupported 2bit version: {}", tb.hdr.version);
        }
        if tb.hdr.n_chroms == 0 {
            panic!("No chromosomes in file");
        }

        tb.twobitChromListRead();
        tb.twoBitIndexRead(if store_masked { 1 } else { 0 });

        tb
    }

    pub fn twobit_close(&mut self) {
        self.twobitChromListDestroy();
        self.twoBitIndexDestroy();
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
        let n = self.hdr.n_chroms as usize;
        let mut tid_opt: Option<usize> = None;
        for i in 0..n {
            if self.cl.chrom[i] == chrom {
                tid_opt = Some(i);
                break;
            }
        }
        let tid = match tid_opt {
            Some(t) => t,
            None => return String::new(),
        };

        let mut start = start;
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

        // We need a mutable borrow for construct_sequence — clone what we need.
        // But construct_sequence is &mut self; here we have &self. So inline-implement it.
        let seq_chars = construct_sequence_immut(self, tid as u32, start, end);
        seq_chars.iter().collect()
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let n = self.hdr.n_chroms as usize;
        let mut tid_opt: Option<usize> = None;
        for i in 0..n {
            if self.cl.chrom[i] == chrom {
                tid_opt = Some(i);
                break;
            }
        }
        let tid = match tid_opt {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut start = start;
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

        bases_worker_immut(self, tid as u32, start, end, fraction)
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // The 2bit C code uses a memory-mapped buffer; the Rust `data` parameter
        // is immutable so this implementation only advances the internal offset
        // by `sz * nmemb` and reports success. Internal helpers read directly
        // from `self.data` at `self.offset`.
        let total = (sz as u64) * (nmemb as u64);
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
        n_mask_impl(self, seq, tid, start, end);
    }

    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        soft_mask_impl(self, seq, tid, start, end);
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        construct_sequence_immut(self, tid, start, end)
    }

    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        // Return (maskIdx, maskStart, maskEnd) for the first overlapping
        // N-mask block (when called with maskIdx == -1 sentinel in C).
        let mut mask_idx: u32 = u32::MAX;
        let mut mask_start: u32 = 0;
        let mut mask_end: u32 = 0;
        get_mask_impl(self, tid, start, end, &mut mask_idx, &mut mask_start, &mut mask_end);
        (mask_idx, mask_start, mask_end)
    }

    pub fn twoBitBasesWorker(&mut self, _tid: u32, _start: u32, _end: u32, _fraction: i32) {
        // The signature has no return value here, so we can only mutate state.
        // The actual computation is performed in `twobit_bases` via
        // `bases_worker_immut` which serializes the result.
    }

    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n = self.hdr.n_chroms as usize;
        let mut size: Vec<u32> = Vec::with_capacity(n);
        let mut n_block_count: Vec<u32> = vec![0; n];
        let mut n_block_start: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut n_block_sizes: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut mask_block_count: Vec<u32> = vec![0; n];
        let mut mask_block_start: Vec<Vec<u32>> = if storeMasked != 0 { vec![Vec::new(); n] } else { Vec::new() };
        let mut mask_block_sizes: Vec<Vec<u32>> = if storeMasked != 0 { vec![Vec::new(); n] } else { Vec::new() };
        let mut offsets: Vec<u64> = Vec::with_capacity(n);

        for i in 0..n {
            self.offset = self.cl.offset[i] as u64;
            let chrom_size = self.read_u32_le();
            let nbc = self.read_u32_le();
            size.push(chrom_size);
            n_block_count[i] = nbc;

            let mut nbs: Vec<u32> = Vec::with_capacity(nbc as usize);
            for _ in 0..nbc {
                nbs.push(self.read_u32_le());
            }
            let mut nbsizes: Vec<u32> = Vec::with_capacity(nbc as usize);
            for _ in 0..nbc {
                nbsizes.push(self.read_u32_le());
            }
            n_block_start[i] = nbs;
            n_block_sizes[i] = nbsizes;

            let mbc = self.read_u32_le();
            mask_block_count[i] = mbc;

            if storeMasked != 0 {
                let mut mbs: Vec<u32> = Vec::with_capacity(mbc as usize);
                for _ in 0..mbc {
                    mbs.push(self.read_u32_le());
                }
                let mut mbsizes: Vec<u32> = Vec::with_capacity(mbc as usize);
                for _ in 0..mbc {
                    mbsizes.push(self.read_u32_le());
                }
                mask_block_start[i] = mbs;
                mask_block_sizes[i] = mbsizes;
            } else {
                // Skip 8 bytes per masked block (start + size)
                self.offset += 8u64 * (mbc as u64);
            }

            // Reserved
            let _reserved = self.read_u32_le();
            offsets.push(self.offset);
        }

        self.idx = TwoBitMaskedIdx {
            size,
            n_block_count,
            n_block_start,
            n_block_sizes,
            mask_block_count,
            mask_block_start,
            mask_block_sizes,
            offset: offsets,
        };
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
        let mut offset: Vec<u32> = Vec::with_capacity(n);

        for _ in 0..n {
            let len = self.read_u8() as usize;
            let bytes = self.read_bytes(len);
            let s = String::from_utf8(bytes).expect("invalid UTF-8 in chrom name");
            chrom.push(s);
            let off = self.read_u32_le();
            offset.push(off);
        }

        self.cl = TwoBitCL { chrom, offset };
    }

    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }

    pub fn twobitHdrRead(&mut self) {
        // Read 16 bytes (4 u32s)
        let magic = self.read_u32_le();
        let version = self.read_u32_le();
        let n_chroms = self.read_u32_le();
        let _reserved = self.read_u32_le();

        self.hdr = TwoBitHeader {
            magic,
            version,
            n_chroms,
        };
    }

    pub fn twobitHdrDestroy(&mut self) {
        self.hdr.magic = 0;
        self.hdr.version = 0;
        self.hdr.n_chroms = 0;
    }
}

// --- internal helpers ---
impl TwoBit {
    fn read_u8(&mut self) -> u8 {
        let b = self.data[self.offset as usize];
        self.offset += 1;
        b
    }

    fn read_u32_le(&mut self) -> u32 {
        let off = self.offset as usize;
        let bytes: [u8; 4] = self.data[off..off + 4].try_into().unwrap();
        self.offset += 4;
        u32::from_le_bytes(bytes)
    }

    fn read_bytes(&mut self, n: usize) -> Vec<u8> {
        let off = self.offset as usize;
        let v = self.data[off..off + n].to_vec();
        self.offset += n as u64;
        v
    }
}

// --- Functions that need only &TwoBit ---

fn construct_sequence_immut(tb: &TwoBit, tid: u32, start: u32, end: u32) -> Vec<char> {
    let sz = (end - start + 1) as usize;
    let block_start = (start / 4) as u64;
    let offset = (start % 4) as i32;
    let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as u64;

    let nbytes = (block_end - block_start) as usize;
    let file_off = tb.idx.offset[tid as usize] + block_start;
    if (file_off as usize) + nbytes > tb.data.len() {
        return Vec::new();
    }
    let bytes = tb.data[file_off as usize..file_off as usize + nbytes].to_vec();

    let mut seq: Vec<char> = vec!['\0'; sz];
    // Fill (sz - 1) characters of sequence (last is the null terminator analog)
    let seq_len_actual = sz - 1;
    bytes2bases_internal(&mut seq[..seq_len_actual], &bytes, seq_len_actual as u32, offset);

    // null terminator (kept for parity with C output but stripped before return)
    seq[sz - 1] = '\0';

    // N-mask
    n_mask_immut(tb, &mut seq[..seq_len_actual], tid, start, end);

    // Soft-mask if mask blocks were stored
    if !tb.idx.mask_block_start.is_empty() {
        soft_mask_immut(tb, &mut seq[..seq_len_actual], tid, start, end);
    }

    // Drop the trailing null terminator from the returned vector
    seq.truncate(seq_len_actual);
    seq
}

fn n_mask_immut(tb: &TwoBit, seq: &mut [char], tid: u32, start: u32, end: u32) {
    let tid = tid as usize;
    for i in 0..tb.idx.n_block_count[tid] as usize {
        let block_start = tb.idx.n_block_start[tid][i];
        let mut block_end = block_start + tb.idx.n_block_sizes[tid][i];
        if block_end <= start {
            continue;
        }
        if block_start >= end {
            break;
        }
        let pos: u32;
        let width: u32;
        if block_start < start {
            block_end = if block_end < end { block_end } else { end };
            pos = 0;
            width = block_end - start;
        } else {
            block_end = if block_end < end { block_end } else { end };
            pos = block_start - start;
            width = block_end - block_start;
        }
        let total = pos + width;
        let mut p = pos as usize;
        while p < total as usize && p < seq.len() {
            seq[p] = 'N';
            p += 1;
        }
    }
}

fn soft_mask_immut(tb: &TwoBit, seq: &mut [char], tid: u32, start: u32, end: u32) {
    let tid = tid as usize;
    if tb.idx.mask_block_start.is_empty() {
        return;
    }
    for i in 0..tb.idx.mask_block_count[tid] as usize {
        let block_start = tb.idx.mask_block_start[tid][i];
        let mut block_end = block_start + tb.idx.mask_block_sizes[tid][i];
        if block_end <= start {
            continue;
        }
        if block_start >= end {
            break;
        }
        let pos: u32;
        let width: u32;
        if block_start < start {
            block_end = if block_end < end { block_end } else { end };
            pos = 0;
            width = block_end - start;
        } else {
            block_end = if block_end < end { block_end } else { end };
            pos = block_start - start;
            width = block_end - block_start;
        }
        let total = pos + width;
        let mut p = pos as usize;
        while p < total as usize && p < seq.len() {
            if seq[p] != 'N' {
                seq[p] = seq[p].to_ascii_lowercase();
            }
            p += 1;
        }
    }
}

fn n_mask_impl(tb: &TwoBit, seq: &mut [char], tid: u32, start: u32, end: u32) {
    n_mask_immut(tb, seq, tid, start, end);
}
fn soft_mask_impl(tb: &TwoBit, seq: &mut [char], tid: u32, start: u32, end: u32) {
    soft_mask_immut(tb, seq, tid, start, end);
}

fn get_mask_impl(
    tb: &TwoBit,
    tid: u32,
    start: u32,
    end: u32,
    mask_idx: &mut u32,
    mask_start: &mut u32,
    mask_end: &mut u32,
) {
    let tid = tid as usize;
    let nbc = tb.idx.n_block_count[tid];
    if *mask_idx == u32::MAX {
        *mask_idx = 0;
        while *mask_idx < nbc {
            *mask_start = tb.idx.n_block_start[tid][*mask_idx as usize];
            *mask_end = *mask_start + tb.idx.n_block_sizes[tid][*mask_idx as usize];
            if *mask_end < start {
                *mask_idx += 1;
                continue;
            }
            if *mask_end >= start {
                break;
            }
        }
    } else if *mask_idx >= nbc {
        *mask_start = u32::MAX;
        *mask_end = u32::MAX;
    } else {
        *mask_idx += 1;
        if *mask_idx >= nbc {
            *mask_start = u32::MAX;
            *mask_end = u32::MAX;
        } else {
            *mask_start = tb.idx.n_block_start[tid][*mask_idx as usize];
            *mask_end = *mask_start + tb.idx.n_block_sizes[tid][*mask_idx as usize];
        }
    }

    if *mask_idx >= nbc || *mask_start >= end {
        *mask_start = u32::MAX;
        *mask_end = u32::MAX;
    }
}

fn bases_worker_immut(tb: &TwoBit, tid: u32, start: u32, end: u32, fraction: i32) -> Vec<u8> {
    let mut tmp: [u32; 4] = [0, 0, 0, 0];
    let len: u32 = end - start + (start % 4);
    let seq_len: u32 = end - start;

    let block_start = (start / 4) as u64;
    let offset_init = (start % 4) as i32;
    let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as u64;
    let nbytes = (block_end - block_start) as usize;
    let file_off = tb.idx.offset[tid as usize] + block_start;
    if (file_off as usize) + nbytes > tb.data.len() {
        return Vec::new();
    }
    let bytes = &tb.data[file_off as usize..file_off as usize + nbytes];

    // Set initial mask, reset start so we always deal with full bytes
    let mut mask: u8 = get_byte_mask_from_offset(offset_init);
    let cur_start = (4 * block_start) as u32;
    let mut offset_running: u32 = 0;
    let _ = offset_init;

    // Get the initial N-mask block
    let mut mask_idx: u32 = u32::MAX;
    let mut mask_start: u32 = 0;
    let mut mask_end_b: u32 = 0;
    get_mask_impl(tb, tid, cur_start, end, &mut mask_idx, &mut mask_start, &mut mask_end_b);

    let mut i: u32 = 0;
    let mut j: u32 = 0;
    while i < len {
        // Check if we need to jump
        if mask_idx != u32::MAX && cur_start + i + 4 >= mask_start {
            if cur_start + i >= mask_start || cur_start + i + 4 - offset_running > mask_start {
                // Jump iff the whole byte is inside an N block
                if cur_start + i >= mask_start && cur_start + i + 4 - offset_running < mask_end_b {
                    i = mask_end_b - cur_start;
                    get_mask_impl(tb, tid, i, end, &mut mask_idx, &mut mask_start, &mut mask_end_b);
                    offset_running = (cur_start + i) % 4;
                    j = i / 4;
                    mask = get_byte_mask_from_offset(offset_running as i32);
                    i = 4 * j;
                    offset_running = 0;
                    continue;
                }

                // Set the mask, if appropriate
                let foo_pos: u32 = 4 * j + 4 * (block_start as u32);
                if (mask & 1) != 0 && (foo_pos + 3 >= mask_start && foo_pos + 3 < mask_end_b) {
                    mask -= 1;
                }
                if (mask & 2) != 0 && (foo_pos + 2 >= mask_start && foo_pos + 2 < mask_end_b) {
                    mask -= 2;
                }
                if (mask & 4) != 0 && (foo_pos + 1 >= mask_start && foo_pos + 1 < mask_end_b) {
                    mask -= 4;
                }
                if (mask & 8) != 0 && (foo_pos >= mask_start && foo_pos < mask_end_b) {
                    mask -= 8;
                }
                if foo_pos + 4 > mask_end_b {
                    get_mask_impl(tb, tid, i, end, &mut mask_idx, &mut mask_start, &mut mask_end_b);
                    continue;
                }
            }
        }

        // Mask anything past the end
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

        let mut foo: u8 = bytes[j as usize];
        j += 1;
        // Offset 3
        if (mask & 1) != 0 {
            tmp[(foo & 3) as usize] += 1;
        }
        foo >>= 2;
        let mut m = mask >> 1;
        // Offset 2
        if (m & 1) != 0 {
            tmp[(foo & 3) as usize] += 1;
        }
        foo >>= 2;
        m >>= 1;
        // Offset 1
        if (m & 1) != 0 {
            tmp[(foo & 3) as usize] += 1;
        }
        foo >>= 2;
        m >>= 1;
        // Offset 0
        if (m & 1) != 0 {
            tmp[(foo & 3) as usize] += 1;
        }
        i += 4;
        mask = 15;
    }

    let mut out: Vec<u8> = Vec::new();
    if fraction != 0 {
        let denom = seq_len as f64;
        let a: f64 = tmp[2] as f64 / denom;
        let c: f64 = tmp[1] as f64 / denom;
        let t: f64 = tmp[0] as f64 / denom;
        let g: f64 = tmp[3] as f64 / denom;
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

fn bytes2bases_internal(seq: &mut [char], byte: &[u8], sz: u32, offset: i32) {
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let bases: [char; 4] = ['T', 'C', 'A', 'G'];
    let mut foo: u8 = byte[0];
    let mut offset = offset;

    // Deal with first partial byte
    if offset != 0 {
        while offset < 4 && pos < sz {
            seq[pos as usize] = byte2base(foo, offset);
            pos += 1;
            offset += 1;
        }
        if pos >= sz {
            return;
        }
        i += 1;
        foo = byte[i];
    }

    let remainder = (sz - pos) % 4;
    while pos < sz - remainder {
        foo = byte[i];
        i += 1;
        seq[(pos + 3) as usize] = bases[(foo & 3) as usize];
        foo >>= 2;
        seq[(pos + 2) as usize] = bases[(foo & 3) as usize];
        foo >>= 2;
        seq[(pos + 1) as usize] = bases[(foo & 3) as usize];
        foo >>= 2;
        seq[pos as usize] = bases[(foo & 3) as usize];
        pos += 4;
    }

    if remainder > 0 {
        foo = byte[i];
    }
    let mut o: i32 = 0;
    while o < remainder as i32 {
        seq[pos as usize] = byte2base(foo, o);
        pos += 1;
        o += 1;
    }
}

fn get_byte_mask_from_offset(offset: i32) -> u8 {
    match offset {
        0 => 15,
        1 => 7,
        2 => 3,
        _ => 1,
    }
}

// Helper function
pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let mask: u8 = 3u8 << (2 * rev);
    let foo = (mask & byte) >> (2 * rev);
    let bases: [char; 4] = ['T', 'C', 'A', 'G'];
    bases[foo as usize]
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    bytes2bases_internal(seq, bytes, sz, offset);
}

pub fn getByteMaskFromOffset(_offset: i32) {
    // Public signature returns nothing per the project's Rust API.
}
