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

const MAGIC: u32 = 0x1A41_2743;

impl TwoBit {
    pub fn twobit_open(fname: &str, store_masked: bool) -> Self {
        let mut fp = File::open(fname).expect("Failed to open 2bit file");
        let sz = fp
            .metadata()
            .expect("Failed to stat file")
            .len();
        // Read entire file into a buffer (mimicking the mmap behavior)
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
        self.twoBitIndexDestroy();
        self.twobitChromListDestroy();
        self.twobitHdrDestroy();
        self.data.clear();
    }

    pub fn twobit_chrom_len(&self, chrom: &str) -> u32 {
        for i in 0..(self.hdr.n_chroms as usize) {
            if self.cl.chrom[i] == chrom {
                return self.idx.size[i];
            }
        }
        0
    }

    pub fn twobit_sequence(&self, chrom: &str, start: u32, end: u32) -> String {
        // Find chrom id
        let n = self.hdr.n_chroms as usize;
        let mut tid: Option<usize> = None;
        for i in 0..n {
            if self.cl.chrom[i] == chrom {
                tid = Some(i);
                break;
            }
        }
        let tid = match tid {
            Some(i) => i,
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

        // Construct the sequence
        let chars = self.construct_sequence_pure(tid as u32, start, end);
        chars.into_iter().collect()
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
            Some(i) => i,
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

        let counts = self.compute_base_counts(tid as u32, start, end);
        let _ = fraction;

        // C uses TCAG ordering, output as ACTG
        // counts[0]=T, counts[1]=C, counts[2]=A, counts[3]=G
        // out[0]=A=counts[2], out[1]=C=counts[1], out[2]=T=counts[0], out[3]=G=counts[3]
        // Return 4 byte values with the integer counts (clamped to u8 range).
        let vals: [u32; 4] = [counts[2], counts[1], counts[0], counts[3]];
        let mut out = Vec::with_capacity(4);
        for v in &vals {
            out.push((*v).min(255) as u8);
        }
        out
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // Mimics fread-like behavior: advance offset by sz*nmemb if possible
        let total = sz * nmemb;
        if self.offset as usize + total > self.data.len() {
            return 0;
        }
        self.offset += total as u64;
        nmemb
    }

    pub fn twobitSeek(&mut self, offset: u64) {
        if offset >= self.sz {
            return;
        }
        self.offset = offset;
    }

    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        n_mask_pure(seq, &self.idx, tid, start, end);
    }

    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        soft_mask_pure(seq, &self.idx, tid, start, end);
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        self.construct_sequence_pure(tid, start, end)
    }

    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        // Returns (maskIdx, maskStart, maskEnd) for the first overlapping N-block.
        // Mimics getMask called with maskIdx == -1 (initial call).
        let tidu = tid as usize;
        let n_count = self.idx.n_block_count[tidu];
        if n_count == 0 {
            return (u32::MAX, u32::MAX, u32::MAX);
        }
        let mut mask_idx: u32 = 0;
        let mut mask_start: u32 = 0;
        let mut mask_end: u32 = 0;
        while mask_idx < n_count {
            mask_start = self.idx.n_block_start[tidu][mask_idx as usize];
            mask_end = mask_start + self.idx.n_block_sizes[tidu][mask_idx as usize];
            if mask_end < start {
                mask_idx += 1;
                continue;
            }
            if mask_end >= start {
                break;
            }
        }
        if mask_idx >= n_count || mask_start >= end {
            return (mask_idx, u32::MAX, u32::MAX);
        }
        (mask_idx, mask_start, mask_end)
    }

    pub fn twoBitBasesWorker(&mut self, tid: u32, start: u32, end: u32, _fraction: i32) {
        // Underlying counts get computed in `compute_base_counts`.
        let _ = self.compute_base_counts(tid, start, end);
    }

    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n = self.hdr.n_chroms as usize;
        let mut size: Vec<u32> = Vec::with_capacity(n);
        let mut n_block_count: Vec<u32> = vec![0; n];
        let mut n_block_start: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut n_block_sizes: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut mask_block_count: Vec<u32> = vec![0; n];
        let mut mask_block_start: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut mask_block_sizes: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut offsets: Vec<u64> = Vec::with_capacity(n);

        for i in 0..n {
            let off = self.cl.offset[i] as u64;
            self.seek(off);

            let chrom_size = self.read_u32();
            let nblk = self.read_u32();
            size.push(chrom_size);
            n_block_count[i] = nblk;

            let mut starts = Vec::with_capacity(nblk as usize);
            for _ in 0..nblk {
                starts.push(self.read_u32());
            }
            let mut sizes = Vec::with_capacity(nblk as usize);
            for _ in 0..nblk {
                sizes.push(self.read_u32());
            }
            n_block_start[i] = starts;
            n_block_sizes[i] = sizes;

            let mblk = self.read_u32();
            mask_block_count[i] = mblk;

            if storeMasked != 0 {
                let mut mstarts = Vec::with_capacity(mblk as usize);
                for _ in 0..mblk {
                    mstarts.push(self.read_u32());
                }
                let mut msizes = Vec::with_capacity(mblk as usize);
                for _ in 0..mblk {
                    msizes.push(self.read_u32());
                }
                mask_block_start[i] = mstarts;
                mask_block_sizes[i] = msizes;
            } else {
                let new_off = self.tell() + 8 * mblk as u64;
                self.seek(new_off);
            }

            // Reserved 4 bytes
            let _reserved = self.read_u32();
            offsets.push(self.tell());
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
            let mut bytes = Vec::with_capacity(len);
            for _ in 0..len {
                bytes.push(self.read_u8());
            }
            let s = String::from_utf8(bytes).expect("Invalid UTF-8 in chrom name");
            chrom.push(s);
            offset.push(self.read_u32());
        }

        self.cl = TwoBitCL { chrom, offset };
    }

    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }

    pub fn twobitHdrRead(&mut self) {
        self.offset = 0;
        let magic = self.read_u32();
        let version = self.read_u32();
        let n_chroms = self.read_u32();
        let _reserved = self.read_u32();

        if magic != MAGIC {
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
}

// ===================== Helper methods =====================

impl TwoBit {
    fn seek(&mut self, offset: u64) {
        if offset > self.sz {
            return;
        }
        self.offset = offset;
    }

    fn tell(&self) -> u64 {
        self.offset
    }

    fn read_u8(&mut self) -> u8 {
        let b = self.data[self.offset as usize];
        self.offset += 1;
        b
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

    fn construct_sequence_pure(&self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let sz = (end - start) as usize;

        // 4 bases/byte
        let block_start = (start / 4) as usize;
        let offset = (start % 4) as i32;
        let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as usize;

        let file_off = self.idx.offset[tid as usize] as usize + block_start;
        let bytes_len = block_end - block_start;
        let mut bytes: Vec<u8> = self.data[file_off..file_off + bytes_len].to_vec();

        let mut seq: Vec<char> = vec!['\0'; sz];
        bytes2bases(&mut seq, &mut bytes, sz as u32, offset);

        // N-mask
        n_mask_pure(&mut seq, &self.idx, tid, start, end);

        // Soft-mask
        soft_mask_pure(&mut seq, &self.idx, tid, start, end);

        seq
    }

    /// Returns counts in TCAG order (matching the C internal `tmp` array).
    fn compute_base_counts(&self, tid: u32, start: u32, end: u32) -> [u32; 4] {
        let mut tmp = [0u32; 4];

        let tidu = tid as usize;

        // 4 bases/byte
        let block_start = (start / 4) as usize;
        let initial_offset = (start % 4) as i32;
        let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as usize;

        let file_off = self.idx.offset[tidu] as usize + block_start;
        let bytes_len = block_end - block_start;
        let bytes: Vec<u8> = self.data[file_off..file_off + bytes_len].to_vec();

        // Initial mask
        let mut mask: u8 = get_byte_mask_from_offset(initial_offset);
        let aligned_start = (4 * block_start) as u32;
        let len = (end - aligned_start) as u32;

        // N-block traversal state
        let n_count = self.idx.n_block_count[tidu];
        let mut mask_idx: u32 = u32::MAX; // -1 sentinel
        let mut mask_start: u32 = 0;
        let mut mask_end: u32 = 0;

        // Initial getMask call: find first N-block whose end >= aligned_start
        get_mask_state(
            &self.idx,
            tid,
            aligned_start,
            end,
            &mut mask_idx,
            &mut mask_start,
            &mut mask_end,
        );

        let mut i: u32 = 0;
        let mut j: usize = 0;

        while i < len {
            // Check if we need to handle a mask boundary
            if mask_idx != u32::MAX && aligned_start + i + 4 >= mask_start {
                // Note: we cannot compute aligned_start+i+4-offset safely if it underflows;
                // The C compares `start + i + 4 - offset > maskStart` where offset can be 0
                // (after the initial setup). After init, offset is reset to 0.
                let cur_offset: u32 = 0; // after init, offset is 0 in the C code

                if aligned_start + i >= mask_start
                    || aligned_start + i + 4 - cur_offset > mask_start
                {
                    // Jump iff the whole byte is inside an N block
                    if aligned_start + i >= mask_start
                        && aligned_start + i + 4 - cur_offset < mask_end
                    {
                        // Fully in N block, jump
                        i = mask_end - aligned_start;
                        get_mask_state(
                            &self.idx,
                            tid,
                            i,
                            end,
                            &mut mask_idx,
                            &mut mask_start,
                            &mut mask_end,
                        );
                        let new_offset = ((aligned_start + i) % 4) as i32;
                        j = (i / 4) as usize;
                        mask = get_byte_mask_from_offset(new_offset);
                        i = 4 * j as u32;
                        // offset is reset to 0
                        continue;
                    }

                    // Set the mask, if appropriate
                    let foo_pos: u32 = 4 * j as u32 + 4 * block_start as u32;
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
                        get_mask_state(
                            &self.idx,
                            tid,
                            i,
                            end,
                            &mut mask_idx,
                            &mut mask_start,
                            &mut mask_end,
                        );
                        // Note: don't advance i here, just refresh mask state and continue loop
                        // But we still need to fall through and process this byte using the
                        // current (possibly modified) mask. Actually the C code does
                        // `continue` here, going back to the while loop top.
                        continue;
                    }
                }
            }

            // Ensure that anything after the end is masked off
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

            let mut foo: u32 = bytes[j] as u32;
            j += 1;

            // Offset 3
            if mask & 1 != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            mask >>= 1;
            // Offset 2
            if mask & 1 != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            mask >>= 1;
            // Offset 1
            if mask & 1 != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            foo >>= 2;
            mask >>= 1;
            // Offset 0
            if mask & 1 != 0 {
                tmp[(foo & 3) as usize] += 1;
            }

            i += 4;
            mask = 15;
        }

        let _ = n_count; // silence unused warning if any
        tmp
    }
}

// ===================== Free functions =====================

pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let mask: u8 = 3u8 << (2 * rev);
    let foo = ((mask & byte) >> (2 * rev)) as usize;
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo]
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    bytes2bases_inner(seq, bytes, sz, offset);
}

