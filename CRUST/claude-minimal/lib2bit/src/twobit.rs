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
        let mut fp = File::open(fname).expect("Cannot open 2bit file");
        let sz = fp.metadata().expect("Cannot get metadata").len();
        let mut data = vec![0u8; sz as usize];
        fp.read_exact(&mut data).expect("Cannot read file");

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

    pub fn twobit_sequence(&mut self, chrom: &str, start: u32, end: u32) -> String {
        // Find tid
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

        let seq = self.constructSequence(tid as u32, start, end);
        seq.iter().collect::<String>()
    }

    pub fn twobit_bases(&mut self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<f64> {
        // Find tid
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

        self.twoBitBasesWorker(tid as u32, start, end, fraction)
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // The original C signature wants to read into data, but the Rust signature
        // takes &Vec<u8> (immutable). We can't write back through this signature, so
        // we provide a no-op-style helper that simply advances the offset by sz*nmemb.
        // Internal code uses direct slice access into self.data instead.
        let _ = data;
        let total = sz * nmemb;
        if (self.offset as usize) + total > self.data.len() {
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
        let tid = tid as usize;
        for i in 0..self.idx.n_block_count[tid] as usize {
            let block_start = self.idx.n_block_start[tid][i];
            let block_end_raw = block_start + self.idx.n_block_sizes[tid][i];
            if block_end_raw <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let mut pos: u32;
            let width: u32;
            let block_end = if block_end_raw < end { block_end_raw } else { end };
            if block_start < start {
                pos = 0;
                width = block_end - start;
            } else {
                pos = block_start - start;
                width = block_end - block_start;
            }
            let total = width + pos;
            while pos < total {
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
        for i in 0..self.idx.mask_block_count[tid] as usize {
            let block_start = self.idx.mask_block_start[tid][i];
            let block_end_raw = block_start + self.idx.mask_block_sizes[tid][i];
            if block_end_raw <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let mut pos: u32;
            let width: u32;
            let block_end = if block_end_raw < end { block_end_raw } else { end };
            if block_start < start {
                pos = 0;
                width = block_end - start;
            } else {
                pos = block_start - start;
                width = block_end - block_start;
            }
            let total = width + pos;
            while pos < total {
                if seq[pos as usize] != 'N' {
                    seq[pos as usize] = seq[pos as usize].to_ascii_lowercase();
                }
                pos += 1;
            }
        }
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let sz = (end - start) as usize;
        let mut seq: Vec<char> = vec!['\0'; sz];

        let block_start = (start / 4) as u64;
        let offset = (start % 4) as i32;
        let block_end = ((end / 4) + if end % 4 != 0 { 1 } else { 0 }) as u64;
        let bytes_len = (block_end - block_start) as usize;
        let mut bytes = vec![0u8; bytes_len];

        let read_offset = self.idx.offset[tid as usize] + block_start;
        self.twobitSeek(read_offset);
        let off = self.offset as usize;
        bytes.copy_from_slice(&self.data[off..off + bytes_len]);
        self.offset += bytes_len as u64;

        bytes2bases(&mut seq, &mut bytes, sz as u32, offset);

        // N-mask everything
        self.NMask(&mut seq, tid, start, end);

        // Soft-mask if requested
        self.softMask(&mut seq, tid, start, end);

        seq
    }

    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        // First-call semantics: maskIdx == -1 path of the C function.
        // For iterative use within twoBitBasesWorker, we use getMaskIter below.
        self.getMaskIter(tid, start, end, u32::MAX)
    }

    fn getMaskIter(&self, tid: u32, start: u32, end: u32, mask_idx_in: u32) -> (u32, u32, u32) {
        let tid = tid as usize;
        let n_count = self.idx.n_block_count[tid];
        let mut mask_idx = mask_idx_in;
        let mut mask_start: u32 = u32::MAX;
        let mut mask_end: u32 = u32::MAX;

        if mask_idx == u32::MAX {
            // First call: find first overlapping block
            mask_idx = 0;
            while mask_idx < n_count {
                mask_start = self.idx.n_block_start[tid][mask_idx as usize];
                mask_end = mask_start + self.idx.n_block_sizes[tid][mask_idx as usize];
                if mask_end < start {
                    mask_idx += 1;
                    continue;
                }
                if mask_end >= start {
                    break;
                }
                mask_idx += 1;
            }
        } else if mask_idx >= n_count {
            mask_start = u32::MAX;
            mask_end = u32::MAX;
        } else {
            mask_idx += 1;
            if mask_idx >= n_count {
                mask_start = u32::MAX;
                mask_end = u32::MAX;
            } else {
                mask_start = self.idx.n_block_start[tid][mask_idx as usize];
                mask_end = mask_start + self.idx.n_block_sizes[tid][mask_idx as usize];
            }
        }

        if mask_idx >= n_count || mask_start >= end {
            mask_start = u32::MAX;
            mask_end = u32::MAX;
        }

        (mask_idx, mask_start, mask_end)
    }

    pub fn twoBitBasesWorker(
        &mut self,
        tid: u32,
        start: u32,
        end: u32,
        fraction: i32,
    ) -> Vec<f64> {
        let tid_usize = tid as usize;
        let mut tmp = [0u32; 4];
        let seq_len = end - start;

        // Compute alignment values (matching the C logic)
        let original_start = start;
        let block_start = original_start / 4;
        let offset_initial = original_start % 4;
        let block_end = end / 4 + if end % 4 != 0 { 1 } else { 0 };
        let bytes_len = (block_end - block_start) as usize;
        let mut bytes = vec![0u8; bytes_len];

        // len = end - start_original + (start_original % 4)
        let len: u32 = end - original_start + offset_initial;
        let mut mask: u8 = getByteMaskFromOffset(offset_initial as i32);
        let aligned_start: u32 = 4 * block_start;
        let mut offset: u32 = 0;

        // Read the bytes
        let read_offset = self.idx.offset[tid_usize] + block_start as u64;
        self.twobitSeek(read_offset);
        let off = self.offset as usize;
        bytes.copy_from_slice(&self.data[off..off + bytes_len]);
        self.offset += bytes_len as u64;

        // Get the index/start/end of the next N-mask block
        let (mut mask_idx, mut mask_start, mut mask_end) =
            self.getMaskIter(tid, aligned_start, end, u32::MAX);

        let mut i: u32 = 0;
        let mut j: u32 = 0;

        while i < len {
            // Check if we need to jump
            if mask_idx != u32::MAX && aligned_start + i + 4 >= mask_start {
                if aligned_start + i >= mask_start
                    || aligned_start + i + 4 - offset > mask_start
                {
                    // Jump iff the whole byte is inside an N block
                    if aligned_start + i >= mask_start
                        && aligned_start + i + 4 - offset < mask_end
                    {
                        i = mask_end - aligned_start;
                        let (mi, ms, me) = self.getMaskIter(tid, i, end, mask_idx);
                        mask_idx = mi;
                        mask_start = ms;
                        mask_end = me;
                        offset = (aligned_start + i) % 4;
                        j = i / 4;
                        mask = getByteMaskFromOffset(offset as i32);
                        i = 4 * j;
                        offset = 0;
                        continue;
                    }

                    // Set the mask, if appropriate
                    let foo_pos = 4 * j + 4 * block_start;
                    if (mask & 1) != 0
                        && (foo_pos + 3 >= mask_start && foo_pos + 3 < mask_end)
                    {
                        mask -= 1;
                    }
                    if (mask & 2) != 0
                        && (foo_pos + 2 >= mask_start && foo_pos + 2 < mask_end)
                    {
                        mask -= 2;
                    }
                    if (mask & 4) != 0
                        && (foo_pos + 1 >= mask_start && foo_pos + 1 < mask_end)
                    {
                        mask -= 4;
                    }
                    if (mask & 8) != 0 && (foo_pos >= mask_start && foo_pos < mask_end) {
                        mask -= 8;
                    }
                    if foo_pos + 4 > mask_end {
                        let (mi, ms, me) = self.getMaskIter(tid, i, end, mask_idx);
                        mask_idx = mi;
                        mask_start = ms;
                        mask_end = me;
                        continue;
                    }
                }
            }

            // Ensure that anything after the end is masked
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

        // tmp is in TCAG order (since 2bit stores it that way).
        // The C output is in ACTG order.
        let mut out: Vec<f64> = Vec::with_capacity(4);
        if fraction != 0 {
            out.push((tmp[2] as f64) / (seq_len as f64));
            out.push((tmp[1] as f64) / (seq_len as f64));
            out.push((tmp[0] as f64) / (seq_len as f64));
            out.push((tmp[3] as f64) / (seq_len as f64));
        } else {
            out.push(tmp[2] as f64);
            out.push(tmp[1] as f64);
            out.push(tmp[0] as f64);
            out.push(tmp[3] as f64);
        }

        out
    }

    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n_chroms = self.hdr.n_chroms as usize;
        self.idx.size = vec![0u32; n_chroms];
        self.idx.n_block_count = vec![0u32; n_chroms];
        self.idx.n_block_start = vec![Vec::new(); n_chroms];
        self.idx.n_block_sizes = vec![Vec::new(); n_chroms];
        self.idx.mask_block_count = vec![0u32; n_chroms];
        if storeMasked != 0 {
            self.idx.mask_block_start = vec![Vec::new(); n_chroms];
            self.idx.mask_block_sizes = vec![Vec::new(); n_chroms];
        }
        self.idx.offset = vec![0u64; n_chroms];

        for i in 0..n_chroms {
            let chrom_offset = self.cl.offset[i] as u64;
            self.twobitSeek(chrom_offset);

            // Read size and nBlockCount
            let size = self.read_u32();
            let n_block_count = self.read_u32();
            self.idx.size[i] = size;
            self.idx.n_block_count[i] = n_block_count;

            // Read n_block_start array
            let mut starts = Vec::with_capacity(n_block_count as usize);
            for _ in 0..n_block_count {
                starts.push(self.read_u32());
            }
            self.idx.n_block_start[i] = starts;

            // Read n_block_sizes array
            let mut sizes = Vec::with_capacity(n_block_count as usize);
            for _ in 0..n_block_count {
                sizes.push(self.read_u32());
            }
            self.idx.n_block_sizes[i] = sizes;

            // Read maskBlockCount
            let mask_count = self.read_u32();
            self.idx.mask_block_count[i] = mask_count;

            if storeMasked != 0 {
                let mut mstarts = Vec::with_capacity(mask_count as usize);
                for _ in 0..mask_count {
                    mstarts.push(self.read_u32());
                }
                self.idx.mask_block_start[i] = mstarts;

                let mut msizes = Vec::with_capacity(mask_count as usize);
                for _ in 0..mask_count {
                    msizes.push(self.read_u32());
                }
                self.idx.mask_block_sizes[i] = msizes;
            } else {
                // Skip 8 * mask_count bytes
                let new_offset = self.twobitTell() + 8u64 * mask_count as u64;
                self.twobitSeek(new_offset);
            }

            // Reserved u32
            let _reserved = self.read_u32();

            self.idx.offset[i] = self.twobitTell();
        }
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
        self.cl.chrom = Vec::with_capacity(n_chroms);
        self.cl.offset = Vec::with_capacity(n_chroms);

        for _ in 0..n_chroms {
            // Read string length byte
            let byte = self.read_u8();
            let str_len = byte as usize;

            // Read the string itself
            let off = self.offset as usize;
            let s = String::from_utf8(self.data[off..off + str_len].to_vec())
                .expect("Invalid UTF-8 in chromosome name");
            self.offset += str_len as u64;
            self.cl.chrom.push(s);

            // Read the chromosome's file offset
            let chrom_offset = self.read_u32();
            self.cl.offset.push(chrom_offset);
        }
    }

    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }

    pub fn twobitHdrRead(&mut self) {
        let magic = self.read_u32();
        let version = self.read_u32();
        let n_chroms = self.read_u32();
        let _reserved = self.read_u32();

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

        self.hdr.magic = magic;
        self.hdr.version = version;
        self.hdr.n_chroms = n_chroms;
    }

    pub fn twobitHdrDestroy(&mut self) {
        self.hdr.magic = 0;
        self.hdr.version = 0;
        self.hdr.n_chroms = 0;
    }

    // Internal helpers for reading from self.data at self.offset
    fn read_u32(&mut self) -> u32 {
        let off = self.offset as usize;
        let val = u32::from_le_bytes([
            self.data[off],
            self.data[off + 1],
            self.data[off + 2],
            self.data[off + 3],
        ]);
        self.offset += 4;
        val
    }

    fn read_u8(&mut self) -> u8 {
        let val = self.data[self.offset as usize];
        self.offset += 1;
        val
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
    let mut pos: usize = 0;
    let mut i: usize = 0;
    let bases = ['T', 'C', 'A', 'G'];
    let mut foo: u8 = bytes[0];
    let mut offset = offset;
    let sz = sz as usize;

    // Deal with the first partial byte
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
        foo = bytes[i];
    }

    // Main loop: process full bytes
    let remainder = (sz - pos) % 4;
    while pos < sz - remainder {
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

    // Deal with the last partial byte
    if remainder > 0 {
        foo = bytes[i];
    }
    let mut o: i32 = 0;
    while o < remainder as i32 {
        seq[pos] = byte2base(foo, o);
        pos += 1;
        o += 1;
    }
}

pub fn getByteMaskFromOffset(offset: i32) -> u8 {
    match offset {
        0 => 15u8,
        1 => 7u8,
        2 => 3u8,
        _ => 1u8,
    }
}
