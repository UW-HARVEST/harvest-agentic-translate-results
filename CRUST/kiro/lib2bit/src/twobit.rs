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

fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off+1], data[off+2], data[off+3]])
}

impl TwoBit {
    pub fn twobit_open(fname: &str, store_masked: bool) -> Self {
        let mut fp = File::open(fname).expect("Cannot open file");
        let mut data = Vec::new();
        fp.read_to_end(&mut data).expect("Cannot read file");
        let sz = data.len() as u64;
        let mut tb = TwoBit {
            fp,
            sz,
            offset: 0,
            data,
            hdr: TwoBitHeader { magic: 0, version: 0, n_chroms: 0 },
            cl: TwoBitCL { chrom: Vec::new(), offset: Vec::new() },
            idx: TwoBitMaskedIdx {
                size: Vec::new(), n_block_count: Vec::new(),
                n_block_start: Vec::new(), n_block_sizes: Vec::new(),
                mask_block_count: Vec::new(), mask_block_start: Vec::new(),
                mask_block_sizes: Vec::new(), offset: Vec::new(),
            },
        };
        tb.twobitHdrRead();
        tb.twobitChromListRead();
        tb.twoBitIndexRead(if store_masked { 1 } else { 0 });
        tb
    }

    pub fn twobit_close(&mut self) {
        // No-op in Rust; resources freed on drop
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
        // Find tid
        let mut tid: Option<usize> = None;
        for i in 0..self.hdr.n_chroms as usize {
            if self.cl.chrom[i] == chrom {
                tid = Some(i);
                break;
            }
        }
        let tid = match tid { Some(t) => t, None => return String::new() };

        let mut start = start;
        let mut end = end;
        if start == 0 && end == 0 {
            end = self.idx.size[tid];
        }
        if end > self.idx.size[tid] { return String::new(); }
        if start >= end { return String::new(); }

        // constructSequence inlined with a clone for offset manipulation
        let sz = (end - start) as usize;
        let block_start = (start / 4) as usize;
        let offset = (start % 4) as i32;
        let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as usize;
        let byte_len = block_end - block_start;
        let data_offset = self.idx.offset[tid] as usize + block_start;
        let bytes = &self.data[data_offset..data_offset + byte_len];

        let mut seq = vec![' '; sz];
        bytes2bases(&mut seq, &mut bytes.to_vec(), sz as u32, offset);

        // N-mask
        n_mask(&mut seq, &self.idx, tid, start, end);
        // soft-mask
        soft_mask(&mut seq, &self.idx, tid, start, end);

        seq.into_iter().collect()
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let mut tid: Option<usize> = None;
        for i in 0..self.hdr.n_chroms as usize {
            if self.cl.chrom[i] == chrom {
                tid = Some(i);
                break;
            }
        }
        let tid = match tid { Some(t) => t, None => return Vec::new() };

        let mut start = start;
        let mut end = end;
        if start == 0 && end == 0 {
            end = self.idx.size[tid];
        }
        if end > self.idx.size[tid] { return Vec::new(); }
        if start >= end { return Vec::new(); }

        twobit_bases_worker(&self.data, &self.idx, tid, start, end, fraction)
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // Not used externally; kept for signature compatibility
        nmemb
    }

