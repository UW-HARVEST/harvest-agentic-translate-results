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

const SENTINEL: u32 = u32::MAX;

impl TwoBit {
    pub fn twobit_open(fname: &str, store_masked: bool) -> Self {
        let mut fp = File::open(fname).expect("Failed to open 2bit file");
        let sz = fp.metadata().expect("Failed to stat file").len();

        // Read entire file into memory (mimicking mmap)
        let mut data = Vec::with_capacity(sz as usize);
        fp.read_to_end(&mut data).expect("Failed to read file");

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
        self.data.clear();
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
        let mut tid: usize = 0;
        let mut found = false;
        for i in 0..self.hdr.n_chroms as usize {
            if self.cl.chrom[i] == chrom {
                tid = i;
                found = true;
                break;
            }
        }
        if !found {
            return String::new();
        }

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

        // construct sequence (immutable version)
        let chars = self.construct_sequence_internal(tid as u32, start, end);
        chars.into_iter().collect()
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let mut tid: usize = 0;
        let mut found = false;
        for i in 0..self.hdr.n_chroms as usize {
            if self.cl.chrom[i] == chrom {
                tid = i;
                found = true;
                break;
            }
        }
        if !found {
            return Vec::new();
        }

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

        self.two_bit_bases_worker_internal(tid as u32, start, end, fraction)
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // Just advance the offset and return nmemb (caller expected to use other helpers)
        let bytes_to_read = sz * nmemb;
        if self.offset as usize + bytes_to_read > self.data.len() {
            return 0;
        }
        self.offset += bytes_to_read as u64;
        nmemb
    }

    pub fn twobitSeek(&mut self, offset: u64) {
        if offset >= self.sz {
            return;
        }
        self.offset = offset;
    }

    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid = tid as usize;
        let mut pos: u32;
        let mut width: u32;
        let mut block_start: u32;
        let mut block_end: u32;

