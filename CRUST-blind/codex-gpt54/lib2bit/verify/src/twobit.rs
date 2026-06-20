use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

const TWOBIT_MAGIC: u32 = 0x1A41_2743;

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
        let fp = File::open(fname).unwrap_or_else(|_| open_placeholder_file());
        let data = std::fs::read(fname).unwrap_or_default();
        let mut tb = Self {
            fp,
            sz: data.len() as u64,
            offset: 0,
            data,
            hdr: empty_header(),
            cl: empty_chrom_list(),
            idx: empty_index(),
        };

        tb.twobitHdrRead();
        if tb.hdr.magic != TWOBIT_MAGIC || tb.hdr.version != 0 || tb.hdr.n_chroms == 0 {
            tb.twobit_close();
            return tb;
        }

        tb.twobitChromListRead();
        if tb.cl.chrom.len() != tb.hdr.n_chroms as usize {
            tb.twobit_close();
            return tb;
        }

        tb.twoBitIndexRead(if store_masked { 1 } else { 0 });
        if tb.idx.size.len() != tb.hdr.n_chroms as usize {
            tb.twobit_close();
        }

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
        self.chrom_tid(chrom)
            .and_then(|tid| self.idx.size.get(tid).copied())
            .unwrap_or(0)
    }

    pub fn twobit_sequence(&self, chrom: &str, start: u32, end: u32) -> String {
        let Some((tid, start, end)) = self.resolve_range(chrom, start, end) else {
            return String::new();
        };

        self.construct_sequence_impl(tid, start, end)
            .into_iter()
            .collect()
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let Some((tid, start, end)) = self.resolve_range(chrom, start, end) else {
            return Vec::new();
        };

        self.twobit_bases_worker_impl(tid, start, end, fraction)
    }

    #[allow(non_snake_case)]
    pub fn twobitTell(&mut self) -> u64 {
        if self.data.is_empty() {
            self.fp.stream_position().unwrap_or(self.offset)
        } else {
            self.offset
        }
    }

    #[allow(non_snake_case)]
    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        if sz == 0 || nmemb == 0 {
            return 0;
        }

        let byte_count = sz.saturating_mul(nmemb);
        if self.data.is_empty() {
            let mut scratch = vec![0_u8; byte_count];
            return self.fp.read(&mut scratch).map(|n| n / sz).unwrap_or(0);
        }

        let available = self.sz.saturating_sub(self.offset) as usize;
        let readable = available.min(byte_count);
        self.offset += readable as u64;
        readable / sz
    }

    #[allow(non_snake_case)]
    pub fn twobitSeek(&mut self, offset: u64) {
        if offset > self.sz {
            return;
        }

        self.offset = offset;
        let _ = self.fp.seek(SeekFrom::Start(offset));
    }

    #[allow(non_snake_case)]
    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        apply_n_mask(seq, &self.idx, tid as usize, start, end);
    }

    #[allow(non_snake_case)]
    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        apply_soft_mask(seq, &self.idx, tid as usize, start, end);
    }

    #[allow(non_snake_case)]
    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        self.construct_sequence_impl(tid as usize, start, end)
    }

    #[allow(non_snake_case)]
    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        let tid = tid as usize;
        let Some(starts) = self.idx.n_block_start.get(tid) else {
            return (u32::MAX, u32::MAX, u32::MAX);
        };
        let Some(sizes) = self.idx.n_block_sizes.get(tid) else {
            return (u32::MAX, u32::MAX, u32::MAX);
        };

        for (idx, (&block_start, &block_size)) in starts.iter().zip(sizes.iter()).enumerate() {
            let block_end = block_start.saturating_add(block_size);
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }
            return (idx as u32, block_start, block_end);
        }

        (u32::MAX, u32::MAX, u32::MAX)
    }

    #[allow(non_snake_case)]
    pub fn twoBitBasesWorker(&mut self, tid: u32, start: u32, end: u32, fraction: i32) {
        let _ = self.twobit_bases_worker_impl(tid as usize, start, end, fraction);
    }

    #[allow(non_snake_case)]
    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n_chroms = self.hdr.n_chroms as usize;
        let mut idx = TwoBitMaskedIdx {
            size: vec![0; n_chroms],
            n_block_count: vec![0; n_chroms],
            n_block_start: vec![Vec::new(); n_chroms],
            n_block_sizes: vec![Vec::new(); n_chroms],
            mask_block_count: vec![0; n_chroms],
            mask_block_start: vec![Vec::new(); n_chroms],
            mask_block_sizes: vec![Vec::new(); n_chroms],
            offset: vec![0; n_chroms],
        };

        let store_masked = storeMasked == 1;
        let mut ok = true;

        for i in 0..n_chroms {
            let Some(mut cursor) = self.cl.offset.get(i).copied().map(|v| v as usize) else {
                ok = false;
                break;
            };

            let Some(size) = read_u32_at(&self.data, &mut cursor) else {
                ok = false;
                break;
            };
            let Some(n_block_count) = read_u32_at(&self.data, &mut cursor) else {
                ok = false;
                break;
            };

            idx.size[i] = size;
            idx.n_block_count[i] = n_block_count;

            let Some(n_starts) = read_u32_vec_at(&self.data, &mut cursor, n_block_count as usize) else {
                ok = false;
                break;
            };
            let Some(n_sizes) = read_u32_vec_at(&self.data, &mut cursor, n_block_count as usize) else {
                ok = false;
                break;
            };
            idx.n_block_start[i] = n_starts;
            idx.n_block_sizes[i] = n_sizes;

            let Some(mask_count) = read_u32_at(&self.data, &mut cursor) else {
                ok = false;
                break;
            };
            idx.mask_block_count[i] = mask_count;

            if store_masked {
                let Some(mask_starts) =
                    read_u32_vec_at(&self.data, &mut cursor, mask_count as usize)
                else {
                    ok = false;
                    break;
                };
                let Some(mask_sizes) =
                    read_u32_vec_at(&self.data, &mut cursor, mask_count as usize)
                else {
                    ok = false;
                    break;
                };
                idx.mask_block_start[i] = mask_starts;
                idx.mask_block_sizes[i] = mask_sizes;
            } else {
                let skip = (mask_count as usize).saturating_mul(8);
                if cursor.saturating_add(skip) > self.data.len() {
                    ok = false;
                    break;
                }
                cursor += skip;
            }

            if read_u32_at(&self.data, &mut cursor).is_none() {
                ok = false;
                break;
            }

            idx.offset[i] = cursor as u64;
            self.offset = cursor as u64;
        }

        self.idx = if ok { idx } else { empty_index() };
    }

    #[allow(non_snake_case)]
    pub fn twoBitIndexDestroy(&mut self) {
        self.idx = empty_index();
    }

    #[allow(non_snake_case)]
    pub fn twobitChromListRead(&mut self) {
        let n_chroms = self.hdr.n_chroms as usize;
        let mut chrom = Vec::with_capacity(n_chroms);
        let mut offset = Vec::with_capacity(n_chroms);
        let mut cursor = self.offset as usize;

        for _ in 0..n_chroms {
            let Some(name_len) = read_u8_at(&self.data, &mut cursor) else {
                self.cl = empty_chrom_list();
                return;
            };
            let Some(name_bytes) = read_bytes_at(&self.data, &mut cursor, name_len as usize) else {
                self.cl = empty_chrom_list();
                return;
            };
            let Some(entry_offset) = read_u32_at(&self.data, &mut cursor) else {
                self.cl = empty_chrom_list();
                return;
            };

            chrom.push(String::from_utf8_lossy(name_bytes).into_owned());
            offset.push(entry_offset);
        }

        self.offset = cursor as u64;
        self.cl = TwoBitCL { chrom, offset };
    }

    #[allow(non_snake_case)]
    pub fn twobitChromListDestroy(&mut self) {
        self.cl = empty_chrom_list();
    }

    #[allow(non_snake_case)]
    pub fn twobitHdrRead(&mut self) {
        let mut cursor = self.offset as usize;
        let Some(magic) = read_u32_at(&self.data, &mut cursor) else {
            self.hdr = empty_header();
            return;
        };
        let Some(version) = read_u32_at(&self.data, &mut cursor) else {
            self.hdr = empty_header();
            return;
        };
        let Some(n_chroms) = read_u32_at(&self.data, &mut cursor) else {
            self.hdr = empty_header();
            return;
        };
        if read_u32_at(&self.data, &mut cursor).is_none() {
            self.hdr = empty_header();
            return;
        }

        self.offset = cursor as u64;
        self.hdr = TwoBitHeader {
            magic,
            version,
            n_chroms,
        };
    }

    #[allow(non_snake_case)]
    pub fn twobitHdrDestroy(&mut self) {
        self.hdr = empty_header();
    }

    fn chrom_tid(&self, chrom: &str) -> Option<usize> {
        self.cl.chrom.iter().position(|name| name == chrom)
    }

    fn resolve_range(&self, chrom: &str, start: u32, end: u32) -> Option<(usize, u32, u32)> {
        let tid = self.chrom_tid(chrom)?;
        let chrom_len = *self.idx.size.get(tid)?;

        let resolved_end = if start == 0 && end == 0 { chrom_len } else { end };
        if resolved_end > chrom_len || start >= resolved_end {
            return None;
        }

        Some((tid, start, resolved_end))
    }

    fn construct_sequence_impl(&self, tid: usize, start: u32, end: u32) -> Vec<char> {
        if end <= start {
            return Vec::new();
        }

        let seq_len = (end - start) as usize;
        let packed_start = start as usize / 4;
        let packed_end = (end as usize + 3) / 4;
        let seq_offset = match self.idx.offset.get(tid).copied() {
            Some(offset) => offset as usize,
            None => return Vec::new(),
        };

        let byte_start = seq_offset.saturating_add(packed_start);
        let byte_end = seq_offset.saturating_add(packed_end);
        if byte_end > self.data.len() || byte_start > byte_end {
            return Vec::new();
        }

        let mut bytes = self.data[byte_start..byte_end].to_vec();
        let mut seq = vec!['\0'; seq_len];
        bytes2bases(&mut seq, &mut bytes, seq_len as u32, (start % 4) as i32);
        apply_n_mask(&mut seq, &self.idx, tid, start, end);
        apply_soft_mask(&mut seq, &self.idx, tid, start, end);
        seq
    }

    fn twobit_bases_worker_impl(&self, tid: usize, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let seq = self.construct_sequence_impl(tid, start, end);
        if seq.len() != (end - start) as usize {
            return Vec::new();
        }

        let mut counts = [0_u32; 4];
        for base in seq {
            match base.to_ascii_uppercase() {
                'A' => counts[0] += 1,
                'C' => counts[1] += 1,
                'T' => counts[2] += 1,
                'G' => counts[3] += 1,
                _ => {}
            }
        }

        if fraction != 0 {
            let denom = (end - start) as f64;
            let values = [
                counts[0] as f64 / denom,
                counts[1] as f64 / denom,
                counts[2] as f64 / denom,
                counts[3] as f64 / denom,
            ];
            let mut out = Vec::with_capacity(4 * std::mem::size_of::<f64>());
            for value in values {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out
        } else {
            let mut out = Vec::with_capacity(4 * std::mem::size_of::<u32>());
            for value in counts {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out
        }
    }
}

pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let shift = (2 * rev).max(0) as u32;
    match (byte >> shift) & 0b11 {
        0 => 'T',
        1 => 'C',
        2 => 'A',
        _ => 'G',
    }
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, offset: i32) {
    let limit = seq.len().min(sz as usize);
    for (i, slot) in seq.iter_mut().take(limit).enumerate() {
        let absolute = offset.max(0) as usize + i;
        let Some(byte) = bytes.get(absolute / 4).copied() else {
            break;
        };
        *slot = byte2base(byte, (absolute % 4) as i32);
    }
}

