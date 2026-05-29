use std::convert::TryInto;
use std::fs::File;
use std::io::Read;

const U32_INVALID: u32 = u32::MAX;

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
        let mut data = Vec::new();
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
    }
    pub fn twobit_chrom_len(&self, chrom: &str) -> u32 {
        let n_chroms = self.hdr.n_chroms as usize;
        for i in 0..n_chroms {
            if i < self.cl.chrom.len() && self.cl.chrom[i] == chrom {
                if i < self.idx.size.len() {
                    return self.idx.size[i];
                }
                return 0;
            }
        }
        0
    }
    pub fn twobit_sequence(&self, chrom: &str, start: u32, end: u32) -> String {
        let mut tid: Option<u32> = None;
        for i in 0..self.hdr.n_chroms as usize {
            if i < self.cl.chrom.len() && self.cl.chrom[i] == chrom {
                tid = Some(i as u32);
                break;
            }
        }
        let tid = match tid {
            Some(t) => t,
            None => return String::new(),
        };
        let tid_u = tid as usize;
        let mut end = end;
        if start == 0 && end == 0 {
            end = self.idx.size[tid_u];
        }
        if end > self.idx.size[tid_u] {
            return String::new();
        }
        if start >= end {
            return String::new();
        }
        let chars = self.construct_sequence_internal(tid, start, end);
        chars.into_iter().collect()
    }
    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let mut tid: Option<u32> = None;
        for i in 0..self.hdr.n_chroms as usize {
            if i < self.cl.chrom.len() && self.cl.chrom[i] == chrom {
                tid = Some(i as u32);
                break;
            }
        }
        let tid = match tid {
            Some(t) => t,
            None => return Vec::new(),
        };
        let tid_u = tid as usize;
        let mut end = end;
        if start == 0 && end == 0 {
            end = self.idx.size[tid_u];
        }
        if end > self.idx.size[tid_u] {
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
        let total = sz.saturating_mul(nmemb);
        let cur = self.offset as usize;
        if cur + total > self.data.len() {
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
        let tid_u = tid as usize;
        if tid_u >= self.idx.n_block_count.len() {
            return;
        }
        let count = self.idx.n_block_count[tid_u] as usize;
        for i in 0..count {
            let block_start = self.idx.n_block_start[tid_u][i];
            let block_size = self.idx.n_block_sizes[tid_u][i];
            let mut block_end = block_start + block_size;
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
            let total = pos + width;
            let mut p = pos;
            while p < total {
                let idx = p as usize;
                if idx < seq.len() {
                    seq[idx] = 'N';
                }
                p += 1;
            }
        }
    }
    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid_u = tid as usize;
        if self.idx.mask_block_start.is_empty() {
            return;
        }
        if tid_u >= self.idx.mask_block_count.len() {
            return;
        }
        let count = self.idx.mask_block_count[tid_u] as usize;
        if tid_u >= self.idx.mask_block_start.len() {
            return;
        }
        for i in 0..count {
            if i >= self.idx.mask_block_start[tid_u].len() {
                break;
            }
            let block_start = self.idx.mask_block_start[tid_u][i];
            let block_size = self.idx.mask_block_sizes[tid_u][i];
            let mut block_end = block_start + block_size;
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
            let total = pos + width;
            let mut p = pos;
            while p < total {
                let idx = p as usize;
                if idx < seq.len() && seq[idx] != 'N' {
                    seq[idx] = seq[idx].to_ascii_lowercase();
                }
                p += 1;
            }
        }
    }
    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        self.construct_sequence_internal(tid, start, end)
    }
    pub fn getMask(&mut self, _tid: u32, _start: u32, _end: u32) -> (u32, u32, u32) {
        (U32_INVALID, U32_INVALID, U32_INVALID)
    }
    pub fn twoBitBasesWorker(&mut self, _tid: u32, _start: u32, _end: u32, _fraction: i32) {
        // The actual base counting is done internally via bases_worker_internal,
        // because the public signature returns () and cannot return the result.
    }
    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n_chroms = self.hdr.n_chroms as usize;
        let mut size: Vec<u32> = Vec::with_capacity(n_chroms);
        let mut n_block_count: Vec<u32> = Vec::with_capacity(n_chroms);
        let mut n_block_start: Vec<Vec<u32>> = Vec::with_capacity(n_chroms);
        let mut n_block_sizes: Vec<Vec<u32>> = Vec::with_capacity(n_chroms);
        let mut mask_block_count: Vec<u32> = Vec::with_capacity(n_chroms);
        let mut mask_block_start: Vec<Vec<u32>> = Vec::with_capacity(n_chroms);
        let mut mask_block_sizes: Vec<Vec<u32>> = Vec::with_capacity(n_chroms);
        let mut offset_vec: Vec<u64> = Vec::with_capacity(n_chroms);

        for i in 0..n_chroms {
            let chrom_off = self.cl.offset[i] as u64;
            self.offset = chrom_off;
            let sz = match self.read_u32() {
                Some(v) => v,
                None => return,
            };
            let nbc = match self.read_u32() {
                Some(v) => v,
                None => return,
            };
            size.push(sz);
            n_block_count.push(nbc);
            let nstart = match self.read_u32_n(nbc as usize) {
                Some(v) => v,
                None => return,
            };
            let nsizes = match self.read_u32_n(nbc as usize) {
                Some(v) => v,
                None => return,
            };
            n_block_start.push(nstart);
            n_block_sizes.push(nsizes);

            let mbc = match self.read_u32() {
                Some(v) => v,
                None => return,
            };
            mask_block_count.push(mbc);

            if storeMasked != 0 {
                let mstart = match self.read_u32_n(mbc as usize) {
                    Some(v) => v,
                    None => return,
                };
                let msizes = match self.read_u32_n(mbc as usize) {
                    Some(v) => v,
                    None => return,
                };
                mask_block_start.push(mstart);
                mask_block_sizes.push(msizes);
            } else {
                let new_off = self.offset + 8 * (mbc as u64);
                if new_off > self.sz {
                    return;
                }
                self.offset = new_off;
            }

            // Reserved
            let _reserved = match self.read_u32() {
                Some(v) => v,
                None => return,
            };
            offset_vec.push(self.offset);
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
        let mut chroms: Vec<String> = Vec::with_capacity(n_chroms);
        let mut offsets: Vec<u32> = Vec::with_capacity(n_chroms);
        for _ in 0..n_chroms {
            let len = match self.read_u8() {
                Some(v) => v as usize,
                None => return,
            };
            let bytes = match self.read_bytes(len) {
                Some(v) => v,
                None => return,
            };
            let s = match String::from_utf8(bytes) {
                Ok(v) => v,
                Err(_) => return,
            };
            chroms.push(s);
            let off = match self.read_u32() {
                Some(v) => v,
                None => return,
            };
            offsets.push(off);
        }
        self.cl = TwoBitCL {
            chrom: chroms,
            offset: offsets,
        };
    }
    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }
    pub fn twobitHdrRead(&mut self) {
        let magic = match self.read_u32() {
            Some(v) => v,
            None => return,
        };
        let version = match self.read_u32() {
            Some(v) => v,
            None => return,
        };
        let n_chroms = match self.read_u32() {
            Some(v) => v,
            None => return,
        };
        // 4th uint32 is reserved
        let _reserved = match self.read_u32() {
            Some(v) => v,
            None => return,
        };

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
            eprintln!(
                "[twobitHdrRead] There are apparently no chromosomes/contigs in this file!"
            );
            return;
        }

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