    pub fn twobitSeek(&mut self, offset: u64) {
        self.offset = offset;
    }

    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        n_mask(seq, &self.idx, tid as usize, start, end);
    }

    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        soft_mask(seq, &self.idx, tid as usize, start, end);
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let s = self.twobit_sequence(&self.cl.chrom[tid as usize].clone(), start, end);
        s.chars().collect()
    }

    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        let (mut mask_idx, mut mask_start, mut mask_end) = (u32::MAX, 0u32, 0u32);
        get_mask(&self.idx, tid as usize, start, end, &mut mask_idx, &mut mask_start, &mut mask_end);
        (mask_idx, mask_start, mask_end)
    }

    pub fn twoBitBasesWorker(&mut self, tid: u32, start: u32, end: u32, fraction: i32) {
        // Result discarded per signature; actual work done in twobit_bases
    }

    pub fn twoBitIndexRead(&mut self, store_masked: i32) {
        let n = self.hdr.n_chroms as usize;
        let mut idx = TwoBitMaskedIdx {
            size: vec![0; n],
            n_block_count: vec![0; n],
            n_block_start: vec![Vec::new(); n],
            n_block_sizes: vec![Vec::new(); n],
            mask_block_count: vec![0; n],
            mask_block_start: vec![Vec::new(); n],
            mask_block_sizes: vec![Vec::new(); n],
            offset: vec![0; n],
        };

        for i in 0..n {
            self.offset = self.cl.offset[i] as u64;
            let off = self.offset as usize;

            idx.size[i] = read_u32(&self.data, off);
            idx.n_block_count[i] = read_u32(&self.data, off + 4);
            self.offset += 8;

            let nbc = idx.n_block_count[i] as usize;
            let mut o = self.offset as usize;
            idx.n_block_start[i] = (0..nbc).map(|j| read_u32(&self.data, o + j * 4)).collect();
            o += nbc * 4;
            idx.n_block_sizes[i] = (0..nbc).map(|j| read_u32(&self.data, o + j * 4)).collect();
            o += nbc * 4;

            idx.mask_block_count[i] = read_u32(&self.data, o);
            o += 4;
            let mbc = idx.mask_block_count[i] as usize;

            if store_masked != 0 {
                idx.mask_block_start[i] = (0..mbc).map(|j| read_u32(&self.data, o + j * 4)).collect();
                o += mbc * 4;
                idx.mask_block_sizes[i] = (0..mbc).map(|j| read_u32(&self.data, o + j * 4)).collect();
                o += mbc * 4;
            } else {
                o += mbc * 8;
            }

            // reserved
            o += 4;
            idx.offset[i] = o as u64;
            self.offset = o as u64;
        }
        self.idx = idx;
    }

    pub fn twoBitIndexDestroy(&mut self) {
        // No-op in Rust
    }

    pub fn twobitChromListRead(&mut self) {
        let n = self.hdr.n_chroms as usize;
        let mut chroms = Vec::with_capacity(n);
        let mut offsets = Vec::with_capacity(n);

        for _ in 0..n {
            let o = self.offset as usize;
            let name_len = self.data[o] as usize;
            self.offset += 1;
            let o = self.offset as usize;
            let name = String::from_utf8_lossy(&self.data[o..o + name_len]).to_string();
            self.offset += name_len as u64;
            let o = self.offset as usize;
            let off = read_u32(&self.data, o);
            self.offset += 4;
            chroms.push(name);
            offsets.push(off);
        }
        self.cl = TwoBitCL { chrom: chroms, offset: offsets };
    }

    pub fn twobitChromListDestroy(&mut self) {
        // No-op in Rust
    }

    pub fn twobitHdrRead(&mut self) {
        let magic = read_u32(&self.data, 0);
        assert_eq!(magic, 0x1A412743, "Invalid magic number");
        let version = read_u32(&self.data, 4);
        assert_eq!(version, 0, "Unsupported version");
        let n_chroms = read_u32(&self.data, 8);
        assert!(n_chroms > 0, "No chromosomes");
        self.hdr = TwoBitHeader { magic, version, n_chroms };
        self.offset = 16; // 4 u32s = 16 bytes
    }

    pub fn twobitHdrDestroy(&mut self) {
        // No-op in Rust
    }
}

// Helper functions

pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let mask = 3u8 << (2 * rev);
    let foo = (mask & byte) >> (2 * rev);
    let bases = [b'T', b'C', b'A', b'G'];
    bases[foo as usize] as char
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    let bases = ['T', 'C', 'A', 'G'];
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let mut offset = offset;

    // First partial byte
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

    let remainder = (sz - pos) % 4;
    let full_end = sz - remainder;
    while pos < full_end {
        let mut foo = bytes[i] as u32;
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
        let foo = bytes[i];
        for off in 0..remainder as i32 {
            seq[pos as usize] = byte2base(foo, off);
            pos += 1;
        }
    }
}

pub fn getByteMaskFromOffset(offset: i32) {
    // Signature returns nothing per the Rust interface; actual logic is inlined where needed
}

fn get_byte_mask_from_offset(offset: i32) -> u8 {
    match offset {
        0 => 15,
        1 => 7,
        2 => 3,
        _ => 1,
    }
}

fn n_mask(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32) {
    for i in 0..idx.n_block_count[tid] as usize {
        let block_start = idx.n_block_start[tid][i];
        let mut block_end = block_start + idx.n_block_sizes[tid][i];
        if block_end <= start { continue; }
        if block_start >= end { break; }
        let pos;
        if block_start < start {
            block_end = block_end.min(end);
            pos = 0u32;
            let width = block_end - start;
            for p in pos..pos + width { seq[p as usize] = 'N'; }
        } else {
            block_end = block_end.min(end);
            pos = block_start - start;
            let width = block_end - block_start;
            for p in pos..pos + width { seq[p as usize] = 'N'; }
        }
    }
}