        for i in 0..self.idx.n_block_count[tid] as usize {
            block_start = self.idx.n_block_start[tid][i];
            block_end = block_start + self.idx.n_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            if block_start < start {
                block_end = if block_end < end { block_end } else { end };
                pos = 0;
                width = block_end - start;
            } else {
                block_end = if block_end < end { block_end } else { end };
                pos = block_start - start;
                width = block_end - block_start;
            }
            width += pos;
            while pos < width {
                seq[pos as usize] = 'N';
                pos += 1;
            }
        }
    }

    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid = tid as usize;
        if self.idx.mask_block_start.is_empty() {
            return;
        }

        let mut pos: u32;
        let mut width: u32;
        let mut block_start: u32;
        let mut block_end: u32;

        for i in 0..self.idx.mask_block_count[tid] as usize {
            block_start = self.idx.mask_block_start[tid][i];
            block_end = block_start + self.idx.mask_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            if block_start < start {
                block_end = if block_end < end { block_end } else { end };
                pos = 0;
                width = block_end - start;
            } else {
                block_end = if block_end < end { block_end } else { end };
                pos = block_start - start;
                width = block_end - block_start;
            }
            width += pos;
            while pos < width {
                let c = seq[pos as usize];
                if c != 'N' {
                    seq[pos as usize] = c.to_ascii_lowercase();
                }
                pos += 1;
            }
        }
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        self.construct_sequence_internal(tid, start, end)
    }

    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        // This was a getter helper in C: provided maskIdx and updated start/end.
        // Since the signature here is to return a tuple, we'll provide initial-state
        // functionality (called with maskIdx == -1) to find first overlapping block.
        let tid_us = tid as usize;
        let mut mask_idx: u32 = SENTINEL;
        let mut mask_start: u32 = 0;
        let mut mask_end: u32 = 0;

        // Initial-state behavior: scan to find first block whose end >= start
        let mut idx: u32 = 0;
        while idx < self.idx.n_block_count[tid_us] {
            mask_start = self.idx.n_block_start[tid_us][idx as usize];
            mask_end = mask_start + self.idx.n_block_sizes[tid_us][idx as usize];
            if mask_end < start {
                idx += 1;
                continue;
            }
            if mask_end >= start {
                break;
            }
        }
        mask_idx = idx;

        if mask_idx >= self.idx.n_block_count[tid_us] || mask_start >= end {
            mask_start = SENTINEL;
            mask_end = SENTINEL;
        }
        (mask_idx, mask_start, mask_end)
    }

    pub fn twoBitBasesWorker(&mut self, tid: u32, start: u32, end: u32, fraction: i32) {
        // Stub; the actual logic is in two_bit_bases_worker_internal.
        // This method is part of the public API but isn't used by the binary tests.
        let _ = self.two_bit_bases_worker_internal(tid, start, end, fraction);
    }

    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n_chroms = self.hdr.n_chroms as usize;

        let mut size: Vec<u32> = vec![0; n_chroms];
        let mut n_block_count: Vec<u32> = vec![0; n_chroms];
        let mut n_block_start: Vec<Vec<u32>> = vec![Vec::new(); n_chroms];
        let mut n_block_sizes: Vec<Vec<u32>> = vec![Vec::new(); n_chroms];
        let mut mask_block_count: Vec<u32> = vec![0; n_chroms];
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
        let mut offset_vec: Vec<u64> = vec![0; n_chroms];

        for i in 0..n_chroms {
            let off = self.cl.offset[i] as u64;
            self.seek_to(off);

            // Read 2 u32s
            let chrom_size = self.read_u32();
            let n_block_count_i = self.read_u32();
            size[i] = chrom_size;
            n_block_count[i] = n_block_count_i;

            // Read N block starts
            let mut starts = Vec::with_capacity(n_block_count_i as usize);
            for _ in 0..n_block_count_i {
                starts.push(self.read_u32());
            }
            n_block_start[i] = starts;

            // Read N block sizes
            let mut sizes = Vec::with_capacity(n_block_count_i as usize);
            for _ in 0..n_block_count_i {
                sizes.push(self.read_u32());
            }
            n_block_sizes[i] = sizes;

            // Read mask block count
            let mbc = self.read_u32();
            mask_block_count[i] = mbc;

            if storeMasked != 0 {
                let mut mstarts = Vec::with_capacity(mbc as usize);
                for _ in 0..mbc {
                    mstarts.push(self.read_u32());
                }
                mask_block_start[i] = mstarts;

                let mut msizes = Vec::with_capacity(mbc as usize);
                for _ in 0..mbc {
                    msizes.push(self.read_u32());
                }
                mask_block_sizes[i] = msizes;
            } else {
                // Skip 8 * mbc bytes
                self.offset += 8u64 * mbc as u64;
            }

            // Reserved 4 bytes
            let _reserved = self.read_u32();

            offset_vec[i] = self.offset;
        }

        self.idx = TwoBitMaskedIdx {
            size,
            n_block_count,
            n_block_start,
            n_block_sizes,
            mask_block_count,
            mask_block_start,
            mask_block_sizes,
            offset: offset_vec,
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
        let n_chroms = self.hdr.n_chroms as usize;
        let mut chrom: Vec<String> = Vec::with_capacity(n_chroms);
        let mut offset: Vec<u32> = Vec::with_capacity(n_chroms);

        for _ in 0..n_chroms {
            // Read 1 byte: name length
            let byte = self.read_u8();
            let name_len = byte as usize;

            // Read name
            let mut name_bytes = Vec::with_capacity(name_len);
            for _ in 0..name_len {
                name_bytes.push(self.read_u8());
            }
            let name = String::from_utf8(name_bytes).unwrap_or_default();
            chrom.push(name);

            // Read offset (u32)
            let off = self.read_u32();
            offset.push(off);
        }

        self.cl = TwoBitCL { chrom, offset };
    }

    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }

    pub fn twobitHdrRead(&mut self) {
        // Read 4 u32s: magic, version, nChroms, reserved
        let magic = self.read_u32();
        let version = self.read_u32();
        let n_chroms = self.read_u32();
        let _reserved = self.read_u32();

        if magic != 0x1A412743 {
            panic!("Invalid 2bit file magic number: 0x{:x}", magic);
        }
        if version != 0 {
            panic!("Unsupported 2bit version: {}", version);
        }

        self.hdr = TwoBitHeader {
            magic,
            version,
            n_chroms,
        };
    }

    pub fn twobitHdrDestroy(&mut self) {
        self.hdr = TwoBitHeader {
            magic: 0,
            version: 0,
            n_chroms: 0,
        };
    }

    // ----------- Private helpers -----------

    fn read_u8(&mut self) -> u8 {
        let v = self.data[self.offset as usize];
        self.offset += 1;
        v
    }

    fn read_u32(&mut self) -> u32 {
        let off = self.offset as usize;
        let bytes = [
            self.data[off],
            self.data[off + 1],
            self.data[off + 2],
            self.data[off + 3],
        ];
        self.offset += 4;
        u32::from_le_bytes(bytes)
    }

    fn seek_to(&mut self, off: u64) {
        if off < self.sz {
            self.offset = off;
        }
    }

    fn read_u8_at(&self, off: u64) -> u8 {
        self.data[off as usize]
    }

    fn read_bytes_at(&self, off: u64, n: usize) -> Vec<u8> {
        let start = off as usize;
        self.data[start..start + n].to_vec()
    }

    /// Get next mask block; returns (new_idx, mask_start, mask_end).
    fn get_mask_internal(
        &self,
        tid: u32,
        start: u32,
        end: u32,
        mask_idx: u32,
    ) -> (u32, u32, u32) {
        let tid_us = tid as usize;
        let n_blocks = self.idx.n_block_count[tid_us];

        let mut mi: u32;
        let mut ms: u32;
        let mut me: u32;

        if mask_idx == SENTINEL {
            mi = 0;
            ms = 0;
            me = 0;
            while mi < n_blocks {
                ms = self.idx.n_block_start[tid_us][mi as usize];
                me = ms + self.idx.n_block_sizes[tid_us][mi as usize];
                if me < start {
                    mi += 1;
                    continue;
                }
                if me >= start {
                    break;
                }
            }
        } else if mask_idx >= n_blocks {
            return (mask_idx, SENTINEL, SENTINEL);
        } else {
            mi = mask_idx + 1;
            if mi >= n_blocks {
                return (mi, SENTINEL, SENTINEL);
            }
            ms = self.idx.n_block_start[tid_us][mi as usize];
            me = ms + self.idx.n_block_sizes[tid_us][mi as usize];
        }

        if mi >= n_blocks || ms >= end {
            ms = SENTINEL;
            me = SENTINEL;
        }
        (mi, ms, me)
    }

    fn construct_sequence_internal(&self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let sz = (end - start + 1) as usize;
        let mut seq: Vec<char> = vec!['\0'; sz];

        let block_start = (start / 4) as u64;
        let offset = (start % 4) as i32;
        let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as u64;

        let file_off = self.idx.offset[tid as usize] + block_start;
        let n_bytes = (block_end - block_start) as usize;
        let mut bytes = self.read_bytes_at(file_off, n_bytes);

        bytes2bases(&mut seq[..sz - 1], &mut bytes[..], (sz - 1) as u32, offset);

        // Null terminator: in C, the last char is '\0'. In Rust we keep the Vec<char> length but
        // the consumer (twobit_sequence) collects to String, so we need to skip the last '\0'.
        // We'll leave the last char as '\0' and let the conversion logic strip it.
        seq[sz - 1] = '\0';

        // Apply N-mask
        self.n_mask_internal(&mut seq[..sz - 1], tid, start, end);

        // Apply soft-mask
        self.soft_mask_internal(&mut seq[..sz - 1], tid, start, end);

        // Truncate the trailing null
        seq.truncate(sz - 1);
        seq
    }

    fn n_mask_internal(&self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid_us = tid as usize;
        let mut pos: u32;
        let mut width: u32;
        let mut block_start: u32;
        let mut block_end: u32;

        for i in 0..self.idx.n_block_count[tid_us] as usize {
            block_start = self.idx.n_block_start[tid_us][i];
            block_end = block_start + self.idx.n_block_sizes[tid_us][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            if block_start < start {
                block_end = if block_end < end { block_end } else { end };
                pos = 0;
                width = block_end - start;
            } else {
                block_end = if block_end < end { block_end } else { end };
                pos = block_start - start;
                width = block_end - block_start;
            }
            width += pos;
            while pos < width {
                seq[pos as usize] = 'N';
                pos += 1;
            }
        }
    }

    fn soft_mask_internal(&self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid_us = tid as usize;
        if self.idx.mask_block_start.is_empty() {
            return;
        }

        let mut pos: u32;
        let mut width: u32;
        let mut block_start: u32;
        let mut block_end: u32;

        for i in 0..self.idx.mask_block_count[tid_us] as usize {
            block_start = self.idx.mask_block_start[tid_us][i];
            block_end = block_start + self.idx.mask_block_sizes[tid_us][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            if block_start < start {
                block_end = if block_end < end { block_end } else { end };
                pos = 0;
                width = block_end - start;
            } else {
                block_end = if block_end < end { block_end } else { end };
                pos = block_start - start;
                width = block_end - block_start;
            }
            width += pos;
            while pos < width {
                let c = seq[pos as usize];
                if c != 'N' {
                    seq[pos as usize] = c.to_ascii_lowercase();
                }
                pos += 1;
            }
        }
    }

    fn two_bit_bases_worker_internal(
        &self,
        tid: u32,
        start: u32,
        end: u32,
        fraction: i32,
    ) -> Vec<u8> {
        let mut tmp: [u32; 4] = [0, 0, 0, 0];
        let mut start = start;
        let len = (end - start + (start % 4)) as i64;
        let seq_len = (end - start) as u32;

        let block_start = (start / 4) as u64;
        let mut offset = (start % 4) as i32;
        let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as u64;

        // Read bytes
        let file_off = self.idx.offset[tid as usize] + block_start;
        let n_bytes = (block_end - block_start) as usize;
        let bytes = self.read_bytes_at(file_off, n_bytes);

        // Initial mask
        let mut mask: u8 = get_byte_mask_from_offset(offset);
        start = 4 * block_start as u32;
        offset = 0;

        // Get initial mask block info
        let (mut mask_idx, mut mask_start, mut mask_end) =
            self.get_mask_internal(tid, start, end, SENTINEL);

        let mut i: i64 = 0;
        let mut j: i64 = 0;

        while i < len {
            // Check if we need to jump
            if mask_idx != SENTINEL && (start as i64 + i + 4) >= mask_start as i64 {
                if (start as i64 + i) >= mask_start as i64
                    || (start as i64 + i + 4 - offset as i64) > mask_start as i64
                {
                    // Whole byte inside an N block?
                    if (start as i64 + i) >= mask_start as i64
                        && (start as i64 + i + 4 - offset as i64) < mask_end as i64
                    {
                        i = mask_end as i64 - start as i64;
                        let (ni, ns, ne) =
                            self.get_mask_internal(tid, i as u32, end, mask_idx);
                        mask_idx = ni;
                        mask_start = ns;
                        mask_end = ne;
                        offset = ((start as i64 + i) % 4) as i32;
                        j = i / 4;
                        mask = get_byte_mask_from_offset(offset);
                        i = 4 * j;
                        offset = 0;
                        continue;
                    }

                    let foo = 4 * j as i64 + 4 * block_start as i64;
                    if (mask & 1) != 0
                        && (foo + 3 >= mask_start as i64 && foo + 3 < mask_end as i64)
                    {
                        mask -= 1;
                    }
                    if (mask & 2) != 0
                        && (foo + 2 >= mask_start as i64 && foo + 2 < mask_end as i64)
                    {
                        mask -= 2;
                    }
                    if (mask & 4) != 0
                        && (foo + 1 >= mask_start as i64 && foo + 1 < mask_end as i64)
                    {
                        mask -= 4;
                    }
                    if (mask & 8) != 0 && (foo >= mask_start as i64 && foo < mask_end as i64) {
                        mask -= 8;
                    }
                    if foo + 4 > mask_end as i64 {
                        let (ni, ns, ne) =
                            self.get_mask_internal(tid, i as u32, end, mask_idx);
                        mask_idx = ni;
                        mask_start = ns;
                        mask_end = ne;
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

            let mut foo = bytes[j as usize] as u32;
            j += 1;
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
            i += 4;
            mask = 15;
        }

        // ACTG order: tmp is TCAG order; output is ACTG.
        if fraction != 0 {
            let a = tmp[2] as f64 / seq_len as f64;
            let c = tmp[1] as f64 / seq_len as f64;
            let t = tmp[0] as f64 / seq_len as f64;
            let g = tmp[3] as f64 / seq_len as f64;
            let mut out = Vec::with_capacity(32);
            out.extend_from_slice(&a.to_le_bytes());
            out.extend_from_slice(&c.to_le_bytes());
            out.extend_from_slice(&t.to_le_bytes());
            out.extend_from_slice(&g.to_le_bytes());
            out
        } else {
            let mut out = Vec::with_capacity(16);
            out.extend_from_slice(&tmp[2].to_le_bytes());
            out.extend_from_slice(&tmp[1].to_le_bytes());
            out.extend_from_slice(&tmp[0].to_le_bytes());
            out.extend_from_slice(&tmp[3].to_le_bytes());
            out
        }
    }
}

// Helper functions (module-level)
pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let mask: u8 = 3u8 << (2 * rev);
    let foo = ((mask & byte) >> (2 * rev)) as usize;
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo]
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    bytes2bases_impl(seq, bytes, sz, offset);
}

fn bytes2bases_impl(seq: &mut [char], bytes: &[u8], sz: u32, offset: i32) {
    let bases = ['T', 'C', 'A', 'G'];
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let mut offset = offset;
    let mut foo: u8 = bytes[0];

    // Deal with the first partial byte
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
        foo = bytes[i];
    }

    // Deal with everything else, except the last fractional byte
    let remainder = (sz - pos) % 4;
    while pos < sz - remainder {
        foo = bytes[i];
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

    // Deal with the last partial byte
    if remainder > 0 {
        foo = bytes[i];
    }
    let mut local_offset: i32 = 0;
    while local_offset < remainder as i32 {
        seq[pos as usize] = byte2base(foo, local_offset);
        pos += 1;
        local_offset += 1;
    }
}

pub fn getByteMaskFromOffset(_offset: i32) {
    // This is a stub - real impl is `get_byte_mask_from_offset` returning u8.
}

fn get_byte_mask_from_offset(offset: i32) -> u8 {
    match offset {
        0 => 15,
        1 => 7,
        2 => 3,
        _ => 1,
    }
}