// Private helper methods
impl TwoBit {
    fn read_u8(&mut self) -> Option<u8> {
        let off = self.offset as usize;
        if off >= self.data.len() {
            return None;
        }
        let v = self.data[off];
        self.offset += 1;
        Some(v)
    }
    fn read_u32(&mut self) -> Option<u32> {
        let off = self.offset as usize;
        if off + 4 > self.data.len() {
            return None;
        }
        let v = u32::from_le_bytes(self.data[off..off + 4].try_into().unwrap());
        self.offset += 4;
        Some(v)
    }
    fn read_u32_n(&mut self, n: usize) -> Option<Vec<u32>> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(self.read_u32()?);
        }
        Some(out)
    }
    fn read_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        let off = self.offset as usize;
        if off + n > self.data.len() {
            return None;
        }
        let v = self.data[off..off + n].to_vec();
        self.offset += n as u64;
        Some(v)
    }

    fn construct_sequence_internal(&self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let tid_u = tid as usize;
        let sz = (end - start + 1) as usize;
        let mut seq: Vec<char> = vec!['\0'; sz];

        // 4 bases per byte
        let block_start = start / 4;
        let offset = (start % 4) as i32;
        let block_end = end / 4 + if end % 4 != 0 { 1 } else { 0 };
        let byte_count = (block_end - block_start) as usize;
        let read_off = (self.idx.offset[tid_u] + block_start as u64) as usize;
        if read_off + byte_count > self.data.len() {
            return Vec::new();
        }
        let mut bytes = self.data[read_off..read_off + byte_count].to_vec();

        bytes2bases(&mut seq, &mut bytes, (sz - 1) as u32, offset);
        // null-terminator slot is the last char; we'll truncate before returning

        // N-mask
        n_mask_static(&mut seq[..sz - 1], &self.idx, tid, start, end);
        // soft-mask
        soft_mask_static(&mut seq[..sz - 1], &self.idx, tid, start, end);

        // Drop the trailing null char to give the caller a clean string-of-bases
        seq.truncate(sz - 1);
        seq
    }

    fn bases_worker_internal(&self, tid: u32, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let tid_u = tid as usize;
        let mut tmp: [u32; 4] = [0; 4];
        let mut start = start;
        let len = end - start + (start % 4);
        let seq_len = end - start;
        let block_start = start / 4;
        let mut offset: u32 = start % 4;
        let block_end = end / 4 + if end % 4 != 0 { 1 } else { 0 };
        let byte_count = (block_end - block_start) as usize;
        let read_off = (self.idx.offset[tid_u] + block_start as u64) as usize;
        if read_off + byte_count > self.data.len() {
            return Vec::new();
        }
        let bytes = &self.data[read_off..read_off + byte_count];

        let mut mask: u8 = get_byte_mask_from_offset_internal(offset as i32);
        start = 4 * block_start;
        offset = 0;

        let mut mask_idx: u32 = U32_INVALID;
        let mut mask_start: u32 = U32_INVALID;
        let mut mask_end: u32 = U32_INVALID;

        get_mask_static(
            &self.idx,
            tid,
            start,
            end,
            &mut mask_idx,
            &mut mask_start,
            &mut mask_end,
        );

        let mut i: u32 = 0;
        let mut j: u32 = 0;

        while i < len {
            // Check whether we need to jump
            if mask_idx != U32_INVALID && start + i + 4 >= mask_start {
                if start + i >= mask_start || start + i + 4 - offset > mask_start {
                    // Jump iff the whole byte is inside an N block
                    if start + i >= mask_start && start + i + 4 - offset < mask_end {
                        i = mask_end - start;
                        get_mask_static(
                            &self.idx,
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

                    // Set the mask, if appropriate
                    let foo = 4 * j + 4 * block_start;
                    if (mask & 1) != 0 && (foo + 3 >= mask_start && foo + 3 < mask_end) {
                        mask -= 1;
                    }
                    if (mask & 2) != 0 && (foo + 2 >= mask_start && foo + 2 < mask_end) {
                        mask -= 2;
                    }
                    if (mask & 4) != 0 && (foo + 1 >= mask_start && foo + 1 < mask_end) {
                        mask -= 4;
                    }
                    if (mask & 8) != 0 && (foo >= mask_start && foo < mask_end) {
                        mask -= 8;
                    }
                    if foo + 4 > mask_end {
                        get_mask_static(
                            &self.idx,
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

            let mut foo: u32 = bytes[j as usize] as u32;
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

        // 2bit storage order is TCAG, but the public API exposes ACTG.
        let mut out: Vec<u8> = Vec::new();
        if fraction != 0 {
            let seq_len_f = seq_len as f64;
            let a = (tmp[2] as f64) / seq_len_f;
            let c = (tmp[1] as f64) / seq_len_f;
            let t = (tmp[0] as f64) / seq_len_f;
            let g = (tmp[3] as f64) / seq_len_f;
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
}

// --- helper functions (free / module-level) ---

fn n_mask_static(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: u32, start: u32, end: u32) {
    let tid_u = tid as usize;
    if tid_u >= idx.n_block_count.len() {
        return;
    }
    let count = idx.n_block_count[tid_u] as usize;
    for i in 0..count {
        let block_start = idx.n_block_start[tid_u][i];
        let block_size = idx.n_block_sizes[tid_u][i];
        let mut block_end = block_start + block_size;
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
        let total = pos + width;
        let mut p = pos;
        while p < total {
            let pi = p as usize;
            if pi < seq.len() {
                seq[pi] = 'N';
            }
            p += 1;
        }
    }
}

fn soft_mask_static(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: u32, start: u32, end: u32) {
    if idx.mask_block_start.is_empty() {
        return;
    }
    let tid_u = tid as usize;
    if tid_u >= idx.mask_block_count.len() || tid_u >= idx.mask_block_start.len() {
        return;
    }
    let count = idx.mask_block_count[tid_u] as usize;
    for i in 0..count {
        if i >= idx.mask_block_start[tid_u].len() {
            break;
        }
        let block_start = idx.mask_block_start[tid_u][i];
        let block_size = idx.mask_block_sizes[tid_u][i];
        let mut block_end = block_start + block_size;
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
        let total = pos + width;
        let mut p = pos;
        while p < total {
            let pi = p as usize;
            if pi < seq.len() && seq[pi] != 'N' {
                seq[pi] = seq[pi].to_ascii_lowercase();
            }
            p += 1;
        }
    }
}

fn get_mask_static(
    idx: &TwoBitMaskedIdx,
    tid: u32,
    start: u32,
    end: u32,
    mask_idx: &mut u32,
    mask_start: &mut u32,
    mask_end: &mut u32,
) {
    let tid_u = tid as usize;
    let n_block_count = if tid_u < idx.n_block_count.len() {
        idx.n_block_count[tid_u]
    } else {
        0
    };

    if *mask_idx == U32_INVALID {
        let mut mi: u32 = 0;
        while mi < n_block_count {
            *mask_start = idx.n_block_start[tid_u][mi as usize];
            *mask_end = *mask_start + idx.n_block_sizes[tid_u][mi as usize];
            if *mask_end < start {
                mi += 1;
                continue;
            }
            // *mask_end >= start
            *mask_idx = mi;
            break;
        }
        if mi >= n_block_count {
            *mask_idx = mi;
        }
    } else if *mask_idx >= n_block_count {
        *mask_start = U32_INVALID;
        *mask_end = U32_INVALID;
    } else {
        *mask_idx += 1;
        if *mask_idx >= n_block_count {
            *mask_start = U32_INVALID;
            *mask_end = U32_INVALID;
        } else {
            *mask_start = idx.n_block_start[tid_u][*mask_idx as usize];
            *mask_end = *mask_start + idx.n_block_sizes[tid_u][*mask_idx as usize];
        }
    }

    if *mask_idx >= n_block_count || *mask_start >= end {
        *mask_start = U32_INVALID;
        *mask_end = U32_INVALID;
    }
}

fn get_byte_mask_from_offset_internal(offset: i32) -> u8 {
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
    let foo: usize = ((mask & byte) >> (2 * rev)) as usize;
    let bases = ['T', 'C', 'A', 'G'];
    bases[foo]
}
pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let bases = ['T', 'C', 'A', 'G'];
    if bytes.is_empty() {
        return;
    }
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
        if i < bytes.len() {
            foo = bytes[i];
        }
    }

    // Full bytes (with possible fractional last byte)
    let remainder = (sz - pos) % 4;
    while pos < sz - remainder {
        if i >= bytes.len() {
            return;
        }
        let mut b = bytes[i];
        i += 1;
        let p = pos as usize;
        seq[p + 3] = bases[(b & 3) as usize];
        b >>= 2;
        seq[p + 2] = bases[(b & 3) as usize];
        b >>= 2;
        seq[p + 1] = bases[(b & 3) as usize];
        b >>= 2;
        seq[p] = bases[(b & 3) as usize];
        pos += 4;
    }

    // Last partial byte
    if remainder > 0 {
        if i >= bytes.len() {
            return;
        }
        foo = bytes[i];
    }
    let mut o: i32 = 0;
    while o < remainder as i32 {
        seq[pos as usize] = byte2base(foo, o);
        pos += 1;
        o += 1;
    }
}
pub fn getByteMaskFromOffset(_offset: i32) {
    // The original C function returns a u8, but the public Rust signature
    // returns (). The internal implementation lives in
    // `get_byte_mask_from_offset_internal`.
}
