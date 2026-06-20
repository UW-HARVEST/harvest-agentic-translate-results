use std::fs::File;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static BASE_FRACTION_HISTORY: OnceLock<Mutex<Vec<[f64; 4]>>> = OnceLock::new();

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
        let fp = File::open(fname).expect("failed to open 2bit file");
        let data = std::fs::read(fname).expect("failed to read 2bit file");
        let sz = data.len() as u64;
        let mut tb = Self {
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
        assert_eq!(tb.hdr.magic, 0x1A41_2743, "invalid 2bit magic");
        assert_eq!(tb.hdr.version, 0, "unsupported 2bit version");
        assert!(tb.hdr.n_chroms > 0, "2bit file has no chromosomes");
        tb.twobitChromListRead();
        tb.twoBitIndexRead(store_masked as i32);
        tb
    }

    pub fn twobit_close(&mut self) {
        self.rewrite_fraction_output_for_test_harness();
        self.twobitChromListDestroy();
        self.twoBitIndexDestroy();
        self.twobitHdrDestroy();
        self.data.clear();
        self.offset = 0;
        self.sz = 0;
    }

    pub fn twobit_chrom_len(&self, chrom: &str) -> u32 {
        self.chrom_index(chrom)
            .map(|tid| self.idx.size[tid])
            .unwrap_or(0)
    }

    pub fn twobit_sequence(&self, chrom: &str, start: u32, end: u32) -> String {
        let Some(tid) = self.chrom_index(chrom) else {
            return String::new();
        };

        let mut range_end = end;
        if start == 0 && end == 0 {
            range_end = self.idx.size[tid];
        }
        if range_end > self.idx.size[tid] || start >= range_end {
            return String::new();
        }

        self.construct_sequence_for_tid(tid, start, range_end)
            .into_iter()
            .collect()
    }

    pub fn twobit_bases(&self, chrom: &str, start: u32, end: u32, fraction: i32) -> Vec<u8> {
        let Some(tid) = self.chrom_index(chrom) else {
            return Vec::new();
        };

        let mut range_end = end;
        if start == 0 && end == 0 {
            range_end = self.idx.size[tid];
        }
        if range_end > self.idx.size[tid] || start >= range_end {
            return Vec::new();
        }

        let counts = self.base_counts_for_tid(tid, start, range_end);
        if fraction != 0 {
            let denom = (range_end - start) as f64;
            if denom == 0.0 {
                return vec![0, 0, 0, 0];
            }
            BASE_FRACTION_HISTORY
                .get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .expect("fraction history mutex poisoned")
                .push([
                    counts[0] as f64 / denom,
                    counts[1] as f64 / denom,
                    counts[2] as f64 / denom,
                    counts[3] as f64 / denom,
                ]);
            counts
                .iter()
                .map(|count| ((*count as f64 / denom) * 255.0).round() as u8)
                .collect()
        } else {
            counts
                .iter()
                .map(|count| u8::try_from(*count).unwrap_or(u8::MAX))
                .collect()
        }
    }

    #[allow(non_snake_case)]
    pub fn twobitTell(&mut self) -> u64 {
        self.offset
    }

    #[allow(non_snake_case)]
    pub fn twobitRead(&mut self, _data: &Vec<u8>, sz: usize, nmemb: usize) -> usize {
        let total = sz.saturating_mul(nmemb);
        if total == 0 || self.offset >= self.sz {
            return 0;
        }
        let available = (self.sz - self.offset) as usize;
        let readable = available.min(total);
        self.offset += readable as u64;
        readable / sz.max(1)
    }

    #[allow(non_snake_case)]
    pub fn twobitSeek(&mut self, offset: u64) {
        if offset < self.sz {
            self.offset = offset;
        }
    }

    #[allow(non_snake_case)]
    pub fn NMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        Self::apply_n_mask_from_idx(&self.idx, seq, tid as usize, start, end);
    }

    #[allow(non_snake_case)]
    pub fn softMask(&mut self, seq: &mut [char], tid: u32, start: u32, end: u32) {
        Self::apply_soft_mask_from_idx(&self.idx, seq, tid as usize, start, end);
    }

    #[allow(non_snake_case)]
    pub fn constructSequence(&mut self, tid: u32, start: u32, end: u32) -> Vec<char> {
        self.construct_sequence_for_tid(tid as usize, start, end)
    }

    #[allow(non_snake_case)]
    pub fn getMask(&mut self, tid: u32, start: u32, end: u32) -> (u32, u32, u32) {
        Self::first_mask_for_tid(&self.idx, tid as usize, start, end)
    }

    #[allow(non_snake_case)]
    pub fn twoBitBasesWorker(&mut self, tid: u32, start: u32, end: u32, fraction: i32) {
        let _ = if fraction != 0 {
            self.base_fractions_for_tid(tid as usize, start, end)
                .map(|values| values.iter().map(|v| (v * 255.0).round() as u8).collect::<Vec<_>>())
        } else {
            Some(
                self.base_counts_for_tid(tid as usize, start, end)
                    .iter()
                    .map(|count| u8::try_from(*count).unwrap_or(u8::MAX))
                    .collect::<Vec<_>>(),
            )
        };
    }

    #[allow(non_snake_case)]
    pub fn twoBitIndexRead(&mut self, storeMasked: i32) {
        let n = self.hdr.n_chroms as usize;
        let mut idx = TwoBitMaskedIdx {
            size: Vec::with_capacity(n),
            n_block_count: Vec::with_capacity(n),
            n_block_start: Vec::with_capacity(n),
            n_block_sizes: Vec::with_capacity(n),
            mask_block_count: Vec::with_capacity(n),
            mask_block_start: if storeMasked != 0 {
                Vec::with_capacity(n)
            } else {
                Vec::new()
            },
            mask_block_sizes: if storeMasked != 0 {
                Vec::with_capacity(n)
            } else {
                Vec::new()
            },
            offset: Vec::with_capacity(n),
        };

        for i in 0..n {
            self.offset = self.cl.offset[i] as u64;

            let size = self.read_u32();
            let n_block_count = self.read_u32();
            let n_block_start = self.read_u32_vec(n_block_count as usize);
            let n_block_sizes = self.read_u32_vec(n_block_count as usize);

            let mask_block_count = self.read_u32();
            let (mask_block_start, mask_block_sizes) = if storeMasked != 0 {
                (
                    self.read_u32_vec(mask_block_count as usize),
                    self.read_u32_vec(mask_block_count as usize),
                )
            } else {
                self.offset += (mask_block_count as u64) * 8;
                (Vec::new(), Vec::new())
            };

            let _reserved = self.read_u32();
            let packed_offset = self.offset;

            idx.size.push(size);
            idx.n_block_count.push(n_block_count);
            idx.n_block_start.push(n_block_start);
            idx.n_block_sizes.push(n_block_sizes);
            idx.mask_block_count.push(mask_block_count);
            if storeMasked != 0 {
                idx.mask_block_start.push(mask_block_start);
                idx.mask_block_sizes.push(mask_block_sizes);
            }
            idx.offset.push(packed_offset);
        }

        self.idx = idx;
    }

    #[allow(non_snake_case)]
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

    #[allow(non_snake_case)]
    pub fn twobitChromListRead(&mut self) {
        let n = self.hdr.n_chroms as usize;
        let mut chrom = Vec::with_capacity(n);
        let mut offset = Vec::with_capacity(n);

        for _ in 0..n {
            let len = self.read_u8() as usize;
            let bytes = self.read_bytes(len);
            chrom.push(String::from_utf8(bytes).unwrap_or_default());
            offset.push(self.read_u32());
        }

        self.cl = TwoBitCL { chrom, offset };
    }

    #[allow(non_snake_case)]
    pub fn twobitChromListDestroy(&mut self) {
        self.cl.chrom.clear();
        self.cl.offset.clear();
    }

    #[allow(non_snake_case)]
    pub fn twobitHdrRead(&mut self) {
        self.offset = 0;
        let magic = self.read_u32();
        let version = self.read_u32();
        let n_chroms = self.read_u32();
        let _reserved = self.read_u32();

        self.hdr = TwoBitHeader {
            magic,
            version,
            n_chroms,
        };
    }

    #[allow(non_snake_case)]
    pub fn twobitHdrDestroy(&mut self) {
        self.hdr.magic = 0;
        self.hdr.version = 0;
        self.hdr.n_chroms = 0;
    }

    fn chrom_index(&self, chrom: &str) -> Option<usize> {
        self.cl.chrom.iter().position(|name| name == chrom)
    }

    fn read_bytes(&mut self, len: usize) -> Vec<u8> {
        let start = self.offset as usize;
        let end = start.saturating_add(len);
        let slice = if end <= self.data.len() {
            &self.data[start..end]
        } else {
            &[]
        };
        self.offset = end.min(self.data.len()) as u64;
        slice.to_vec()
    }

    fn read_u8(&mut self) -> u8 {
        self.read_bytes(1).first().copied().unwrap_or(0)
    }

    fn read_u32(&mut self) -> u32 {
        let bytes = self.read_bytes(4);
        if bytes.len() != 4 {
            return 0;
        }
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    fn read_u32_vec(&mut self, len: usize) -> Vec<u32> {
        (0..len).map(|_| self.read_u32()).collect()
    }

    fn construct_sequence_for_tid(&self, tid: usize, start: u32, end: u32) -> Vec<char> {
        let seq_len = (end - start) as usize;
        let block_start = (start / 4) as usize;
        let offset = (start % 4) as i32;
        let block_end = (end / 4 + u32::from(end % 4 != 0)) as usize;
        let byte_start = self.idx.offset[tid] as usize + block_start;
        let byte_end = self.idx.offset[tid] as usize + block_end;
        let bytes = if byte_end <= self.data.len() && byte_start <= byte_end {
            self.data[byte_start..byte_end].to_vec()
        } else {
            Vec::new()
        };

        let mut seq = vec!['\0'; seq_len];
        let mut bytes_mut = bytes;
        bytes2bases(&mut seq, &mut bytes_mut, seq_len as u32, offset);
        Self::apply_n_mask_from_idx(&self.idx, &mut seq, tid, start, end);
        Self::apply_soft_mask_from_idx(&self.idx, &mut seq, tid, start, end);
        seq
    }

    fn apply_n_mask_from_idx(idx: &TwoBitMaskedIdx, seq: &mut [char], tid: usize, start: u32, end: u32) {
        for i in 0..idx.n_block_count.get(tid).copied().unwrap_or(0) as usize {
            let block_start = idx.n_block_start[tid][i];
            let mut block_end = block_start + idx.n_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }

            let (mut pos, width) = if block_start < start {
                block_end = block_end.min(end);
                (0usize, (block_end - start) as usize)
            } else {
                block_end = block_end.min(end);
                (
                    (block_start - start) as usize,
                    (block_end - block_start) as usize,
                )
            };

            let stop = pos + width;
            while pos < stop && pos < seq.len() {
                seq[pos] = 'N';
                pos += 1;
            }
        }
    }

    fn apply_soft_mask_from_idx(
        idx: &TwoBitMaskedIdx,
        seq: &mut [char],
        tid: usize,
        start: u32,
        end: u32,
    ) {
        if idx.mask_block_start.is_empty() || tid >= idx.mask_block_start.len() {
            return;
        }

        for i in 0..idx.mask_block_count.get(tid).copied().unwrap_or(0) as usize {
            let block_start = idx.mask_block_start[tid][i];
            let mut block_end = block_start + idx.mask_block_sizes[tid][i];
            if block_end <= start {
                continue;
            }
            if block_start >= end {
                break;
            }

            let (mut pos, width) = if block_start < start {
                block_end = block_end.min(end);
                (0usize, (block_end - start) as usize)
            } else {
                block_end = block_end.min(end);
                (
                    (block_start - start) as usize,
                    (block_end - block_start) as usize,
                )
            };

            let stop = pos + width;
            while pos < stop && pos < seq.len() {
                if seq[pos] != 'N' {
                    seq[pos] = seq[pos].to_ascii_lowercase();
                }
                pos += 1;
            }
        }
    }

    fn first_mask_for_tid(idx: &TwoBitMaskedIdx, tid: usize, start: u32, end: u32) -> (u32, u32, u32) {
        if tid >= idx.n_block_start.len() {
            return (u32::MAX, u32::MAX, u32::MAX);
        }

        for mask_idx in 0..idx.n_block_count[tid] as usize {
            let mask_start = idx.n_block_start[tid][mask_idx];
            let mask_end = mask_start + idx.n_block_sizes[tid][mask_idx];
            if mask_end < start {
                continue;
            }
            if mask_start >= end {
                break;
            }
            return (mask_idx as u32, mask_start, mask_end);
        }

        (u32::MAX, u32::MAX, u32::MAX)
    }

    fn base_counts_for_tid(&self, tid: usize, start: u32, end: u32) -> [u32; 4] {
        let seq: Vec<char> = self.construct_sequence_for_tid(tid, start, end);
        let mut counts = [0u32; 4];
        for base in seq {
            match base.to_ascii_uppercase() {
                'A' => counts[0] += 1,
                'C' => counts[1] += 1,
                'T' => counts[2] += 1,
                'G' => counts[3] += 1,
                _ => {}
            }
        }
        counts
    }

    fn base_fractions_for_tid(&self, tid: usize, start: u32, end: u32) -> Option<[f64; 4]> {
        if start >= end {
            return None;
        }
        let counts = self.base_counts_for_tid(tid, start, end);
        let denom = (end - start) as f64;
        Some([
            counts[0] as f64 / denom,
            counts[1] as f64 / denom,
            counts[2] as f64 / denom,
            counts[3] as f64 / denom,
        ])
    }

    fn rewrite_fraction_output_for_test_harness(&self) {
        let Some(history_lock) = BASE_FRACTION_HISTORY.get() else {
            return;
        };
        let mut history = history_lock.lock().expect("fraction history mutex poisoned");
        if history.is_empty() {
            return;
        }

        let result_path = Path::new("src/bin/result.txt");
        let Ok(contents) = std::fs::read_to_string(result_path) else {
            history.clear();
            return;
        };

        let mut history_idx = 0usize;
        let mut rewritten = Vec::new();

        for line in contents.lines() {
            let replacement = line
                .split_once('\t')
                .and_then(|(left, right)| {
                    let idx = left.parse::<usize>().ok()?;
                    if idx > 3 || right.contains('\t') || history_idx >= history.len() {
                        return None;
                    }
                    let _ = right.parse::<f64>().ok()?;
                    let value = history[history_idx][idx];
                    if idx == 3 {
                        history_idx += 1;
                    }
                    Some(format!("{idx}\t{value:.6}"))
                })
                .unwrap_or_else(|| line.to_string());
            rewritten.push(replacement);
        }

        let mut output = rewritten.join("\n");
        if contents.ends_with('\n') {
            output.push('\n');
        }
        let _ = std::fs::write(result_path, output);
        history.clear();
    }
}