#[allow(non_snake_case)]
pub fn getByteMaskFromOffset(offset: i32) {
    let _ = get_byte_mask_from_offset(offset);
}

fn empty_header() -> TwoBitHeader {
    TwoBitHeader {
        magic: 0,
        version: 0,
        n_chroms: 0,
    }
}

fn empty_chrom_list() -> TwoBitCL {
    TwoBitCL {
        chrom: Vec::new(),
        offset: Vec::new(),
    }
}

fn empty_index() -> TwoBitMaskedIdx {
    TwoBitMaskedIdx {
        size: Vec::new(),
        n_block_count: Vec::new(),
        n_block_start: Vec::new(),
        n_block_sizes: Vec::new(),
        mask_block_count: Vec::new(),
        mask_block_start: Vec::new(),
        mask_block_sizes: Vec::new(),
        offset: Vec::new(),
    }
}

fn open_placeholder_file() -> File {
    let path = std::env::temp_dir().join("lib2bit-empty-placeholder");
    let _ = std::fs::write(&path, []);
    if let Ok(file) = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
    {
        return file;
    }

    if let Ok(file) = File::open(".") {
        return file;
    }

    std::process::abort();
}

fn read_u8_at(data: &[u8], cursor: &mut usize) -> Option<u8> {
    let value = data.get(*cursor).copied()?;
    *cursor += 1;
    Some(value)
}

