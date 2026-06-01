const N_CHARS: usize = 256;

pub struct SubstringEnumerator<'a> {
    pub data: &'a [u8],
    pub min_length: usize,
    pub max_length: usize,
    pub bigram_offsets: Vec<usize>, // [N_CHARS][N_CHARS] flattened
    pub bigram_sizes: Vec<usize>,   // [N_CHARS][N_CHARS] flattened
    pub bigram_positions: Vec<usize>,
}

impl<'a> SubstringEnumerator<'a> {
    pub fn memory_usage(data_size: usize) -> usize {
        std::mem::size_of::<usize>() * data_size + std::mem::size_of::<SubstringEnumerator>()
    }
    pub fn new(data: &'a [u8], min_length: usize, max_length: usize) -> Self {
        // Only min_length=2 supported to match C
        assert!(min_length == 2, "only min_length = 2 is supported");

        let data_size = data.len();
        let mut bigram_offsets = vec![0usize; N_CHARS * N_CHARS];
        let mut bigram_sizes = vec![0usize; N_CHARS * N_CHARS];
        let mut bigram_positions = vec![0usize; data_size];

        // Count bigrams: bigram_sizes[data[i-1]][data[i]]++
        if data_size >= 2 {
            for i in 1..data_size {
                let f = data[i - 1] as usize;
                let s = data[i] as usize;
                // C indexes as bigram_sizes[first][second], which in row-major is
                // bigram_sizes[f * N_CHARS + s]. We stick to the same layout.
                bigram_sizes[f * N_CHARS + s] += 1;
            }
        }

        // Build offsets.
        // C iterates linearly: for (int i = 1; i < N_CHARS*N_CHARS; i++)
        //    bigram_offsets[i%N_CHARS][i/N_CHARS] = bigram_offsets[(i-1)%N_CHARS][(i-1)/N_CHARS] + bigram_sizes[(i-1)%N_CHARS][(i-1)/N_CHARS]
        // Note that this iterates in column-major order (over the first dimension).
        // We must replicate exactly.
        for i in 1..(N_CHARS * N_CHARS) {
            let j = i - 1;
            let cur_first = i % N_CHARS;
            let cur_second = i / N_CHARS;
            let prev_first = j % N_CHARS;
            let prev_second = j / N_CHARS;
            bigram_offsets[cur_first * N_CHARS + cur_second] =
                bigram_offsets[prev_first * N_CHARS + prev_second]
                    + bigram_sizes[prev_first * N_CHARS + prev_second];
        }

        // Fill bigram_positions
        let mut fills = vec![0usize; N_CHARS * N_CHARS];
        if data_size >= 2 {
            for i in 1..data_size {
                let first = data[i - 1] as usize;
                let second = data[i] as usize;
                let idx = first * N_CHARS + second;
                let offset = bigram_offsets[idx] + fills[idx];
                bigram_positions[offset] = i - 1;
                fills[idx] += 1;
            }
        }

        Self {
            data,
            min_length,
            max_length,
            bigram_offsets,
            bigram_sizes,
            bigram_positions,
        }
    }
    /// For a given position `pos` (the “current” start), call the callback for each previous substring
    /// (starting at some index < pos) that matches the substring beginning at pos.
    pub fn for_each<F>(&self, pos: usize, mut callback: F)
    where
        F: FnMut(usize, usize),
    {
        if pos == 0 {
            return;
        }
        let data_size = self.data.len();
        if data_size == 0 || pos >= data_size - 1 {
            return;
        }
        let first = self.data[pos] as usize;
        let second = self.data[pos + 1] as usize;
        let idx = first * N_CHARS + second;
        let offset = self.bigram_offsets[idx];
        let size = self.bigram_sizes[idx];
        for i in 0..size {
            let position = self.bigram_positions[offset + i];
            if position >= pos {
                break;
            }
            callback(position, 2);
            let mut j = 2;
            while j < self.max_length && j + pos < data_size {
                if self.data[pos + j] != self.data[position + j] {
                    break;
                }
                callback(position, 1 + j);
                j += 1;
            }
        }
    }
}