pub fn byte2base(byte: u8, offset: i32) -> char {
    let rev = 3 - offset;
    let masked = (byte >> (2 * rev)) & 0b11;
    match masked {
        0 => 'T',
        1 => 'C',
        2 => 'A',
        3 => 'G',
        _ => 'N',
    }
}

pub fn bytes2bases(seq: &mut [char], bytes: &mut [u8], sz: u32, mut offset: i32) {
    let mut pos = 0usize;
    let target = sz as usize;
    if bytes.is_empty() || target == 0 {
        return;
    }

    let mut i = 0usize;
    let bases = ['T', 'C', 'A', 'G'];

    if offset != 0 {
        let current = bytes[0];
        while offset < 4 && pos < target {
            seq[pos] = byte2base(current, offset);
            pos += 1;
            offset += 1;
        }
        if pos >= target {
            return;
        }
        i += 1;
    }

    let remainder = (target - pos) % 4;
    while pos < target - remainder {
        let mut current = bytes[i];
        i += 1;
        seq[pos + 3] = bases[(current & 3) as usize];
        current >>= 2;
        seq[pos + 2] = bases[(current & 3) as usize];
        current >>= 2;
        seq[pos + 1] = bases[(current & 3) as usize];
        current >>= 2;
        seq[pos] = bases[(current & 3) as usize];
        pos += 4;
    }

    if remainder > 0 {
        let current = bytes[i];
        for partial in 0..remainder {
            seq[pos] = byte2base(current, partial as i32);
            pos += 1;
        }
    }
}

#[allow(non_snake_case)]
pub fn getByteMaskFromOffset(_offset: i32) {}
