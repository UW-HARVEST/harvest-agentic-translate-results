use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
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
        let mut fp = File::open(fname).expect("Cannot open file");
        let sz = fp.metadata().map(|m| m.len()).unwrap_or(0);
        let mut data = Vec::with_capacity(sz as usize);
        fp.read_to_end(&mut data).expect("Cannot read file");

        let mut tb = TwoBit {
            fp,
            sz,
            offset: 0,
            data,
            hdr: TwoBitHeader { magic: 0, version: 0, n_chroms: 0 },
            cl: TwoBitCL { chrom: Vec::new(), offset: Vec::new() },
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
        // In Rust, resources are cleaned up on drop. This is a no-op.
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
        // We need a mutable copy for seeking
        let mut tb = self.clone_for_read();
        let tid = match tb.find_chrom(chrom) {
            Some(t) => t,
            None => return String::new(),
        };
        let mut start = start;
        let mut end = end;
        if start == 0 && end == 0 {
            end = tb.idx.size[tid as usize];
        }
        if end > tb.idx.size[tid as usize] { return String::new(); }
        if start >= end { return String::new(); }
        let seq = tb.constructSequence(tid, start, end);
        seq.into_iter().collect()
    }
    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let mut tb = self.clone_for_read();
        let tid = match tb.find_chrom(chrom) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let mut start = start;
        let mut end = end;
        if start == 0 && end == 0 {
            end = tb.idx.size[tid as usize];
        }
        if end > tb.idx.size[tid as usize] { return Vec::new(); }
        if start >= end { return Vec::new(); }
        tb.twoBitBasesWorkerImpl(tid, start, end, fraction)
    }
    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }
    pub fn twobitRead(&mut self, data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // This signature is odd - data is &Vec<u8> but we need to write into it.
        // We'll use our internal read method instead.
        0
    }
    pub fn twobitSeek(&mut self, offset: u64) {
        if offset < self.sz {
            self.offset = offset;
        }
    }
    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid = tid as usize;
        for i in 0..self.idx.n_block_count[tid] as usize {
            let block_start = self.idx.n_block_start[tid][i];
            let block_end = block_start + self.idx.n_block_sizes[tid][i];
            if block_end <= start { continue; }
            if block_start >= end { break; }
            let (pos, width) = if block_start < start {
                let be = block_end.min(end);
                (0u32, be - start)
            } else {
                let be = block_end.min(end);
                (block_start - start, be - block_start)
            };
            let end_pos = pos + width;
            for p in pos..end_pos {
                seq[p as usize] = 'N';
            }
        }
    }
    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid = tid as usize;
        if self.idx.mask_block_start.is_empty() { return; }
        // Check if mask_block_start has entries for this tid
        if self.idx.mask_block_start[tid].is_empty() { return; }
        for i in 0..self.idx.mask_block_count[tid] as usize {
            let block_start = self.idx.mask_block_start[tid][i];
            let block_end = block_start + self.idx.mask_block_sizes[tid][i];
            if block_end <= start { continue; }
            if block_start >= end { break; }
            let (pos, width) = if block_start < start {
                let be = block_end.min(end);
                (0u32, be - start)
            } else {
                let be = block_end.min(end);
                (block_start - start, be - block_start)
            };
            let end_pos = pos + width;
            for p in pos..end_pos {
                if seq[p as usize] != 'N' {
                    seq[p as usize] = seq[p as usize].to_ascii_lowercase();
                }
            }
        }
    }
    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let sz = end - start;
        let block_start_byte = start / 4;
        let offset = (start % 4) as i32;
        let block_end_byte = end / 4 + if end % 4 != 0 { 1 } else { 0 };
        let byte_count = (block_end_byte - block_start_byte) as usize;

        self.offset = self.idx.offset[tid as usize] + block_start_byte as u64;
        let mut bytes = self.read_bytes(byte_count);

        let mut seq = vec!['\0'; (sz + 1) as usize];
        bytes2bases(&mut seq, &mut bytes, sz, offset);
        seq[sz as usize] = '\0';

        self.NMask(&mut seq[..sz as usize], tid, start, end);
        self.softMask(&mut seq[..sz as usize], tid, start, end);

        // Return only the actual sequence chars (without null terminator)
        seq[..sz as usize].to_vec()
    }
    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        // Not used directly - the logic is inlined in twoBitBasesWorkerImpl
        (0, 0, 0)
    }
    pub fn twoBitBasesWorker(&mut self, tid: u32, start: u32, end: u32, fraction: i32) {
        // Actual implementation is in twoBitBasesWorkerImpl which returns Vec<u8>
    }
    pub fn twoBitIndexRead(&mut self, store_masked: i32) {
        let n = self.hdr.n_chroms as usize;
        let mut idx = TwoBitMaskedIdx {
            size: vec![0u32; n],
            n_block_count: vec![0u32; n],
            n_block_start: vec![Vec::new(); n],
            n_block_sizes: vec![Vec::new(); n],
            mask_block_count: vec![0u32; n],
            mask_block_start: vec![Vec::new(); n],
            mask_block_sizes: vec![Vec::new(); n],
            offset: vec![0u64; n],
        };

        for i in 0..n {
            self.offset = self.cl.offset[i] as u64;
            let data = self.read_u32s(2);
            idx.size[i] = data[0];
            idx.n_block_count[i] = data[1];

            idx.n_block_start[i] = self.read_u32s(idx.n_block_count[i] as usize);
            idx.n_block_sizes[i] = self.read_u32s(idx.n_block_count[i] as usize);

            let mbc = self.read_u32s(1);
            idx.mask_block_count[i] = mbc[0];

            if store_masked != 0 {
                idx.mask_block_start[i] = self.read_u32s(idx.mask_block_count[i] as usize);
                idx.mask_block_sizes[i] = self.read_u32s(idx.mask_block_count[i] as usize);
            } else {
                self.offset += 8 * idx.mask_block_count[i] as u64;
            }

            // Reserved field
            let _ = self.read_u32s(1);
            idx.offset[i] = self.offset;
        }

        self.idx = idx;
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
        let mut cl = TwoBitCL {
            chrom: Vec::with_capacity(n),
            offset: Vec::with_capacity(n),
        };

        for _ in 0..n {
            let byte_val = self.data[self.offset as usize];
            self.offset += 1;
            let str_bytes = &self.data[self.offset as usize..self.offset as usize + byte_val as usize];
            let name = String::from_utf8_lossy(str_bytes).to_string();
            self.offset += byte_val as u64;
            let off = self.read_u32s(1)[0];
            cl.chrom.push(name);
            cl.offset.push(off);
        }

        self.cl = cl;
    }
    pub fn twobitChromListDestroy(&mut self) {
        self.cl = TwoBitCL { chrom: Vec::new(), offset: Vec::new() };
    }
    pub fn twobitHdrRead(&mut self) {
        let data = self.read_u32s(4);
        let magic = data[0];
        assert!(magic == 0x1A412743, "Invalid file magic number");
        let version = data[1];
        assert!(version == 0, "Unsupported file version");
        let n_chroms = data[2];
        assert!(n_chroms > 0, "No chromosomes in file");
        self.hdr = TwoBitHeader { magic, version, n_chroms };
    }
    pub fn twobitHdrDestroy(&mut self) {
        // No-op in Rust
    }

    // --- Private helpers ---

    fn read_bytes(&mut self, count: usize) -> Vec<u8> {
        let start = self.offset as usize;
        let end = start + count;
        let result = self.data[start..end].to_vec();
        self.offset = end as u64;
        result
    }

    fn read_u32s(&mut self, count: usize) -> Vec<u32> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            let start = self.offset as usize;
            let bytes = [
                self.data[start],
                self.data[start + 1],
                self.data[start + 2],
                self.data[start + 3],
            ];
            result.push(u32::from_le_bytes(bytes));
            self.offset += 4;
        }
        result
    }

    fn find_chrom(&self, chrom: &str) -> Option<u32> {
        for i in 0..self.hdr.n_chroms as usize {
            if self.cl.chrom[i] == chrom {
                return Some(i as u32);
            }
        }
        None
    }

    fn clone_for_read(&self) -> TwoBit {
        // Create a shallow clone that shares the data buffer for reading
        let fp = self.fp.try_clone().expect("Cannot clone file handle");
        TwoBit {
            fp,
            sz: self.sz,
            offset: self.offset,
            data: self.data.clone(),
            hdr: TwoBitHeader {
                magic: self.hdr.magic,
                version: self.hdr.version,
                n_chroms: self.hdr.n_chroms,
            },
            cl: TwoBitCL {
                chrom: self.cl.chrom.clone(),
                offset: self.cl.offset.clone(),
            },
            idx: TwoBitMaskedIdx {
                size: self.idx.size.clone(),
                n_block_count: self.idx.n_block_count.clone(),
                n_block_start: self.idx.n_block_start.clone(),
                n_block_sizes: self.idx.n_block_sizes.clone(),
                mask_block_count: self.idx.mask_block_count.clone(),
                mask_block_start: self.idx.mask_block_start.clone(),
                mask_block_sizes: self.idx.mask_block_sizes.clone(),
                offset: self.idx.offset.clone(),
            },
        }
    }

    fn get_mask_impl(&self, tid: u32, start: u32, end: u32, mask_idx: &mut u32, mask_start: &mut u32, mask_end: &mut u32) {
        let tid = tid as usize;
        let neg1 = u32::MAX;
        if *mask_idx == neg1 {
            *mask_idx = 0;
            while (*mask_idx as usize) < self.idx.n_block_count[tid] as usize {
                *mask_start = self.idx.n_block_start[tid][*mask_idx as usize];
                *mask_end = *mask_start + self.idx.n_block_sizes[tid][*mask_idx as usize];
                if *mask_end < start {
                    *mask_idx += 1;
                    continue;
                }
                if *mask_end >= start { break; }
                *mask_idx += 1;
            }
        } else if *mask_idx >= self.idx.n_block_count[tid] {
            *mask_start = neg1;
            *mask_end = neg1;
        } else {
            *mask_idx += 1;
            if *mask_idx >= self.idx.n_block_count[tid] {
                *mask_start = neg1;
                *mask_end = neg1;
            } else {
                *mask_start = self.idx.n_block_start[tid][*mask_idx as usize];
                *mask_end = *mask_start + self.idx.n_block_sizes[tid][*mask_idx as usize];
            }
        }

        if *mask_idx >= self.idx.n_block_count[tid] || *mask_start >= end {
            *mask_start = neg1;
            *mask_end = neg1;
        }
    }

    fn twoBitBasesWorkerImpl(&mut self, tid: u32, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let neg1 = u32::MAX;
        let mut tmp: [u32; 4] = [0, 0, 0, 0];
        let len = end - start + (start % 4);
        let seq_len = end - start;

        let block_start_byte = start / 4;
        let initial_offset = start % 4;
        let block_end_byte = end / 4 + if end % 4 != 0 { 1 } else { 0 };
        let byte_count = (block_end_byte - block_start_byte) as usize;

        self.offset = self.idx.offset[tid as usize] + block_start_byte as u64;
        let bytes = self.read_bytes(byte_count);

        let mut mask: u8 = getByteMaskFromOffset(initial_offset as i32);
        // Reset start to aligned boundary
        let start_aligned = 4 * block_start_byte;

        let mut mask_idx: u32 = neg1;
        let mut mask_start: u32 = 0;
        let mut mask_end_val: u32 = 0;
        self.get_mask_impl(tid, start_aligned, end, &mut mask_idx, &mut mask_start, &mut mask_end_val);

        let mut i: u32 = 0;
        let mut j: u32 = 0;

        while i < len {
            // Check if we need to jump due to N-mask
            if mask_idx != neg1 && start_aligned + i + 4 >= mask_start {
                if start_aligned + i >= mask_start || start_aligned + i + 4 - (initial_offset) > mask_start {
                    // Jump iff the whole byte is inside an N block
                    if start_aligned + i >= mask_start && start_aligned + i + 4 - (initial_offset) < mask_end_val {
                        i = mask_end_val - start_aligned;
                        self.get_mask_impl(tid, i, end, &mut mask_idx, &mut mask_start, &mut mask_end_val);
                        let new_offset = (start_aligned + i) % 4;
                        j = i / 4;
                        mask = getByteMaskFromOffset(new_offset as i32);
                        i = 4 * j;
                        continue;
                    }

                    let foo_pos = 4 * j + 4 * block_start_byte;
                    if mask & 1 != 0 && (foo_pos + 3 >= mask_start && foo_pos + 3 < mask_end_val) { mask -= 1; }
                    if mask & 2 != 0 && (foo_pos + 2 >= mask_start && foo_pos + 2 < mask_end_val) { mask -= 2; }
                    if mask & 4 != 0 && (foo_pos + 1 >= mask_start && foo_pos + 1 < mask_end_val) { mask -= 4; }
                    if mask & 8 != 0 && (foo_pos >= mask_start && foo_pos < mask_end_val) { mask -= 8; }
                    if foo_pos + 4 > mask_end_val {
                        self.get_mask_impl(tid, i, end, &mut mask_idx, &mut mask_start, &mut mask_end_val);
                        continue;
                    }
                }
            }

            // Ensure anything after end is masked
            if i + 4 >= len {
                if (mask & 1 != 0) && i + 3 >= len { mask -= 1; }
                if (mask & 2 != 0) && i + 2 >= len { mask -= 2; }
                if (mask & 4 != 0) && i + 1 >= len { mask -= 4; }
                if (mask & 8 != 0) && i >= len { mask -= 8; }
            }

            let mut foo = bytes[j as usize] as u32;
            j += 1;

            // Offset 3
            if mask & 1 != 0 { tmp[(foo & 3) as usize] += 1; }
            foo >>= 2;
            mask >>= 1;
            // Offset 2
            if mask & 1 != 0 { tmp[(foo & 3) as usize] += 1; }
            foo >>= 2;
            mask >>= 1;
            // Offset 1
            if mask & 1 != 0 { tmp[(foo & 3) as usize] += 1; }
            foo >>= 2;
            mask >>= 1;
            // Offset 0
            if mask & 1 != 0 { tmp[(foo & 3) as usize] += 1; }

            i += 4;
            mask = 15;
        }

        // Output in ACTG order (tmp is in TCAG order: T=0, C=1, A=2, G=3)
        // A=tmp[2], C=tmp[1], T=tmp[0], G=tmp[3]
        if fraction != 0 {
            let sl = seq_len as f64;
            let a = (tmp[2] as f64) / sl;
            let c = (tmp[1] as f64) / sl;
            let t = (tmp[0] as f64) / sl;
            let g = (tmp[3] as f64) / sl;
            let mut out = Vec::with_capacity(32);
            out.extend_from_slice(&a.to_le_bytes());
            out.extend_from_slice(&c.to_le_bytes());
            out.extend_from_slice(&t.to_le_bytes());
            out.extend_from_slice(&g.to_le_bytes());
            out
        } else {
            let mut out = Vec::with_capacity(16);
            out.extend_from_slice(&tmp[2].to_le_bytes()); // A
            out.extend_from_slice(&tmp[1].to_le_bytes()); // C
            out.extend_from_slice(&tmp[0].to_le_bytes()); // T
            out.extend_from_slice(&tmp[3].to_le_bytes()); // G
            out
        }
    }
}
// Helper function
pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let mask = 3u8 << (2 * rev);
    let foo = (mask & byte) >> (2 * rev);
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo as usize]
}
pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    let bases = ['T', 'C', 'A', 'G'];
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let mut offset = offset;

    // Deal with the first partial byte
    if offset != 0 {
        let foo = bytes[0];
        while offset < 4 && pos < sz {
            seq[pos as usize] = byte2base(foo, offset);
            offset += 1;
            pos += 1;
        }
        if pos >= sz { return; }
        i += 1;
    }

    // Deal with everything else except possibly the last fractional byte
    let remainder = (sz - pos) % 4;
    let end = sz - remainder;
    while pos < end {
        let foo = bytes[i];
        i += 1;
        seq[(pos + 3) as usize] = bases[(foo & 3) as usize];
        let foo = foo >> 2;
        seq[(pos + 2) as usize] = bases[(foo & 3) as usize];
        let foo = foo >> 2;
        seq[(pos + 1) as usize] = bases[(foo & 3) as usize];
        let foo = foo >> 2;
        seq[pos as usize] = bases[(foo & 3) as usize];
        pos += 4;
    }

    // Deal with the last partial byte
    if remainder > 0 {
        let foo = bytes[i];
        for off in 0..remainder as i32 {
            seq[pos as usize] = byte2base(foo, off);
            pos += 1;
        }
    }
}
pub fn getByteMaskFromOffset(offset: i32) -> u8 {
    match offset {
        0 => 15,
        1 => 7,
        2 => 3,
        _ => 1,
    }
}