fn soft_mask(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32) {
    if idx.mask_block_start.is_empty() || idx.mask_block_start[tid].is_empty() && idx.mask_block_count[tid] > 0 {
        return;
    }
    for i in 0..idx.mask_block_count[tid] as usize {
        let block_start = idx.mask_block_start[tid][i];
        let mut block_end = block_start + idx.mask_block_sizes[tid][i];
        if block_end <= start { continue; }
        if block_start >= end { break; }
        let pos;
        let width;
        if block_start < start {
            block_end = block_end.min(end);
            pos = 0u32;
            width = block_end - start;
        } else {
            block_end = block_end.min(end);
            pos = block_start - start;
            width = block_end - block_start;
        }
        for p in pos..pos + width {
            if seq[p as usize] != 'N' {
                seq[p as usize] = seq[p as usize].to_ascii_lowercase();
            }
        }
    }
}

fn get_mask(idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32,
            mask_idx: &mut u32, mask_start: &mut u32, mask_end: &mut u32) {
    if *mask_idx == u32::MAX {
        for mi in 0..idx.n_block_count[tid] {
            *mask_idx = mi;
            *mask_start = idx.n_block_start[tid][mi as usize];
            *mask_end = *mask_start + idx.n_block_sizes[tid][mi as usize];
            if *mask_end < start { continue; }
            if *mask_end >= start { break; }
        }
        if idx.n_block_count[tid] == 0 {
            *mask_idx = 0;
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

fn twobit_bases_worker(data: &[u8], idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32, fraction: i32) -> Vec<u8> {
    let mut tmp: [u32; 4] = [0, 0, 0, 0];
    let seq_len = end - start;
    let len = end - start + (start % 4);

    let block_start = (start / 4) as usize;
    let initial_offset = (start % 4) as i32;
    let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as usize;
    let byte_len = block_end - block_start;
    let data_offset = idx.offset[tid] as usize + block_start;
    let bytes = &data[data_offset..data_offset + byte_len];

    let mut mask: u8 = get_byte_mask_from_offset(initial_offset);
    let mut i: u32 = 0;
    let mut j: usize = 0;

    let mut mask_idx = u32::MAX;
    let mut mask_start = 0u32;
    let mut mask_end = 0u32;
    // Use start aligned to 4-byte boundary for mask calculations
    let aligned_start = 4 * (block_start as u32);

    get_mask(idx, tid, aligned_start, end, &mut mask_idx, &mut mask_start, &mut mask_end);

    while i < len {
        // Check if we need to handle N-mask
        if mask_idx != u32::MAX && aligned_start + i + 4 >= mask_start {
            if aligned_start + i >= mask_start || aligned_start + i + 4 - (0/* offset always 0 after init */) > mask_start {
                // Jump if whole byte is inside N block
                if aligned_start + i >= mask_start && aligned_start + i + 4 < mask_end {
                    let new_pos = mask_end - aligned_start;
                    i = new_pos;
                    get_mask(idx, tid, i, end, &mut mask_idx, &mut mask_start, &mut mask_end);
                    let offset = (aligned_start + i) % 4;
                    j = (i / 4) as usize;
                    mask = get_byte_mask_from_offset(offset as i32);
                    i = 4 * j as u32;
                    continue;
                }

                // Set the mask for partial overlap
                let foo = 4 * j as u32 + 4 * block_start as u32;
                if mask & 1 != 0 && foo + 3 >= mask_start && foo + 3 < mask_end { mask -= 1; }
                if mask & 2 != 0 && foo + 2 >= mask_start && foo + 2 < mask_end { mask -= 2; }
                if mask & 4 != 0 && foo + 1 >= mask_start && foo + 1 < mask_end { mask -= 4; }
                if mask & 8 != 0 && foo >= mask_start && foo < mask_end { mask -= 8; }
                if foo + 4 > mask_end {
                    get_mask(idx, tid, i, end, &mut mask_idx, &mut mask_start, &mut mask_end);
                    continue;
                }
            }
        }

        // Mask anything past the end
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

    // tmp is in TCAG order (indices 0=T, 1=C, 2=A, 3=G)
    // Output is ACTG order
    if fraction != 0 {
        let sl = seq_len as f64;
        let vals: [f64; 4] = [
            tmp[2] as f64 / sl, // A
            tmp[1] as f64 / sl, // C
            tmp[0] as f64 / sl, // T
            tmp[3] as f64 / sl, // G
        ];
        // Return raw bytes of f64 array
        let mut out = Vec::with_capacity(32);
        for v in &vals {
            out.extend_from_slice(&v.to_ne_bytes());
        }
        out
    } else {
        let vals: [u32; 4] = [tmp[2], tmp[1], tmp[0], tmp[3]];
        let mut out = Vec::with_capacity(16);
        for v in &vals {
            out.extend_from_slice(&v.to_ne_bytes());
        }
        out
    }
}
