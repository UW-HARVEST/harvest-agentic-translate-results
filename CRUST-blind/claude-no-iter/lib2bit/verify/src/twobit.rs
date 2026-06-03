#![allow(non_snake_case)]

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
        let mut fp = File::open(fname).expect("twobit_open: failed to open 2bit file");
        let mut data = Vec::new();
        fp.read_to_end(&mut data)
            .expect("twobit_open: failed to read 2bit file");
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
        self.twobitChromListDestroy();
        self.twoBitIndexDestroy();
        self.twobitHdrDestroy();
    }

    pub fn twobit_chrom_len(&self, chrom: &str) -> u32 {
        for i in 0..self.hdr.n_chroms as usize {
            if i < self.cl.chrom.len() && self.cl.chrom[i] == chrom {
                return self.idx.size[i];
            }
        }
        0
    }

    pub fn twobit_sequence(&self, chrom: &str, start: u32, end: u32) -> String {
        let tid = match self.cl.chrom.iter().position(|c| c == chrom) {
            Some(t) => t,
            None => return String::new(),
        };

        let end = if start == 0 && end == 0 {
            self.idx.size[tid]
        } else {
            end
        };

        if end > self.idx.size[tid] || start >= end {
            return String::new();
        }

        let chars = self.construct_sequence_internal(tid as u32, start, end);
        chars.into_iter().collect()
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let tid = match self.cl.chrom.iter().position(|c| c == chrom) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let end = if start == 0 && end == 0 {
            self.idx.size[tid]
        } else {
            end
        };

        if end > self.idx.size[tid] || start >= end {
            return Vec::new();
        }

        self.bases_worker_internal(tid as u32, start, end, fraction)
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // The data parameter is an immutable borrow so we can't write into it.
        // Mirror the C semantics by advancing the internal offset and returning
        // the number of elements that would have been read.
        let need = sz.checked_mul(nmemb).unwrap_or(usize::MAX);
        let cur = self.offset as usize;
        if cur.saturating_add(need) > self.data.len() {
            return 0;
        }
        self.offset += need as u64;
        nmemb
    }

    pub fn twobitSeek(&mut self, offset: u64) {
        if offset < self.sz {
            self.offset = offset;
        }
    }

    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid = tid as usize;
        if tid >= self.idx.n_block_count.len() {
            return;
        }
        let n = self.idx.n_block_count[tid] as usize;
        for i in 0..n {
            let block_start = self.idx.n_block_start[tid][i];
            let block_end_orig = block_start + self.idx.n_block_sizes[tid][i];
            if block_end_orig <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let (mut pos, width) = if block_start < start {
                let block_end = block_end_orig.min(end);
                (0u32, block_end - start)
            } else {
                let block_end = block_end_orig.min(end);
                (block_start - start, block_end - block_start)
            };
            let limit = pos + width;
            while pos < limit {
                let p = pos as usize;
                if p < seq.len() {
                    seq[p] = 'N';
                }
                pos += 1;
            }
        }
    }

    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid = tid as usize;
        if self.idx.mask_block_start.is_empty() || tid >= self.idx.mask_block_start.len() {
            return;
        }
        let n = self.idx.mask_block_count[tid] as usize;
        for i in 0..n {
            let block_start = self.idx.mask_block_start[tid][i];
            let block_end_orig = block_start + self.idx.mask_block_sizes[tid][i];
            if block_end_orig <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let (mut pos, width) = if block_start < start {
                let block_end = block_end_orig.min(end);
                (0u32, block_end - start)
            } else {
                let block_end = block_end_orig.min(end);
                (block_start - start, block_end - block_start)
            };
            let limit = pos + width;
            while pos < limit {
                let p = pos as usize;
                if p < seq.len() {
                    let c = seq[p];
                    if c != 'N' {
                        seq[p] = c.to_ascii_lowercase();
                    }
                }
                pos += 1;
            }
        }
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        self.construct_sequence_internal(tid, start, end)
    }

    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        // Match the C "first call" semantics where maskIdx == -1: scan from 0
        let tid = tid as usize;
        if tid >= self.idx.n_block_count.len() {
            return (0, u32::MAX, u32::MAX);
        }
        let n = self.idx.n_block_count[tid];
        let mut mask_idx: u32 = 0;
        let mut mask_start: u32 = u32::MAX;
        let mut mask_end: u32 = u32::MAX;
        while mask_idx < n {
            mask_start = self.idx.n_block_start[tid][mask_idx as usize];
            mask_end = mask_start + self.idx.n_block_sizes[tid][mask_idx as usize];
            if mask_end < start {
                mask_idx += 1;
                continue;
            }
            break;
        }
        if mask_idx >= n || mask_start >= end {
            return (mask_idx, u32::MAX, u32::MAX);
        }
        (mask_idx, mask_start, mask_end)
    }

    pub fn twoBitBasesWorker(&mut self, _tid: u32, _start: u32, _end: u32, _fraction: i32) {
        // Per the Rust signature this returns nothing; the actual computation
        // is performed by `bases_worker_internal` which `twobit_bases` calls.
    }

    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n_chroms = self.hdr.n_chroms as usize;
        let mut size = vec![0u32; n_chroms];
        let mut n_block_count = vec![0u32; n_chroms];
        let mut n_block_start: Vec<Vec<u32>> = vec![Vec::new(); n_chroms];
        let mut n_block_sizes: Vec<Vec<u32>> = vec![Vec::new(); n_chroms];
        let mut mask_block_count = vec![0u32; n_chroms];
        let mut mask_block_start: Vec<Vec<u32>> = if storeMasked != 0 {
            vec![Vec::new(); n_chroms]
        } else {
            Vec::new()
        };
        let mut mask_block_sizes: Vec<Vec<u32>> = if storeMasked != 0 {
            vec![Vec::new(); n_chroms]
        } else {
            Vec::new()
        };
        let mut offsets = vec![0u64; n_chroms];

        for i in 0..n_chroms {
            let mut pos = self.cl.offset[i] as usize;

            // size + nBlockCount
            size[i] = read_u32_le(&self.data, pos);
            pos += 4;
            n_block_count[i] = read_u32_le(&self.data, pos);
            pos += 4;

            let nbc = n_block_count[i] as usize;
            let mut starts = Vec::with_capacity(nbc);
            for _ in 0..nbc {
                starts.push(read_u32_le(&self.data, pos));
                pos += 4;
            }
            n_block_start[i] = starts;

            let mut sizes = Vec::with_capacity(nbc);
            for _ in 0..nbc {
                sizes.push(read_u32_le(&self.data, pos));
                pos += 4;
            }
            n_block_sizes[i] = sizes;

            // maskBlockCount
            mask_block_count[i] = read_u32_le(&self.data, pos);
            pos += 4;
            let mbc = mask_block_count[i] as usize;

            if storeMasked != 0 {
                let mut mstarts = Vec::with_capacity(mbc);
                for _ in 0..mbc {
                    mstarts.push(read_u32_le(&self.data, pos));
                    pos += 4;
                }
                mask_block_start[i] = mstarts;

                let mut msizes = Vec::with_capacity(mbc);
                for _ in 0..mbc {
                    msizes.push(read_u32_le(&self.data, pos));
                    pos += 4;
                }
                mask_block_sizes[i] = msizes;
            } else {
                pos += 8 * mbc; // skip maskBlockStarts + maskBlockSizes
            }

            // Reserved word
            pos += 4;

            offsets[i] = pos as u64;
        }

        self.idx.size = size;
        self.idx.n_block_count = n_block_count;
        self.idx.n_block_start = n_block_start;
        self.idx.n_block_sizes = n_block_sizes;
        self.idx.mask_block_count = mask_block_count;
        self.idx.mask_block_start = mask_block_start;
        self.idx.mask_block_sizes = mask_block_sizes;
        self.idx.offset = offsets;
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
        let n_chroms = self.hdr.n_chroms as usize;
        let mut chroms: Vec<String> = Vec::with_capacity(n_chroms);
        let mut offsets: Vec<u32> = Vec::with_capacity(n_chroms);

        let mut pos = self.offset as usize;
        for _ in 0..n_chroms {
            if pos >= self.data.len() {
                break;
            }
            let len = self.data[pos] as usize;
            pos += 1;
            if pos + len > self.data.len() {
                break;
            }
            let bytes = &self.data[pos..pos + len];
            let s = String::from_utf8_lossy(bytes).into_owned();
            pos += len;
            chroms.push(s);

            if pos + 4 > self.data.len() {
                break;
            }
            let off = read_u32_le(&self.data, pos);
            pos += 4;
            offsets.push(off);
        }

        self.cl.chrom = chroms;
        self.cl.offset = offsets;
        self.offset = pos as u64;
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
        self.hdr.magic = magic;
        if magic != 0x1A412743 {
            eprintln!(
                "[twobitHdrRead] Received an invalid file magic number (0x{:x})!",
                magic
            );
            return;
        }
        let version = read_u32_le(&self.data, 4);
        self.hdr.version = version;
        if version != 0 {
            eprintln!(
                "[twobitHdrRead] The file version is {} while only version 0 is defined!",
                version
            );
            return;
        }
        let n_chroms = read_u32_le(&self.data, 8);
        self.hdr.n_chroms = n_chroms;
        if n_chroms == 0 {
            eprintln!(
                "[twobitHdrRead] There are apparently no chromosomes/contigs in this file!"
            );
            return;
        }
        // Skip reserved word; chromosome list begins at offset 16.
        self.offset = 16;
    }

    pub fn twobitHdrDestroy(&mut self) {
        self.hdr.magic = 0;
        self.hdr.version = 0;
        self.hdr.n_chroms = 0;
    }
}