// Internal version that takes a non-mut byte slice (so we can call it from
// places where we don't have a mutable byte buffer).
fn bytes2bases_inner(seq: &mut [char], bytes: &[u8], sz: u32, offset: i32) {
    let bases = ['T', 'C', 'A', 'G'];
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let mut offset = offset;
    let sz = sz as usize;

    let mut foo: u8 = bytes[0];

    // Deal with the first partial byte
    if offset != 0 {
        while offset < 4 && (pos as usize) < sz {
            seq[pos as usize] = byte2base(foo, offset);
            pos += 1;
            offset += 1;
        }
        if (pos as usize) >= sz {
            return;
        }
        i += 1;
        foo = bytes[i];
    }

    // Deal with everything else, with the possible exception of the last fractional byte
    let remainder = ((sz - pos as usize) % 4) as u32;
    while (pos as usize) < sz - remainder as usize {
        foo = bytes[i];
        i += 1;
        seq[pos as usize + 3] = bases[(foo & 3) as usize];
        let f1 = foo >> 2;
        seq[pos as usize + 2] = bases[(f1 & 3) as usize];
        let f2 = f1 >> 2;
        seq[pos as usize + 1] = bases[(f2 & 3) as usize];
        let f3 = f2 >> 2;
        seq[pos as usize] = bases[(f3 & 3) as usize];
        pos += 4;
    }

    // Deal with the last partial byte
    if remainder > 0 {
        foo = bytes[i];
    }
    let mut off: i32 = 0;
    while off < remainder as i32 {
        seq[pos as usize] = byte2base(foo, off);
        pos += 1;
        off += 1;
    }
}

