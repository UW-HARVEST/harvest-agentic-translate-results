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
        let mut fp = File::open(fname).expect("Unable to open 2bit file");
        let mut data: Vec<u8> = Vec::new();
        fp.read_to_end(&mut data)
            .expect("Unable to read 2bit file");
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
        if tb.hdr.magic != 0x1A412743 {
            eprintln!(
                "[twobitHdrRead] Received an invalid file magic number (0x{:x})!",
                tb.hdr.magic
            );
            return tb;
        }
        if tb.hdr.version != 0 {
            eprintln!(
                "[twobitHdrRead] The file version is {} while only version 0 is defined!",
                tb.hdr.version
            );
            return tb;
        }
        if tb.hdr.n_chroms == 0 {
            eprintln!(
                "[twobitHdrRead] There are apparently no chromosomes/contigs in this file!"
            );
            return tb;
        }

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
        self.sz = 0;
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
        let mut tid_opt: Option<usize> = None;
        for i in 0..self.hdr.n_chroms as usize {
            if self.cl.chrom[i] == chrom {
                tid_opt = Some(i);
                break;
            }
        }
        let tid = match tid_opt {
            Some(t) => t,
            None => return String::new(),
        };

        let mut e = end;
        if start == 0 && end == 0 {
            e = self.idx.size[tid];
        }
        if e > self.idx.size[tid] {
            return String::new();
        }
        if start >= e {
            return String::new();
        }

        let sz = (e - start) as usize;
        let block_start = (start / 4) as usize;
        let initial_offset = (start % 4) as i32;
        let block_end = (e / 4 + if e % 4 != 0 { 1 } else { 0 }) as usize;
        let data_offset = self.idx.offset[tid] as usize + block_start;

        let bytes: Vec<u8> = self.data[data_offset..data_offset + (block_end - block_start)].to_vec();

        let mut seq: Vec<char> = vec!['\0'; sz];
        bases_from_bytes(&mut seq, &bytes, sz as u32, initial_offset);

        // N-mask
        for i in 0..self.idx.n_block_count[tid] as usize {
            let block_s = self.idx.n_block_start[tid][i];
            let block_e_init = block_s + self.idx.n_block_sizes[tid][i];
            if block_e_init <= start {
                continue;
            }
            if block_s >= e {
                break;
            }
            let block_e = if block_e_init < e { block_e_init } else { e };
            let (pos, width) = if block_s < start {
                (0u32, block_e - start)
            } else {
                (block_s - start, block_e - block_s)
            };
            for p in pos..(pos + width) {
                if (p as usize) < seq.len() {
                    seq[p as usize] = 'N';
                }
            }
        }

        // Soft-mask
        if !self.idx.mask_block_start.is_empty() && tid < self.idx.mask_block_start.len() {
            for i in 0..self.idx.mask_block_count[tid] as usize {
                let block_s = self.idx.mask_block_start[tid][i];
                let block_e_init = block_s + self.idx.mask_block_sizes[tid][i];
                if block_e_init <= start {
                    continue;
                }
                if block_s >= e {
                    break;
                }
                let block_e = if block_e_init < e { block_e_init } else { e };
                let (pos, width) = if block_s < start {
                    (0u32, block_e - start)
                } else {
                    (block_s - start, block_e - block_s)
                };
                for p in pos..(pos + width) {
                    if (p as usize) < seq.len() && seq[p as usize] != 'N' {
                        seq[p as usize] = seq[p as usize].to_ascii_lowercase();
                    }
                }
            }
        }

        seq.into_iter().collect()
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let mut tid_opt: Option<usize> = None;
        for i in 0..self.hdr.n_chroms as usize {
            if self.cl.chrom[i] == chrom {
                tid_opt = Some(i);
                break;
            }
        }
        let tid = match tid_opt {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut e = end;
        if start == 0 && end == 0 {
            e = self.idx.size[tid];
        }
        if e > self.idx.size[tid] {
            return Vec::new();
        }
        if start >= e {
            return Vec::new();
        }

        // Count bases. 2-bit encoding order: T=0, C=1, A=2, G=3
        let mut counts = [0u32; 4];
        let n_block_count = self.idx.n_block_count[tid] as usize;
        let data_offset = self.idx.offset[tid] as usize;

        let mut n_idx: usize = 0;
        for pos in start..e {
            // Advance past N-blocks that already ended
            while n_idx < n_block_count
                && self.idx.n_block_start[tid][n_idx] + self.idx.n_block_sizes[tid][n_idx] <= pos
            {
                n_idx += 1;
            }
            // If pos is inside the current N-block, skip
            if n_idx < n_block_count {
                let s = self.idx.n_block_start[tid][n_idx];
                let en = s + self.idx.n_block_sizes[tid][n_idx];
                if pos >= s && pos < en {
                    continue;
                }
            }
            // Decode 2-bit value at pos
            let byte_idx = (pos / 4) as usize;
            let bit_off = pos % 4;
            let byte = self.data[data_offset + byte_idx];
            let rev = 3 - bit_off;
            let val = ((byte >> (2 * rev)) & 0x03) as usize;
            counts[val] += 1;
        }

        // Output in ACTG order (matches C: tmp[2], tmp[1], tmp[0], tmp[3])
        let _ = fraction; // u8 cannot represent fractional values; return raw counts
        vec![
            counts[2] as u8,
            counts[1] as u8,
            counts[0] as u8,
            counts[3] as u8,
        ]
    }

    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        // The public signature provides &Vec<u8> (immutable), so we can't actually
        // copy data. We advance the internal offset by sz*nmemb if it fits.
        let total = sz.saturating_mul(nmemb);
        let new_off = (self.offset as usize).saturating_add(total);
        if new_off > self.data.len() {
            return 0;
        }
        self.offset = new_off as u64;
        nmemb
    }

    pub fn twobitSeek(&mut self, offset: u64) {
        if offset < self.sz {
            self.offset = offset;
        }
    }

    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid = tid as usize;
        for i in 0..self.idx.n_block_count[tid] as usize {
            let block_s = self.idx.n_block_start[tid][i];
            let block_e_init = block_s + self.idx.n_block_sizes[tid][i];
            if block_e_init <= start {
                continue;
            }
            if block_s >= end {
                break;
            }
            let block_e = if block_e_init < end { block_e_init } else { end };
            let (pos, width) = if block_s < start {
                (0u32, block_e - start)
            } else {
                (block_s - start, block_e - block_s)
            };
            for p in pos..(pos + width) {
                if (p as usize) < seq.len() {
                    seq[p as usize] = 'N';
                }
            }
        }
    }

    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        let tid = tid as usize;
        if self.idx.mask_block_start.is_empty() || tid >= self.idx.mask_block_start.len() {
            return;
        }
        for i in 0..self.idx.mask_block_count[tid] as usize {
            let block_s = self.idx.mask_block_start[tid][i];
            let block_e_init = block_s + self.idx.mask_block_sizes[tid][i];
            if block_e_init <= start {
                continue;
            }
            if block_s >= end {
                break;
            }
            let block_e = if block_e_init < end { block_e_init } else { end };
            let (pos, width) = if block_s < start {
                (0u32, block_e - start)
            } else {
                (block_s - start, block_e - block_s)
            };
            for p in pos..(pos + width) {
                if (p as usize) < seq.len() && seq[p as usize] != 'N' {
                    seq[p as usize] = seq[p as usize].to_ascii_lowercase();
                }
            }
        }
    }

    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        let tid_us = tid as usize;
        let sz = (end - start) as usize;
        let block_start = (start / 4) as usize;
        let initial_offset = (start % 4) as i32;
        let block_end = (end / 4 + if end % 4 != 0 { 1 } else { 0 }) as usize;
        let data_offset = self.idx.offset[tid_us] as usize + block_start;
        let bytes: Vec<u8> = self.data[data_offset..data_offset + (block_end - block_start)].to_vec();

        let mut seq: Vec<char> = vec!['\0'; sz];
        bases_from_bytes(&mut seq, &bytes, sz as u32, initial_offset);

        self.NMask(&mut seq, tid, start, end);
        self.softMask(&mut seq, tid, start, end);

        seq
    }

    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        // Stateless implementation: find the first N-block that overlaps [start, end).
        let tid_us = tid as usize;
        let n = self.idx.n_block_count[tid_us] as usize;
        for i in 0..n {
            let s = self.idx.n_block_start[tid_us][i];
            let e = s + self.idx.n_block_sizes[tid_us][i];
            if e < start {
                continue;
            }
            if s >= end {
                break;
            }
            return (i as u32, s, e);
        }
        (n as u32, u32::MAX, u32::MAX)
    }

    pub fn twoBitBasesWorker(&mut self, _tid: u32, _start: u32, _end: u32, _fraction: i32) {
        // The public twobit_bases performs the work directly. This helper is a
        // no-op since the Rust signature does not return data and there is no
        // internal field to store the result.
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
            self.offset = self.cl.offset[i] as u64;
            let size = self.read_u32_at();
            let n_block_count = self.read_u32_at();
            self.idx.size.push(size);
            self.idx.n_block_count.push(n_block_count);

            let mut n_starts: Vec<u32> = Vec::with_capacity(n_block_count as usize);
            for _ in 0..n_block_count {
                n_starts.push(self.read_u32_at());
            }
            let mut n_sizes: Vec<u32> = Vec::with_capacity(n_block_count as usize);
            for _ in 0..n_block_count {
                n_sizes.push(self.read_u32_at());
            }
            self.idx.n_block_start.push(n_starts);
            self.idx.n_block_sizes.push(n_sizes);

            let mask_block_count = self.read_u32_at();
            self.idx.mask_block_count.push(mask_block_count);

            if storeMasked != 0 {
                let mut m_starts: Vec<u32> = Vec::with_capacity(mask_block_count as usize);
                for _ in 0..mask_block_count {
                    m_starts.push(self.read_u32_at());
                }
                let mut m_sizes: Vec<u32> = Vec::with_capacity(mask_block_count as usize);
                for _ in 0..mask_block_count {
                    m_sizes.push(self.read_u32_at());
                }
                self.idx.mask_block_start.push(m_starts);
                self.idx.mask_block_sizes.push(m_sizes);
            } else {
                self.offset += 8u64 * mask_block_count as u64;
            }

            // Reserved
            let _reserved = self.read_u32_at();
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
        let n = self.hdr.n_chroms as usize;
        self.cl.chrom = Vec::with_capacity(n);
        self.cl.offset = Vec::with_capacity(n);
        for _ in 0..n {
            let len = self.read_u8_at() as usize;
            let s = self.offset as usize;
            let name = String::from_utf8_lossy(&self.data[s..s + len]).into_owned();
            self.offset += len as u64;
            self.cl.chrom.push(name);
            let off = self.read_u32_at();
            self.cl.offset.push(off);
        }
    }

    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }

    pub fn twobitHdrRead(&mut self) {
        // First 16 bytes: magic, version, n_chroms, reserved
        self.offset = 0;
        let magic = self.read_u32_at();
        let version = self.read_u32_at();
        let n_chroms = self.read_u32_at();
        let _reserved = self.read_u32_at();
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

    fn read_u32_at(&mut self) -> u32 {
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

    fn read_u8_at(&mut self) -> u8 {
        let v = self.data[self.offset as usize];
        self.offset += 1;
        v
    }
}

// Internal helper that performs the byte-to-base conversion. It mirrors the
// behavior of bytes2bases in the C source.
fn bases_from_bytes(seq: &mut [char], byte: &[u8], sz: u32, mut offset: i32) {
    let bases = ['T', 'C', 'A', 'G'];
    if sz == 0 || byte.is_empty() {
        return;
    }
    let mut pos: u32 = 0;
    let mut i: usize = 0;
    let mut foo: u8 = byte[0];

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
        if i < byte.len() {
            foo = byte[i];
        }
    }

    // Deal with everything else, possibly excluding the last fractional byte
    let remainder = (sz - pos) % 4;
    while pos < sz - remainder {
        if i >= byte.len() {
            break;
        }
        foo = byte[i];
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
    if remainder > 0 && i < byte.len() {
        foo = byte[i];
    }
    let mut off2: i32 = 0;
    while off2 < remainder as i32 && (pos as usize) < seq.len() {
        seq[pos as usize] = byte2base(foo, off2);
        pos += 1;
        off2 += 1;
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
    bases_from_bytes(seq, bytes, sz, offset);
}

pub fn getByteMaskFromOffset(offset: i32) {
    // The Rust signature returns nothing, so this is a no-op stub.
    // The internal bit-mask logic used by twobit_bases is implemented
    // directly in twobit_bases.
    let _ = match offset {
        0 => 15u8,
        1 => 7u8,
        2 => 3u8,
        _ => 1u8,
    };
}