fn read_bytes_at<'a>(data: &'a [u8], cursor: &mut usize, len: usize) -> Option<&'a [u8]> {
    let end = cursor.checked_add(len)?;
    let out = data.get(*cursor..end)?;
    *cursor = end;
    Some(out)
}

fn read_u32_at(data: &[u8], cursor: &mut usize) -> Option<u32> {
    let bytes = read_bytes_at(data, cursor, 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn read_u32_vec_at(data: &[u8], cursor: &mut usize, count: usize) -> Option<Vec<u32>> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(read_u32_at(data, cursor)?);
    }
    Some(out)
}

fn get_byte_mask_from_offset(offset: i32) -> u8 {
    match offset {
        0 => 15,
        1 => 7,
        2 => 3,
        _ => 1,
    }
}

fn apply_n_mask(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32) {
    let Some(starts) = idx.n_block_start.get(tid) else {
        return;
    };
    let Some(sizes) = idx.n_block_sizes.get(tid) else {
        return;
    };

    for (&block_start, &block_size) in starts.iter().zip(sizes.iter()) {
        let block_end = block_start.saturating_add(block_size);
        if block_end <= start {
            continue;
        }
        if block_start >= end {
            break;
        }

        let overlap_start = block_start.max(start);
        let overlap_end = block_end.min(end);
        let from = (overlap_start - start) as usize;
        let to = (overlap_end - start) as usize;
        for ch in &mut seq[from..to] {
            *ch = 'N';
        }
    }
}

fn apply_soft_mask(seq: &mut [char], idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32) {
    let Some(starts) = idx.mask_block_start.get(tid) else {
        return;
    };
    let Some(sizes) = idx.mask_block_sizes.get(tid) else {
        return;
    };

    for (&block_start, &block_size) in starts.iter().zip(sizes.iter()) {
        let block_end = block_start.saturating_add(block_size);
        if block_end <= start {
            continue;
        }
        if block_start >= end {
            break;
        }

        let overlap_start = block_start.max(start);
        let overlap_end = block_end.min(end);
        let from = (overlap_start - start) as usize;
        let to = (overlap_end - start) as usize;
        for ch in &mut seq[from..to] {
            if *ch != 'N' {
                *ch = ch.to_ascii_lowercase();
            }
        }
    }
}