pub fn getByteMaskFromOffset(_offset: i32) {
    // C returns a uint8_t; the Rust signature provided returns nothing, so
    // we provide an internal helper used by the implementation (`get_byte_mask_from_offset`).
}

fn get_byte_mask_from_offset(offset: i32) -> u8 {
    match offset {
        0 => 15,
        1 => 7,
        2 => 3,
        _ => 1,
    }
}

fn n_mask_pure(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: u32, start: u32, end: u32) {
    let tidu = tid as usize;
    let cnt = idx.n_block_count[tidu];
    let mut pos: u32;
    let mut width: u32;
    for i in 0..cnt as usize {
        let block_start = idx.n_block_start[tidu][i];
        let mut block_end = block_start + idx.n_block_sizes[tidu][i];
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

fn soft_mask_pure(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: u32, start: u32, end: u32) {
    let tidu = tid as usize;
    if idx.mask_block_start.is_empty() || idx.mask_block_start.iter().all(|v| v.is_empty())
        && idx.mask_block_count.iter().all(|&c| c == 0)
    {
        // mimic C's `if(!tb->idx->maskBlockStart) return;` - we approximate by
        // "no mask data stored". This still allows iterating the per-tid vec below.
    }
    let cnt = idx.mask_block_count[tidu];
    if (idx.mask_block_start.get(tidu).map(|v| v.is_empty()).unwrap_or(true)) && cnt > 0 {
        // Soft mask info wasn't loaded; nothing to do.
        return;
    }
    let mut pos: u32;
    let mut width: u32;
    for i in 0..cnt as usize {
        if i >= idx.mask_block_start[tidu].len() {
            return;
        }
        let block_start = idx.mask_block_start[tidu][i];
        let mut block_end = block_start + idx.mask_block_sizes[tidu][i];
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
            if seq[pos as usize] != 'N' {
                let c = seq[pos as usize];
                seq[pos as usize] = c.to_ascii_lowercase();
            }
            pos += 1;
        }
    }
}

fn get_mask_state(
    idx: &TwoBitMaskedIdx,
    tid: u32,
    start: u32,
    end: u32,
    mask_idx: &mut u32,
    mask_start: &mut u32,
    mask_end: &mut u32,
) {
    let tidu = tid as usize;
    let n_count = idx.n_block_count[tidu];

    if *mask_idx == u32::MAX {
        // Initial: scan from beginning
        let mut idx_local: u32 = 0;
        while idx_local < n_count {
            *mask_start = idx.n_block_start[tidu][idx_local as usize];
            *mask_end = *mask_start + idx.n_block_sizes[tidu][idx_local as usize];
            if *mask_end < start {
                idx_local += 1;
                continue;
            }
            // mask_end >= start
            break;
        }
        *mask_idx = idx_local;
    } else if *mask_idx >= n_count {
        *mask_start = u32::MAX;
        *mask_end = u32::MAX;
    } else {
        *mask_idx += 1;
        if *mask_idx >= n_count {
            *mask_start = u32::MAX;
            *mask_end = u32::MAX;
        } else {
            *mask_start = idx.n_block_start[tidu][*mask_idx as usize];
            *mask_end = *mask_start + idx.n_block_sizes[tidu][*mask_idx as usize];
        }
    }

    if *mask_idx >= n_count || *mask_start >= end {
        *mask_start = u32::MAX;
        *mask_end = u32::MAX;
    }
}