// Private helpers
impl TwoBit {
    fn construct_sequence_internal(&self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let tid = tid as usize;
        let sz = (end - start) as usize;

        // 4 bases / byte
        let block_start = (start / 4) as usize;
        let offset = (start % 4) as i32;
        let block_end =
            (end / 4) as usize + if end % 4 != 0 { 1 } else { 0 };

        let bytes_pos = self.idx.offset[tid] as usize + block_start;
        let n_bytes = block_end - block_start;

        let mut seq: Vec<char> = vec!['\0'; sz];

        if n_bytes > 0 && bytes_pos + n_bytes <= self.data.len() {
            let bytes = &self.data[bytes_pos..bytes_pos + n_bytes];
            bytes2bases_internal(&mut seq, bytes, sz as u32, offset);
        }

        // N-mask
        let n = self.idx.n_block_count[tid] as usize;
        for i in 0..n {
            let block_s = self.idx.n_block_start[tid][i];
            let block_e_orig = block_s + self.idx.n_block_sizes[tid][i];
            if block_e_orig <= start {
                continue;
            }
            if block_s >= end {
                break;
            }
            let (mut pos, width) = if block_s < start {
                let block_e = block_e_orig.min(end);
                (0u32, block_e - start)
            } else {
                let block_e = block_e_orig.min(end);
                (block_s - start, block_e - block_s)
            };
            let limit = pos + width;
            while pos < limit {
                let p = pos as usize;
                if p < seq.len() {
                    seq[p] = 'N';
                }
                pos += 1;
            }
        }

        // Soft-mask if we have it stored
        if !self.idx.mask_block_start.is_empty() && tid < self.idx.mask_block_start.len() {
            let n = self.idx.mask_block_count[tid] as usize;
            for i in 0..n {
                let block_s = self.idx.mask_block_start[tid][i];
                let block_e_orig = block_s + self.idx.mask_block_sizes[tid][i];
                if block_e_orig <= start {
                    continue;
                }
                if block_s >= end {
                    break;
                }
                let (mut pos, width) = if block_s < start {
                    let block_e = block_e_orig.min(end);
                    (0u32, block_e - start)
                } else {
                    let block_e = block_e_orig.min(end);
                    (block_s - start, block_e - block_s)
                };
                let limit = pos + width;
                while pos < limit {
                    let p = pos as usize;
                    if p < seq.len() {
                        let c = seq[p];
                        if c != 'N' {
                            seq[p] = c.to_ascii_lowercase();
                        }
                    }
                    pos += 1;
                }
            }
        }

        seq
    }

