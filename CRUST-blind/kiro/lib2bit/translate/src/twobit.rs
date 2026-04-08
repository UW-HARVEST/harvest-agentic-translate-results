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
        // We need &mut self for reading, so use a clone-like approach via a mutable copy of offset
        // Actually the signature takes &self but we need mutability for seek/read.
        // We'll work directly with self.data to avoid needing mutability.
        let n = self.hdr.n_chroms as usize;
        let mut tid: Option<usize> = None;
        for i in 0..n {
            if self.cl.chrom[i] == chrom {
                tid = Some(i);
                break;
            }
        }
        let tid = match tid {
            Some(t) => t,
            None => return String::new(),
        };

        let mut start = start;
        let mut end = end;
        if start == 0 && end == 0 {
            end = self.idx.size[tid];
        }
        if end > self.idx.size[tid] { return String::new(); }
        if start >= end { return String::new(); }

        self.construct_sequence_immut(tid, start, end)
    }
    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let n = self.hdr.n_chroms as usize;
        let mut tid: Option<usize> = None;
        for i in 0..n {
            if self.cl.chrom[i] == chrom {
                tid = Some(i);
                break;
            }
        }
        let tid = match tid {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut start = start;
        let mut end = end;
        if start == 0 && end == 0 {
            end = self.idx.size[tid];
        }
        if end > self.idx.size[tid] { return Vec::new(); }
        if start >= end { return Vec::new(); }

        self.twobit_bases_worker_immut(tid, start, end, fraction)
    }
    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }
    pub fn twobitRead(&mut self, data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // This signature is odd - data is &Vec<u8> (immutable). We won't use this directly.
        // The actual reading is done via helper methods that work on self.data.
        nmemb
    }
    pub fn twobitSeek(&mut self, offset: u64) {
        if offset < self.sz {
            self.offset = offset;
        }
    }
    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        nmask_impl(seq, &self.idx, tid as usize, start, end);
    }
    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        soft_mask_impl(seq, &self.idx, tid as usize, start, end);
    }
    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let sz = (end - start + 1) as usize;
        let block_start = (start / 4) as usize;
        let off = (start % 4) as i32;
        let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as usize;
        let byte_len = block_end - block_start;

        let file_offset = self.idx.offset[tid as usize] + block_start as u64;
        let mut bytes = vec![0u8; byte_len];
        let src = &self.data[file_offset as usize..file_offset as usize + byte_len];
        bytes.copy_from_slice(src);

        let mut seq = vec!['\0'; sz];
        bytes2bases(&mut seq, &mut bytes, (sz - 1) as u32, off);
        seq[sz - 1] = '\0';

        nmask_impl(&mut seq, &self.idx, tid as usize, start, end);
        soft_mask_impl(&mut seq, &self.idx, tid as usize, start, end);

        seq
    }
    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        let mut mask_idx: u32 = u32::MAX;
        let mut mask_start: u32 = 0;
        let mut mask_end: u32 = 0;
        get_mask_impl(&self.idx, tid as usize, start, end, &mut mask_idx, &mut mask_start, &mut mask_end);
        (mask_idx, mask_start, mask_end)
    }
    pub fn twoBitBasesWorker(&mut self, tid: u32, start: u32, end: u32, fraction: i32) {
        // void return in the signature - this is a no-op wrapper
        // actual work done in twobit_bases_worker_immut
    }
    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
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
            let off = self.cl.offset[i] as u64;
            self.offset = off;
            let size = self.read_u32();
            let n_block_count = self.read_u32();
            idx.size[i] = size;
            idx.n_block_count[i] = n_block_count;

            idx.n_block_start[i] = self.read_u32_vec(n_block_count as usize);
            idx.n_block_sizes[i] = self.read_u32_vec(n_block_count as usize);

            let mask_block_count = self.read_u32();
            idx.mask_block_count[i] = mask_block_count;

            if storeMasked != 0 {
                idx.mask_block_start[i] = self.read_u32_vec(mask_block_count as usize);
                idx.mask_block_sizes[i] = self.read_u32_vec(mask_block_count as usize);
            } else {
                self.offset += 8 * mask_block_count as u64;
            }

            // Reserved field
            let _ = self.read_u32();

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
            let byte = self.read_u8();
            let len = byte as usize;
            let s = self.read_string(len);
            cl.chrom.push(s);
            let off = self.read_u32();
            cl.offset.push(off);
        }

        self.cl = cl;
    }
    pub fn twobitChromListDestroy(&mut self) {
        self.cl = TwoBitCL { chrom: Vec::new(), offset: Vec::new() };
    }
    pub fn twobitHdrRead(&mut self) {
        self.offset = 0;
        let magic = self.read_u32();
        assert_eq!(magic, 0x1A412743, "Invalid file magic number");
        let version = self.read_u32();
        assert_eq!(version, 0, "Only version 0 is supported");
        let n_chroms = self.read_u32();
        assert!(n_chroms > 0, "No chromosomes in file");
        let _reserved = self.read_u32();
        self.hdr = TwoBitHeader { magic, version, n_chroms };
    }
    pub fn twobitHdrDestroy(&mut self) {
        // no-op in Rust
    }

    // --- private helpers ---

    fn read_u8(&mut self) -> u8 {
        let v = self.data[self.offset as usize];
        self.offset += 1;
        v
    }

    fn read_u32(&mut self) -> u32 {
        let o = self.offset as usize;
        let v = u32::from_le_bytes([self.data[o], self.data[o+1], self.data[o+2], self.data[o+3]]);
        self.offset += 4;
        v
    }

    fn read_u32_vec(&mut self, count: usize) -> Vec<u32> {
        let mut v = Vec::with_capacity(count);
        for _ in 0..count {
            v.push(self.read_u32());
        }
        v
    }

    fn read_string(&mut self, len: usize) -> String {
        let o = self.offset as usize;
        let s = String::from_utf8_lossy(&self.data[o..o+len]).to_string();
        self.offset += len as u64;
        s
    }

    fn construct_sequence_immut(&self, tid: usize, start: u32, end: u32) -> String {
        let sz = (end - start + 1) as usize;
        let block_start = (start / 4) as usize;
        let off = (start % 4) as i32;
        let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as usize;
        let byte_len = block_end - block_start;

        let file_offset = self.idx.offset[tid] as usize + block_start;
        let mut bytes: Vec<u8> = self.data[file_offset..file_offset + byte_len].to_vec();

        let mut seq = vec!['\0'; sz];
        bytes2bases(&mut seq, &mut bytes, (sz - 1) as u32, off);
        seq[sz - 1] = '\0';

        nmask_impl(&mut seq, &self.idx, tid, start, end);
        soft_mask_impl(&mut seq, &self.idx, tid, start, end);

        seq[..sz-1].iter().collect()
    }

    fn twobit_bases_worker_immut(&self, tid: usize, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let mut tmp: [u32; 4] = [0, 0, 0, 0];
        let seq_len = end - start;
        let len = end - start + (start % 4);

        let block_start_byte = (start / 4) as usize;
        let initial_offset = (start % 4) as u8;
        let block_end_byte = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as usize;
        let byte_len = block_end_byte - block_start_byte;

        let file_offset = self.idx.offset[tid] as usize + block_start_byte;
        let bytes: &[u8] = &self.data[file_offset..file_offset + byte_len];

        let mut mask: u8 = get_byte_mask_from_offset(initial_offset as i32);
        let adj_start = 4 * block_start_byte as u32;

        let mut mask_idx: u32 = u32::MAX;
        let mut mask_start: u32 = 0;
        let mut mask_end_val: u32 = 0;
        get_mask_impl(&self.idx, tid, adj_start, end, &mut mask_idx, &mut mask_start, &mut mask_end_val);

        let mut i: u32 = 0;
        let mut j: usize = 0;
        let mut offset: u8 = 0;

        while i < len {
            // Check if we need to jump due to N-mask
            if mask_idx != u32::MAX && adj_start + i + 4 >= mask_start {
                if adj_start + i >= mask_start || adj_start + i + 4 - offset as u32 > mask_start {
                    // Jump iff the whole byte is inside an N block
                    if adj_start + i >= mask_start && adj_start + i + 4 - offset as u32 <= mask_end_val {
                        // Fully in an N block, jump
                        let new_i = mask_end_val - adj_start;
                        i = new_i;
                        get_mask_impl(&self.idx, tid, i, end, &mut mask_idx, &mut mask_start, &mut mask_end_val);
                        offset = ((adj_start + i) % 4) as u8;
                        j = (i / 4) as usize;
                        mask = get_byte_mask_from_offset(offset as i32);
                        i = 4 * j as u32;
                        offset = 0;
                        continue;
                    }

                    // Set the mask, if appropriate
                    let foo = 4 * j as u32 + 4 * block_start_byte as u32;
                    if mask & 1 != 0 && foo + 3 >= mask_start && foo + 3 < mask_end_val { mask -= 1; }
                    if mask & 2 != 0 && foo + 2 >= mask_start && foo + 2 < mask_end_val { mask -= 2; }
                    if mask & 4 != 0 && foo + 1 >= mask_start && foo + 1 < mask_end_val { mask -= 4; }
                    if mask & 8 != 0 && foo >= mask_start && foo < mask_end_val { mask -= 8; }
                    if foo + 4 > mask_end_val {
                        get_mask_impl(&self.idx, tid, i, end, &mut mask_idx, &mut mask_start, &mut mask_end_val);
                        continue;
                    }
                }
            }

            // Ensure that anything after the end is masked
            if i + 4 >= len {
                if (mask & 1 != 0) && i + 3 >= len { mask -= 1; }
                if (mask & 2 != 0) && i + 2 >= len { mask -= 2; }
                if (mask & 4 != 0) && i + 1 >= len { mask -= 4; }
                if (mask & 8 != 0) && i >= len { mask -= 8; }
            }

            let mut foo = bytes[j] as u32;
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

fn nmask_impl(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32) {
    for i in 0..idx.n_block_count[tid] as usize {
        let block_start = idx.n_block_start[tid][i];
        let mut block_end = block_start + idx.n_block_sizes[tid][i];
        if block_end <= start { continue; }
        if block_start >= end { break; }
        let pos;
        let width;
        if block_start < start {
            block_end = block_end.min(end);
            pos = 0usize;
            width = (block_end - start) as usize;
        } else {
            block_end = block_end.min(end);
            pos = (block_start - start) as usize;
            width = (block_end - block_start) as usize;
        }
        for p in pos..pos + width {
            seq[p] = 'N';
        }
    }
}

fn soft_mask_impl(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32) {
    if idx.mask_block_start.is_empty() || idx.mask_block_start[tid].is_empty() && idx.mask_block_count[tid] == 0 {
        return;
    }
    for i in 0..idx.mask_block_count[tid] as usize {
        if i >= idx.mask_block_start[tid].len() { break; }
        let block_start = idx.mask_block_start[tid][i];
        let mut block_end = block_start + idx.mask_block_sizes[tid][i];
        if block_end <= start { continue; }
        if block_start >= end { break; }
        let pos;
        let width;
        if block_start < start {
            block_end = block_end.min(end);
            pos = 0usize;
            width = (block_end - start) as usize;
        } else {
            block_end = block_end.min(end);
            pos = (block_start - start) as usize;
            width = (block_end - block_start) as usize;
        }
        for p in pos..pos + width {
            if seq[p] != 'N' {
                seq[p] = seq[p].to_ascii_lowercase();
            }
        }
    }
}

fn get_mask_impl(idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32, mask_idx: &mut u32, mask_start: &mut u32, mask_end: &mut u32) {
    if *mask_idx == u32::MAX {
        let count = idx.n_block_count[tid];
        let mut found = false;
        for mi in 0..count {
            *mask_start = idx.n_block_start[tid][mi as usize];
            *mask_end = *mask_start + idx.n_block_sizes[tid][mi as usize];
            if *mask_end < start { continue; }
            if *mask_end >= start {
                *mask_idx = mi;
                found = true;
                break;
            }
        }
        if !found {
            *mask_idx = count;
        }
    } else if *mask_idx >= idx.n_block_count[tid] {
        *mask_start = u32::MAX;
        *mask_end = u32::MAX;
    } else {
        *mask_idx += 1;
        if *mask_idx >= idx.n_block_count[tid] {
            *mask_start = u32::MAX;
            *mask_end = u32::MAX;
        } else {
            *mask_start = idx.n_block_start[tid][*mask_idx as usize];
            *mask_end = *mask_start + idx.n_block_sizes[tid][*mask_idx as usize];
        }
    }

    if *mask_idx >= idx.n_block_count[tid] || *mask_start >= end {
        *mask_start = u32::MAX;
        *mask_end = u32::MAX;
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
    let mask = 3u8 << (2 * rev);
    let foo = (mask & byte) >> (2 * rev);
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo as usize]
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    let sz = sz as usize;
    let mut pos: usize = 0;
    let mut i: usize = 0;
    let bases = ['T', 'C', 'A', 'G'];
    let mut offset = offset;

    // Deal with the first partial byte
    if offset != 0 {
        let foo = bytes[0];
        while offset < 4 && pos < sz {
            seq[pos] = byte2base(foo, offset);
            pos += 1;
            offset += 1;
        }
        if pos >= sz { return; }
        i += 1;
    }

    // Deal with everything else, with the possible exception of the last fractional byte
    let remainder = (sz - pos) % 4;
    while pos < sz - remainder {
        let mut foo = bytes[i];
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

    // Deal with the last partial byte
    if remainder > 0 {
        let foo = bytes[i];
        for off in 0..remainder {
            seq[pos] = byte2base(foo, off as i32);
            pos += 1;
        }
    }
}

pub fn getByteMaskFromOffset(offset: i32) {
    // This is a void-returning public wrapper; actual logic in get_byte_mask_from_offset
    let _ = get_byte_mask_from_offset(offset);
}
