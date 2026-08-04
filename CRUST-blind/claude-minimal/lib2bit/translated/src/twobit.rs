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
        self.twobitChromListDestroy();
        self.twoBitIndexDestroy();
        self.twobitHdrDestroy();
        self.data.clear();
        self.sz = 0;
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
        // Find chromosome ID
        let n_chroms = self.hdr.n_chroms as usize;
        let mut tid: usize = 0;
        let mut found = false;
        for i in 0..n_chroms {
            if self.cl.chrom[i] == chrom {
                tid = i;
                found = true;
                break;
            }
        }
        if !found {
            return String::new();
        }

        let start = start;
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

        self.construct_sequence_internal(tid, start, end)
    }
    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let n_chroms = self.hdr.n_chroms as usize;
        let mut tid: usize = 0;
        let mut found = false;
        for i in 0..n_chroms {
            if self.cl.chrom[i] == chrom {
                tid = i;
                found = true;
                break;
            }
        }
        if !found {
            return Vec::new();
        }

        let start = start;
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

        self.bases_worker_internal(tid, start, end, fraction)
    }
    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }
    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // The signature passes an immutable Vec<u8> reference (we cannot write into it).
        // We mimic fread/memcpy semantics by advancing the offset over (sz * nmemb) bytes
        // and returning nmemb on success, 0 if the read would be out of bounds.
        let total = sz.checked_mul(nmemb).unwrap_or(0);
        let new_offset = (self.offset as usize).checked_add(total).unwrap_or(usize::MAX);
        if new_offset > self.data.len() {
            return 0;
        }
        self.offset = new_offset as u64;
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
            let mut block_end = block_start + self.idx.n_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let pos: u32;
            let width: u32;
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
            let total_width = width + pos;
            let mut p = pos;
            while p < total_width {
                if (p as usize) < seq.len() {
                    seq[p as usize] = 'N';
                }
                p += 1;
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
            let mut block_end = block_start + self.idx.mask_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let pos: u32;
            let width: u32;
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
            let total_width = width + pos;
            let mut p = pos;
            while p < total_width {
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
        // Returns a Vec<char> containing the bases of the requested range.
        let s = self.construct_sequence_internal(tid as usize, start, end);
        s.chars().collect()
    }
    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        // Mirrors the "initial call" path of the C getMask (where maskIdx == -1):
        // returns (maskIdx, maskStart, maskEnd) for the first overlapping N-block,
        // or (nBlockCount, u32::MAX, u32::MAX) if there is no overlap.
        let tid = tid as usize;
        let mut mask_idx: u32 = u32::MAX;
        let mut mask_start: u32 = u32::MAX;
        let mut mask_end: u32 = u32::MAX;
        self.get_mask_internal(tid, start, end, &mut mask_idx, &mut mask_start, &mut mask_end);
        (mask_idx, mask_start, mask_end)
    }
    pub fn twoBitBasesWorker(&mut self, _tid: u32, _start: u32, _end: u32, _fraction: i32) {
        // The Rust signature has no return value, so this is a stub.
        // Real work is done in `bases_worker_internal`, called from `twobit_bases`.
    }
    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n_chroms = self.hdr.n_chroms as usize;

        self.idx.size = Vec::with_capacity(n_chroms);
        self.idx.n_block_count = Vec::with_capacity(n_chroms);
        self.idx.n_block_start = Vec::with_capacity(n_chroms);
        self.idx.n_block_sizes = Vec::with_capacity(n_chroms);
        self.idx.mask_block_count = Vec::with_capacity(n_chroms);
        if storeMasked != 0 {
            self.idx.mask_block_start = Vec::with_capacity(n_chroms);
            self.idx.mask_block_sizes = Vec::with_capacity(n_chroms);
        } else {
            self.idx.mask_block_start = Vec::new();
            self.idx.mask_block_sizes = Vec::new();
        }
        self.idx.offset = Vec::with_capacity(n_chroms);

        for i in 0..n_chroms {
            // Seek to this chromosome's index entry
            self.offset = self.cl.offset[i] as u64;

            let size = self.read_u32();
            let n_block_count = self.read_u32();

            self.idx.size.push(size);
            self.idx.n_block_count.push(n_block_count);

            let mut starts: Vec<u32> = Vec::with_capacity(n_block_count as usize);
            for _ in 0..n_block_count {
                starts.push(self.read_u32());
            }
            let mut sizes: Vec<u32> = Vec::with_capacity(n_block_count as usize);
            for _ in 0..n_block_count {
                sizes.push(self.read_u32());
            }
            self.idx.n_block_start.push(starts);
            self.idx.n_block_sizes.push(sizes);

            let mask_block_count = self.read_u32();
            self.idx.mask_block_count.push(mask_block_count);

            if storeMasked != 0 {
                let mut mstarts: Vec<u32> = Vec::with_capacity(mask_block_count as usize);
                for _ in 0..mask_block_count {
                    mstarts.push(self.read_u32());
                }
                let mut msizes: Vec<u32> = Vec::with_capacity(mask_block_count as usize);
                for _ in 0..mask_block_count {
                    msizes.push(self.read_u32());
                }
                self.idx.mask_block_start.push(mstarts);
                self.idx.mask_block_sizes.push(msizes);
            } else {
                // Skip the mask-block start/size arrays (8 bytes per entry)
                self.offset += 8 * mask_block_count as u64;
            }

            // Reserved (4 bytes)
            let _reserved = self.read_u32();

            // Current offset is the start of this chromosome's packed sequence data.
            self.idx.offset.push(self.offset);
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
            // Read the name length (one byte; not null terminated)
            let name_len = self.read_u8() as usize;
            let start = self.offset as usize;
            let end = start + name_len;
            let bytes = self.data[start..end].to_vec();
            self.offset += name_len as u64;
            let name = String::from_utf8(bytes).expect("Invalid UTF-8 in chromosome name");
            self.cl.chrom.push(name);

            // Read the file offset of the chromosome's index entry
            let off = self.read_u32();
            self.cl.offset.push(off);
        }
    }
    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }
    pub fn twobitHdrRead(&mut self) {
        // The fixed 16-byte header
        self.offset = 0;
        let magic = self.read_u32();
        let version = self.read_u32();
        let n_chroms = self.read_u32();
        let _reserved = self.read_u32();

        if magic != 0x1A412743 {
            panic!(
                "[twobitHdrRead] Received an invalid file magic number (0x{:x})!",
                magic
            );
        }
        if version != 0 {
            panic!(
                "[twobitHdrRead] The file version is {} while only version 0 is defined!",
                version
            );
        }
        if n_chroms == 0 {
            panic!("[twobitHdrRead] There are no chromosomes/contigs in this file!");
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
}

// ============================================================================
// Internal helpers (not part of the public API surface preserved above)
// ============================================================================
impl TwoBit {
    fn read_u8(&mut self) -> u8 {
        let b = self.data[self.offset as usize];
        self.offset += 1;
        b
    }

    fn read_u32(&mut self) -> u32 {
        let s = self.offset as usize;
        let v = u32::from_le_bytes([
            self.data[s],
            self.data[s + 1],
            self.data[s + 2],
            self.data[s + 3],
        ]);
        self.offset += 4;
        v
    }

    fn construct_sequence_internal(&self, tid: usize, start: u32, end: u32) -> String {
        // Number of bases in the requested range (no null terminator in Rust strings)
        let sz = (end - start) as usize;

        // 4 bases per byte
        let block_start = start / 4;
        let offset = (start % 4) as i32;
        let block_end = end / 4 + if end % 4 != 0 { 1 } else { 0 };
        let n_bytes = (block_end - block_start) as usize;

        let file_offset = self.idx.offset[tid] + block_start as u64;
        let bytes = &self.data[file_offset as usize..file_offset as usize + n_bytes];

        // Decode packed bases into individual characters
        let mut seq: Vec<char> = vec!['T'; sz];
        bytes2bases_internal(&mut seq, bytes, sz as u32, offset);

        // N-mask (hard mask)
        self.n_mask_seq(&mut seq, tid, start, end);
        // Soft mask (lower case) if mask info is loaded
        self.soft_mask_seq(&mut seq, tid, start, end);

        seq.into_iter().collect()
    }

    fn n_mask_seq(&self, seq: &mut [char], tid: usize, start: u32, end: u32) {
        for i in 0..self.idx.n_block_count[tid] as usize {
            let block_start = self.idx.n_block_start[tid][i];
            let mut block_end = block_start + self.idx.n_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let pos: u32;
            let width: u32;
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
            let total = width + pos;
            for p in pos..total {
                if (p as usize) < seq.len() {
                    seq[p as usize] = 'N';
                }
            }
        }
    }

    fn soft_mask_seq(&self, seq: &mut [char], tid: usize, start: u32, end: u32) {
        if self.idx.mask_block_start.is_empty() {
            return;
        }
        for i in 0..self.idx.mask_block_count[tid] as usize {
            let block_start = self.idx.mask_block_start[tid][i];
            let mut block_end = block_start + self.idx.mask_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            let pos: u32;
            let width: u32;
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
            let total = width + pos;
            for p in pos..total {
                if (p as usize) < seq.len() {
                    let c = seq[p as usize];
                    if c != 'N' {
                        seq[p as usize] = c.to_ascii_lowercase();
                    }
                }
            }
        }
    }

    fn get_mask_internal(
        &self,
        tid: usize,
        start: u32,
        end: u32,
        mask_idx: &mut u32,
        mask_start: &mut u32,
        mask_end: &mut u32,
    ) {
        let n_block_count = self.idx.n_block_count[tid];
        if *mask_idx == u32::MAX {
            // Initial call: scan forward to find first block whose end >= start
            *mask_idx = 0;
            while *mask_idx < n_block_count {
                *mask_start = self.idx.n_block_start[tid][*mask_idx as usize];
                *mask_end = *mask_start + self.idx.n_block_sizes[tid][*mask_idx as usize];
                if *mask_end < start {
                    *mask_idx += 1;
                    continue;
                }
                break;
            }
        } else if *mask_idx >= n_block_count {
            *mask_start = u32::MAX;
            *mask_end = u32::MAX;
        } else {
            *mask_idx += 1;
            if *mask_idx >= n_block_count {
                *mask_start = u32::MAX;
                *mask_end = u32::MAX;
            } else {
                *mask_start = self.idx.n_block_start[tid][*mask_idx as usize];
                *mask_end = *mask_start + self.idx.n_block_sizes[tid][*mask_idx as usize];
            }
        }

        if *mask_idx >= n_block_count || *mask_start >= end {
            *mask_start = u32::MAX;
            *mask_end = u32::MAX;
        }
    }

    fn bases_worker_internal(&self, tid: usize, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let mut tmp: [u32; 4] = [0; 4];
        let seq_len = end - start;
        let mut start = start;
        let len = end - start + (start % 4);
        let mut i: u32 = 0;
        let mut j: u32 = 0;

        // 4 bases / byte
        let block_start = start / 4;
        let mut offset = (start % 4) as u32;
        let block_end = end / 4 + if end % 4 != 0 { 1 } else { 0 };
        let n_bytes = (block_end - block_start) as usize;

        // Initial mask, then reset start/offset so we always deal with full bytes
        let mut mask: u8 = get_byte_mask_from_offset_internal(offset as i32);
        start = 4 * block_start;
        offset = 0;

        let file_offset = self.idx.offset[tid] + block_start as u64;
        let bytes = &self.data[file_offset as usize..file_offset as usize + n_bytes];

        let mut mask_idx: u32 = u32::MAX;
        let mut mask_start: u32 = u32::MAX;
        let mut mask_end: u32 = u32::MAX;

        self.get_mask_internal(
            tid,
            start,
            end,
            &mut mask_idx,
            &mut mask_start,
            &mut mask_end,
        );

        while i < len {
            // Are we approaching/inside an N-block?
            if mask_idx != u32::MAX && start + i + 4 >= mask_start {
                if start + i >= mask_start || start + i + 4 - offset > mask_start {
                    // If the whole byte is inside an N-block, jump past it
                    if start + i >= mask_start && start + i + 4 - offset < mask_end {
                        i = mask_end - start;
                        self.get_mask_internal(
                            tid,
                            i,
                            end,
                            &mut mask_idx,
                            &mut mask_start,
                            &mut mask_end,
                        );
                        offset = (start + i) % 4;
                        j = i / 4;
                        mask = get_byte_mask_from_offset_internal(offset as i32);
                        i = 4 * j;
                        offset = 0;
                        continue;
                    }

                    // Mask out individual bases that fall inside the N-block
                    let foo_pos = 4 * j + 4 * block_start;
                    if (mask & 1) != 0 && foo_pos + 3 >= mask_start && foo_pos + 3 < mask_end {
                        mask -= 1;
                    }
                    if (mask & 2) != 0 && foo_pos + 2 >= mask_start && foo_pos + 2 < mask_end {
                        mask -= 2;
                    }
                    if (mask & 4) != 0 && foo_pos + 1 >= mask_start && foo_pos + 1 < mask_end {
                        mask -= 4;
                    }
                    if (mask & 8) != 0 && foo_pos >= mask_start && foo_pos < mask_end {
                        mask -= 8;
                    }
                    if foo_pos + 4 > mask_end {
                        self.get_mask_internal(
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

            // Mask out anything past the requested end
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
            j += 1;
            // Offset 3 (low 2 bits)
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
            // Offset 0 (high 2 bits)
            if (mask & 1) != 0 {
                tmp[(foo & 3) as usize] += 1;
            }
            i += 4;
            mask = 15;
        }

        // tmp is in TCAG order (the on-disk 2bit ordering); reorder to ACTG
        // for the public API, matching the C library's behavior.
        let mut out: Vec<u8> = Vec::new();
        if fraction != 0 {
            let vals: [f64; 4] = [
                tmp[2] as f64 / seq_len as f64,
                tmp[1] as f64 / seq_len as f64,
                tmp[0] as f64 / seq_len as f64,
                tmp[3] as f64 / seq_len as f64,
            ];
            for v in vals.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        } else {
            let vals: [u32; 4] = [tmp[2], tmp[1], tmp[0], tmp[3]];
            for v in vals.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }
}

// Helper function: decode the `offset`'th base packed in `byte`
pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = (3 - offset) as u32;
    let mask: u8 = 3u8 << (2 * rev);
    let foo = ((mask & byte) >> (2 * rev)) as usize;
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo]
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    bytes2bases_internal(seq, bytes, sz, offset);
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
            seq[pos as usize] = byte2base(foo, offset);
            offset += 1;
            pos += 1;
        }
        if pos >= sz {
            return;
        }
        i += 1;
        foo = bytes[i];
    }
    let _ = foo; // foo is reassigned in the loop below

    // Process whole bytes (4 bases each), with the possible exception of the last partial byte
    let remainder = (sz - pos) % 4;
    while pos < sz - remainder {
        let mut f: u8 = bytes[i];
        i += 1;
        seq[(pos + 3) as usize] = bases[(f & 3) as usize];
        f >>= 2;
        seq[(pos + 2) as usize] = bases[(f & 3) as usize];
        f >>= 2;
        seq[(pos + 1) as usize] = bases[(f & 3) as usize];
        f >>= 2;
        seq[pos as usize] = bases[(f & 3) as usize];
        pos += 4;
    }

    // Last partial byte
    if remainder > 0 {
        let last = bytes[i];
        for off in 0..remainder as i32 {
            seq[pos as usize] = byte2base(last, off);
            pos += 1;
        }
    }
}

pub fn getByteMaskFromOffset(_offset: i32) {
    // The Rust signature has no return value, so this is intentionally a no-op.
    // The actual mask logic lives in `get_byte_mask_from_offset_internal`.
}

fn get_byte_mask_from_offset_internal(offset: i32) -> u8 {
    match offset {
        0 => 15, // 0b1111 — all 4 bases of the byte are valid
        1 => 7,  // 0b0111
        2 => 3,  // 0b0011
        _ => 1,  // 0b0001
    }
}