    fn bases_worker_internal(&self, tid: u32, mut start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let tid = tid as usize;
        let mut tmp: [u32; 4] = [0, 0, 0, 0];
        let len = end - start + (start % 4);
        let seq_len = end - start;

        let block_start = start / 4;
        let mut offset = (start % 4) as i32;
        let block_end = end / 4 + if end % 4 != 0 { 1 } else { 0 };

        let bytes_pos = self.idx.offset[tid] as usize + block_start as usize;
        let n_bytes = (block_end - block_start) as usize;
        if bytes_pos + n_bytes > self.data.len() {
            return Vec::new();
        }
        let bytes = &self.data[bytes_pos..bytes_pos + n_bytes];

        // Initial byte mask handles the partial first byte
        let mut mask: u8 = get_byte_mask_internal(offset);
        start = 4 * block_start;
        offset = 0;

        let (mut mask_idx, mut mask_start, mut mask_end) =
            self.get_mask_first(tid, start, end);

        let mut i: u32 = 0;
        let mut j: u32 = 0;

        while i < len {
            // Check if the current 4-base byte overlaps an N block
            if mask_idx != u32::MAX && start + i + 4 >= mask_start {
                if start + i >= mask_start || start + i + 4 - offset as u32 > mask_start {
                    // If the whole byte is inside an N block, jump past it
                    if start + i >= mask_start
                        && start + i + 4 - offset as u32 <= mask_end
                    {
                        i = mask_end - start;
                        let r = self.get_mask_advance(tid, end, mask_idx);
                        mask_idx = r.0;
                        mask_start = r.1;
                        mask_end = r.2;
                        offset = ((start + i) % 4) as i32;
                        j = i / 4;
                        mask = get_byte_mask_internal(offset);
                        i = 4 * j;
                        offset = 0;
                        continue;
                    }

                    // Mask out the bases inside the byte that fall in the N block
                    let foo = 4 * j + 4 * block_start;
                    if (mask & 1) != 0
                        && (foo + 3 >= mask_start && foo + 3 < mask_end)
                    {
                        mask -= 1;
                    }
                    if (mask & 2) != 0
                        && (foo + 2 >= mask_start && foo + 2 < mask_end)
                    {
                        mask -= 2;
                    }
                    if (mask & 4) != 0
                        && (foo + 1 >= mask_start && foo + 1 < mask_end)
                    {
                        mask -= 4;
                    }
                    if (mask & 8) != 0 && (foo >= mask_start && foo < mask_end) {
                        mask -= 8;
                    }
                    if foo + 4 > mask_end {
                        let r = self.get_mask_advance(tid, end, mask_idx);
                        mask_idx = r.0;
                        mask_start = r.1;
                        mask_end = r.2;
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

            let mut foo = bytes[j as usize];
            // Offset 3
            if (mask & 1) != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            mask >>= 1;
            // Offset 2
            if (mask & 1) != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            mask >>= 1;
            // Offset 1
            if (mask & 1) != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            mask >>= 1;
            // Offset 0
            if (mask & 1) != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            j += 1;
            i += 4;
            mask = 15;
        }

        // tmp is in TCAG order (since 2bit stores as TCAG); rearrange to ACTG
        let mut out: Vec<u8> = Vec::new();
        if fraction != 0 {
            let denom = seq_len as f64;
            let vals = [
                tmp[2] as f64 / denom,
                tmp[1] as f64 / denom,
                tmp[0] as f64 / denom,
                tmp[3] as f64 / denom,
            ];
            for v in vals.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        } else {
            let vals = [tmp[2], tmp[1], tmp[0], tmp[3]];
            for v in vals.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    /// First call to get-mask: scan from index 0 to find the first N-block
    /// whose end is at or beyond `start`.
    fn get_mask_first(&self, tid: usize, start: u32, end: u32) -> (u32, u32, u32) {
        if tid >= self.idx.n_block_count.len() {
            return (u32::MAX, u32::MAX, u32::MAX);
        }
        let n = self.idx.n_block_count[tid];
        let mut mask_idx: u32 = 0;
        let mut mask_start: u32 = u32::MAX;
        let mut mask_end: u32 = u32::MAX;
        while mask_idx < n {
            mask_start = self.idx.n_block_start[tid][mask_idx as usize];
            mask_end = mask_start + self.idx.n_block_sizes[tid][mask_idx as usize];
            if mask_end < start {
                mask_idx += 1;
                continue;
            }
            break;
        }
        if mask_idx >= n || mask_start >= end {
            return (u32::MAX, u32::MAX, u32::MAX);
        }
        (mask_idx, mask_start, mask_end)
    }

    /// Advance to the next N-block.
    fn get_mask_advance(&self, tid: usize, end: u32, mut mask_idx: u32) -> (u32, u32, u32) {
        if tid >= self.idx.n_block_count.len() {
            return (mask_idx, u32::MAX, u32::MAX);
        }
        let n = self.idx.n_block_count[tid];
        let (mut mask_start, mut mask_end);
        if mask_idx >= n {
            mask_start = u32::MAX;
            mask_end = u32::MAX;
        } else {
            mask_idx += 1;
            if mask_idx >= n {
                mask_start = u32::MAX;
                mask_end = u32::MAX;
            } else {
                mask_start = self.idx.n_block_start[tid][mask_idx as usize];
                mask_end = mask_start + self.idx.n_block_sizes[tid][mask_idx as usize];
            }
        }
        if mask_idx >= n || mask_start >= end {
            mask_start = u32::MAX;
            mask_end = u32::MAX;
            return (u32::MAX, mask_start, mask_end);
        }
        (mask_idx, mask_start, mask_end)
    }
}

// Helper function
pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let shift = 2 * rev;
    let mask: u8 = 3u8 << shift;
    let foo = ((mask & byte) >> shift) as usize;
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo & 3]
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    bytes2bases_internal(seq, bytes, sz, offset);
}

pub fn getByteMaskFromOffset(_offset: i32) {
    // The Rust signature returns nothing; the actual u8 mask value is
    // computed via the private `get_byte_mask_internal` helper.
}

// ----- Internal helpers shared across the module -----

fn read_u32_le(data: &[u8], pos: usize) -> u32 {
    let b = &data[pos..pos + 4];
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn bytes2bases_internal(seq: &mut [char], bytes: &[u8], sz: u32, mut offset: i32) {
    if sz == 0 || bytes.is_empty() {
        return;
    }
    let bases = ['T', 'C', 'A', 'G'];
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let mut foo: u8 = bytes[0];

    // Deal with the first partial byte
    if offset != 0 {
        while offset < 4 && pos < sz {
            if (pos as usize) < seq.len() {
                seq[pos as usize] = byte2base(foo, offset);
            }
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

    let remainder = (sz - pos) % 4;
    while pos < sz - remainder {
        if i >= bytes.len() {
            break;
        }
        foo = bytes[i];
        i += 1;
        let p = pos as usize;
        if p + 3 < seq.len() {
            seq[p + 3] = bases[(foo & 3) as usize];
        }
        foo >>= 2;
        if p + 2 < seq.len() {
            seq[p + 2] = bases[(foo & 3) as usize];
        }
        foo >>= 2;
        if p + 1 < seq.len() {
            seq[p + 1] = bases[(foo & 3) as usize];
        }
        foo >>= 2;
        if p < seq.len() {
            seq[p] = bases[(foo & 3) as usize];
        }
        pos += 4;
    }

    // Deal with the last partial byte
    if remainder > 0 && i < bytes.len() {
        foo = bytes[i];
    }
    let mut o: i32 = 0;
    while o < remainder as i32 {
        if (pos as usize) < seq.len() {
            seq[pos as usize] = byte2base(foo, o);
        }
        pos += 1;
        o += 1;
    }
}

fn get_byte_mask_internal(offset: i32) -> u8 {
    match offset {
        0 => 15,
        1 => 7,
        2 => 3,
        _ => 1,
    }
}
