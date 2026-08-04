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
        let n = self.hdr.n_chroms as usize;
        let mut tid: usize = 0;
        let mut found = false;
        for i in 0..n {
            if self.cl.chrom[i] == chrom {
                tid = i;
                found = true;
                break;
            }
        }
        if !found {
            return String::new();
        }

        let mut end = end;
        if start == 0 && end == 0 {
            end = self.idx.size[tid];
        }

        if end > self.idx.size[tid] {
            return String::new();
        }
        if start >= end {
            return String::new();
        }

        let seq = self._construct_sequence(tid as u32, start, end);
        // seq has length (end-start+1) with last char being '\0' terminator
        let mut result = String::with_capacity(seq.len() - 1);
        for &c in &seq[..seq.len() - 1] {
            result.push(c);
        }
        result
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let n = self.hdr.n_chroms as usize;
        let mut tid: usize = 0;
        let mut found = false;
        for i in 0..n {
            if self.cl.chrom[i] == chrom {
                tid = i;
                found = true;
                break;
            }
        }
        if !found {
            return Vec::new();
        }

        let mut end = end;
        if start == 0 && end == 0 {
            end = self.idx.size[tid];
        }

        if end > self.idx.size[tid] {
            return Vec::new();
        }
        if start >= end {
            return Vec::new();
        }

        let counts = self._bases_count(tid as u32, start, end);
        let seq_len = (end - start) as f64;

        let mut out: Vec<u8> = Vec::new();
        if fraction != 0 {
            // ACTG order from TCAG storage
            let frac = [
                counts[2] as f64 / seq_len, // A
                counts[1] as f64 / seq_len, // C
                counts[0] as f64 / seq_len, // T
                counts[3] as f64 / seq_len, // G
            ];
            for f in &frac {
                out.extend_from_slice(&f.to_le_bytes());
            }
        } else {
            let cnt = [counts[2], counts[1], counts[0], counts[3]];
            for c in &cnt {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        out
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        let total = sz * nmemb;
        if (self.offset as usize) + total > self.data.len() {
            return 0;
        }
        self.offset += total as u64;
        nmemb
    }

    pub fn twobitSeek(&mut self, offset: u64) {
        if offset < self.sz {
            self.offset = offset;
        }
    }

    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid_us = tid as usize;
        let mut pos: u32 = 0;
        let count = self.idx.n_block_count[tid_us];
        for i in 0..count as usize {
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
                block_end = block_end.min(end);
                pos = 0;
                width = block_end - start;
            } else {
                block_end = block_end.min(end);
                pos = block_start - start;
                width = block_end - block_start;
            }
            let total_end = pos + width;
            while pos < total_end {
                seq[pos as usize] = 'N';
                pos += 1;
            }
        }
    }

    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        if self.idx.mask_block_start.is_empty() {
            return;
        }
        let tid_us = tid as usize;
        let mut pos: u32 = 0;
        let count = self.idx.mask_block_count[tid_us];
        for i in 0..count as usize {
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
                block_end = block_end.min(end);
                pos = 0;
                width = block_end - start;
            } else {
                block_end = block_end.min(end);
                pos = block_start - start;
                width = block_end - block_start;
            }
            let total_end = pos + width;
            while pos < total_end {
                if seq[pos as usize] != 'N' {
                    seq[pos as usize] = seq[pos as usize].to_ascii_lowercase();
                }
                pos += 1;
            }
        }
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let sz = end - start + 1;
        let block_start = start / 4;
        let offset = (start % 4) as i32;
        let block_end = end / 4 + (if end % 4 != 0 { 1 } else { 0 });
        let bytes_len = (block_end - block_start) as usize;

        let chrom_offset = self.idx.offset[tid as usize] + block_start as u64;
        // mimic seek+read
        self.offset = chrom_offset + bytes_len as u64;

        let mut bytes: Vec<u8> = self.data
            [chrom_offset as usize..chrom_offset as usize + bytes_len]
            .to_vec();

        let mut seq: Vec<char> = vec!['\0'; sz as usize];
        bytes2bases(&mut seq[..(sz - 1) as usize], &mut bytes, sz - 1, offset);
        // last index is '\0' (already)

        self.NMask(&mut seq, tid, start, end);
        self.softMask(&mut seq, tid, start, end);

        seq
    }

    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        self._get_mask(tid as usize, start, end, u32::MAX, 0, 0)
    }

    pub fn twoBitBasesWorker(&mut self, tid: u32, start: u32, end: u32, _fraction: i32) {
        // Compute bases counts; since the Rust signature has no return,
        // we discard the result. The actual public computation is done in twobit_bases.
        let _ = self._bases_count(tid, start, end);
    }

    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n = self.hdr.n_chroms as usize;
        let mut size: Vec<u32> = Vec::with_capacity(n);
        let mut n_block_count: Vec<u32> = Vec::with_capacity(n);
        let mut n_block_start: Vec<Vec<u32>> = Vec::with_capacity(n);
        let mut n_block_sizes: Vec<Vec<u32>> = Vec::with_capacity(n);
        let mut mask_block_count: Vec<u32> = Vec::with_capacity(n);
        let mut mask_block_start: Vec<Vec<u32>> = Vec::new();
        let mut mask_block_sizes: Vec<Vec<u32>> = Vec::new();
        let mut idx_offset: Vec<u64> = Vec::with_capacity(n);

        for i in 0..n {
            let cl_off = self.cl.offset[i];
            self.offset = cl_off as u64;

            let chrom_size = self.read_u32_at_offset();
            let nbc = self.read_u32_at_offset();
            size.push(chrom_size);
            n_block_count.push(nbc);

            let mut starts: Vec<u32> = Vec::with_capacity(nbc as usize);
            for _ in 0..nbc {
                starts.push(self.read_u32_at_offset());
            }
            let mut sizes: Vec<u32> = Vec::with_capacity(nbc as usize);
            for _ in 0..nbc {
                sizes.push(self.read_u32_at_offset());
            }
            n_block_start.push(starts);
            n_block_sizes.push(sizes);

            let mbc = self.read_u32_at_offset();
            mask_block_count.push(mbc);

            if storeMasked != 0 {
                let mut m_starts: Vec<u32> = Vec::with_capacity(mbc as usize);
                for _ in 0..mbc {
                    m_starts.push(self.read_u32_at_offset());
                }
                let mut m_sizes: Vec<u32> = Vec::with_capacity(mbc as usize);
                for _ in 0..mbc {
                    m_sizes.push(self.read_u32_at_offset());
                }
                mask_block_start.push(m_starts);
                mask_block_sizes.push(m_sizes);
            } else {
                self.offset += 8 * mbc as u64;
            }

            // reserved 4 bytes
            let _reserved = self.read_u32_at_offset();
            idx_offset.push(self.offset);
        }

        self.idx = TwoBitMaskedIdx {
            size,
            n_block_count,
            n_block_start,
            n_block_sizes,
            mask_block_count,
            mask_block_start,
            mask_block_sizes,
            offset: idx_offset,
        };
    }

    pub fn twoBitIndexDestroy(&mut self) {
        self.idx = TwoBitMaskedIdx {
            size: Vec::new(),
            n_block_count: Vec::new(),
            n_block_start: Vec::new(),
            n_block_sizes: Vec::new(),
            mask_block_count: Vec::new(),
            mask_block_start: Vec::new(),
            mask_block_sizes: Vec::new(),
            offset: Vec::new(),
        };
    }

    pub fn twobitChromListRead(&mut self) {
        let n = self.hdr.n_chroms as usize;
        let mut chrom: Vec<String> = Vec::with_capacity(n);
        let mut offset: Vec<u32> = Vec::with_capacity(n);

        for _ in 0..n {
            let len = self.read_u8_at_offset() as usize;
            let bytes = &self.data[self.offset as usize..self.offset as usize + len];
            let name = String::from_utf8_lossy(bytes).into_owned();
            self.offset += len as u64;
            chrom.push(name);
            let off = self.read_u32_at_offset();
            offset.push(off);
        }

        self.cl = TwoBitCL { chrom, offset };
    }

    pub fn twobitChromListDestroy(&mut self) {
        self.cl = TwoBitCL {
            chrom: Vec::new(),
            offset: Vec::new(),
        };
    }

    pub fn twobitHdrRead(&mut self) {
        self.offset = 0;
        if self.data.len() < 16 {
            return;
        }
        let magic = self.read_u32_at_offset();
        let version = self.read_u32_at_offset();
        let n_chroms = self.read_u32_at_offset();
        let _reserved = self.read_u32_at_offset();

        if magic != 0x1A412743 {
            eprintln!(
                "[twobitHdrRead] Received an invalid file magic number (0x{:x})!",
                magic
            );
            return;
        }
        if version != 0 {
            eprintln!(
                "[twobitHdrRead] The file version is {} while only version 0 is defined!",
                version
            );
            return;
        }
        if n_chroms == 0 {
            eprintln!("[twobitHdrRead] There are apparently no chromosomes/contigs in this file!");
            return;
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

    // ---------- Helper (private) methods ----------

    fn read_u32_at_offset(&mut self) -> u32 {
        let off = self.offset as usize;
        let v = u32::from_le_bytes([
            self.data[off],
            self.data[off + 1],
            self.data[off + 2],
            self.data[off + 3],
        ]);
        self.offset += 4;
        v
    }

    fn read_u8_at_offset(&mut self) -> u8 {
        let off = self.offset as usize;
        let v = self.data[off];
        self.offset += 1;
        v
    }

    fn _construct_sequence(&self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let sz = end - start + 1;
        let block_start = start / 4;
        let offset = (start % 4) as i32;
        let block_end = end / 4 + (if end % 4 != 0 { 1 } else { 0 });
        let bytes_len = (block_end - block_start) as usize;

        let chrom_offset = self.idx.offset[tid as usize] + block_start as u64;
        let mut bytes: Vec<u8> = self.data
            [chrom_offset as usize..chrom_offset as usize + bytes_len]
            .to_vec();

        let mut seq: Vec<char> = vec!['\0'; sz as usize];
        bytes2bases(&mut seq[..(sz - 1) as usize], &mut bytes, sz - 1, offset);

        self._n_mask(&mut seq, tid as usize, start, end);
        self._soft_mask(&mut seq, tid as usize, start, end);

        seq
    }

    fn _n_mask(&self, seq: &mut [char], tid: usize, start: u32, end: u32) {
        let mut pos: u32 = 0;
        let count = self.idx.n_block_count[tid];
        for i in 0..count as usize {
            let block_start = self.idx.n_block_start[tid][i];
            let mut block_end = block_start + self.idx.n_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let width;
            if block_start < start {
                block_end = block_end.min(end);
                pos = 0;
                width = block_end - start;
            } else {
                block_end = block_end.min(end);
                pos = block_start - start;
                width = block_end - block_start;
            }
            let total_end = pos + width;
            while pos < total_end {
                seq[pos as usize] = 'N';
                pos += 1;
            }
        }
    }

    fn _soft_mask(&self, seq: &mut [char], tid: usize, start: u32, end: u32) {
        if self.idx.mask_block_start.is_empty() {
            return;
        }
        let mut pos: u32 = 0;
        let count = self.idx.mask_block_count[tid];
        for i in 0..count as usize {
            let block_start = self.idx.mask_block_start[tid][i];
            let mut block_end = block_start + self.idx.mask_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let width;
            if block_start < start {
                block_end = block_end.min(end);
                pos = 0;
                width = block_end - start;
            } else {
                block_end = block_end.min(end);
                pos = block_start - start;
                width = block_end - block_start;
            }
            let total_end = pos + width;
            while pos < total_end {
                if seq[pos as usize] != 'N' {
                    seq[pos as usize] = seq[pos as usize].to_ascii_lowercase();
                }
                pos += 1;
            }
        }
    }

    fn _get_mask(
        &self,
        tid: usize,
        start: u32,
        end: u32,
        mask_idx: u32,
        mask_start: u32,
        mask_end: u32,
    ) -> (u32, u32, u32) {
        let n_blocks = self.idx.n_block_count[tid];
        let mut new_idx = mask_idx;
        let mut new_start = mask_start;
        let mut new_end = mask_end;

        if mask_idx == u32::MAX {
            new_idx = 0;
            while new_idx < n_blocks {
                new_start = self.idx.n_block_start[tid][new_idx as usize];
                new_end = new_start + self.idx.n_block_sizes[tid][new_idx as usize];
                if new_end < start {
                    new_idx += 1;
                    continue;
                }
                // new_end >= start
                break;
            }
        } else if mask_idx >= n_blocks {
            new_start = u32::MAX;
            new_end = u32::MAX;
        } else {
            new_idx = mask_idx + 1;
            if new_idx >= n_blocks {
                new_start = u32::MAX;
                new_end = u32::MAX;
            } else {
                new_start = self.idx.n_block_start[tid][new_idx as usize];
                new_end = new_start + self.idx.n_block_sizes[tid][new_idx as usize];
            }
        }

        if new_idx >= n_blocks || new_start >= end {
            new_start = u32::MAX;
            new_end = u32::MAX;
        }

        (new_idx, new_start, new_end)
    }

    fn _bases_count(&self, tid: u32, start: u32, end: u32) -> [u32; 4] {
        let tid_us = tid as usize;
        let mut tmp: [u32; 4] = [0, 0, 0, 0];
        let len = end - start + (start % 4);
        let mut i: u32 = 0;
        let mut j: usize = 0;

        let block_start = start / 4;
        let initial_offset = (start % 4) as i32;
        let block_end = end / 4 + (if end % 4 != 0 { 1 } else { 0 });
        let bytes_len = (block_end - block_start) as usize;

        let chrom_offset = self.idx.offset[tid_us] + block_start as u64;
        if (chrom_offset as usize) + bytes_len > self.data.len() {
            return tmp;
        }
        let bytes = &self.data[chrom_offset as usize..chrom_offset as usize + bytes_len];

        let mut mask: u8 = get_byte_mask_from_offset(initial_offset);
        let start_aligned: u32 = 4 * block_start;
        // After alignment, the per-iteration offset is 0 (full bytes).
        let offset_in_loop: u32 = 0;

        let (mut mask_idx, mut mask_start, mut mask_end) =
            self._get_mask(tid_us, start_aligned, end, u32::MAX, 0, 0);

        while i < len {
            // Check if we need to jump
            if mask_idx != u32::MAX && start_aligned + i + 4 >= mask_start {
                if start_aligned + i >= mask_start
                    || start_aligned + i + 4 - offset_in_loop > mask_start
                {
                    // Jump iff the whole byte is inside an N block
                    if start_aligned + i >= mask_start
                        && start_aligned + i + 4 - offset_in_loop < mask_end
                    {
                        i = mask_end - start_aligned;
                        let r =
                            self._get_mask(tid_us, i, end, mask_idx, mask_start, mask_end);
                        mask_idx = r.0;
                        mask_start = r.1;
                        mask_end = r.2;
                        let new_offset = ((start_aligned + i) % 4) as i32;
                        j = (i / 4) as usize;
                        mask = get_byte_mask_from_offset(new_offset);
                        i = 4 * j as u32;
                        // offset reset to 0 implicit
                        continue;
                    }

                    // Set the mask, if appropriate
                    let foo_pos: u32 = 4 * j as u32 + 4 * block_start;
                    if mask & 1 != 0 && (foo_pos + 3 >= mask_start && foo_pos + 3 < mask_end) {
                        mask -= 1;
                    }
                    if mask & 2 != 0 && (foo_pos + 2 >= mask_start && foo_pos + 2 < mask_end) {
                        mask -= 2;
                    }
                    if mask & 4 != 0 && (foo_pos + 1 >= mask_start && foo_pos + 1 < mask_end) {
                        mask -= 4;
                    }
                    if mask & 8 != 0 && (foo_pos >= mask_start && foo_pos < mask_end) {
                        mask -= 8;
                    }
                    if foo_pos + 4 > mask_end {
                        let r =
                            self._get_mask(tid_us, i, end, mask_idx, mask_start, mask_end);
                        mask_idx = r.0;
                        mask_start = r.1;
                        mask_end = r.2;
                        continue;
                    }
                }
            }

            // Ensure that anything after the end is masked
            if i + 4 >= len {
                if mask & 1 != 0 && i + 3 >= len {
                    mask -= 1;
                }
                if mask & 2 != 0 && i + 2 >= len {
                    mask -= 2;
                }
                if mask & 4 != 0 && i + 1 >= len {
                    mask -= 4;
                }
                if mask & 8 != 0 && i >= len {
                    mask -= 8;
                }
            }

            let mut foo: u8 = bytes[j];
            j += 1;
            let mut local_mask = mask;
            // Offset 3
            if local_mask & 1 != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            local_mask >>= 1;
            // Offset 2
            if local_mask & 1 != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            local_mask >>= 1;
            // Offset 1
            if local_mask & 1 != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            local_mask >>= 1;
            // Offset 0
            if local_mask & 1 != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            i += 4;
            mask = 15;
        }

        tmp
    }
}

// Helper function
pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let mask: u8 = 3u8 << (2 * rev);
    let foo = ((mask & byte) >> (2 * rev)) as usize;
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo]
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    let bases = ['T', 'C', 'A', 'G'];
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let mut foo: u8 = bytes[0];
    let mut offset = offset;

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

    // Deal with everything else, with the possible exception of the last fractional byte
    let remainder: u32 = (sz - pos) % 4;
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
        // foo >>= 2; // not needed; reassigned at top of loop
        pos += 4;
    }

    // Deal with the last partial byte
    if remainder > 0 {
        foo = bytes[i];
    }
    let mut off2: i32 = 0;
    while off2 < remainder as i32 {
        seq[pos as usize] = byte2base(foo, off2);
        pos += 1;
        off2 += 1;
    }
}

pub fn getByteMaskFromOffset(offset: i32) {
    // Public no-return helper; the actual byte mask is computed via the
    // private `get_byte_mask_from_offset`. We keep this around to match the
    // declared signature.
    let _ = get_byte_mask_from_offset(offset);
}

fn get_byte_mask_from_offset(offset: i32) -> u8 {
    match offset {
        0 => 15,
        1 => 7,
        2 => 3,
        _ => 1,
    }
}
