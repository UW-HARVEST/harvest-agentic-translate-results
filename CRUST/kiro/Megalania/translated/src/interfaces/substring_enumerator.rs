const N_CHARS: usize = 256;

pub struct SubstringEnumerator<'a> {
    data: &'a [u8],
    min_length: usize,
    max_length: usize,
    bigram_offsets: Vec<usize>,  // N_CHARS * N_CHARS
    bigram_sizes: Vec<usize>,    // N_CHARS * N_CHARS
    bigram_positions: Vec<usize>,
}

impl<'a> SubstringEnumerator<'a> {
    pub fn memory_usage(data_size: usize) -> usize {
        std::mem::size_of::<usize>() * data_size + std::mem::size_of::<Self>()
    }

    pub fn new(data: &'a [u8], min_length: usize, max_length: usize) -> Self {
        assert_eq!(min_length, 2, "only min_length = 2 is supported");
        let data_size = data.len();
        let mut bigram_sizes = vec![0usize; N_CHARS * N_CHARS];
        let mut bigram_offsets = vec![0usize; N_CHARS * N_CHARS];

        for i in 1..data_size {
            let idx = data[i - 1] as usize * N_CHARS + data[i] as usize;
            bigram_sizes[idx] += 1;
        }

        // Compute offsets: matches C layout where index = i%N_CHARS * N_CHARS + i/N_CHARS
        // but the C code iterates i from 1..N_CHARS*N_CHARS with:
        //   bigram_offsets[i%N][i/N] = bigram_offsets[j%N][j/N] + bigram_sizes[j%N][j/N]
        // where j = i-1. This is a prefix sum over the linearized array indexed as [col][row]
        // i.e. column-major order: index(c,r) = c * N_CHARS + r
        // The C code uses [first][second] for sizes but [i%N][i/N] for offsets.
        // Let's decode: for i in 1..N*N: offsets[i%N][i/N] = offsets[j%N][j/N] + sizes[j%N][j/N]
        // This means the linear index for offsets at step i is: (i%N)*N + (i/N)
        // For i=0: linear = 0*N + 0 = 0
        // For i=1: linear = 1*N + 0 = N
        // This is a column-major traversal of the [first][second] 2D array.
        // So the prefix sum goes in column-major order over the sizes array.

        // Column-major order: iterate second (outer), first (inner)
        let mut running = 0usize;
        for second in 0..N_CHARS {
            for first in 0..N_CHARS {
                let idx = first * N_CHARS + second;
                bigram_offsets[idx] = running;
                running += bigram_sizes[idx];
            }
        }

        // No wait, let me re-read the C code more carefully:
        // bigram_sizes[data[i-1]][data[i]]++ -- indexed as [first][second]
        // In C, [first][second] means first*N_CHARS + second in memory.
        //
        // For offsets: for i in 1..N*N:
        //   j = i-1
        //   bigram_offsets[i%N][i/N] = bigram_offsets[j%N][j/N] + bigram_sizes[j%N][j/N]
        //
        // The linear index for [a][b] in C is a*N+b.
        // So offsets[(i%N)*N + (i/N)] = offsets[(j%N)*N + (j/N)] + sizes[(j%N)*N + (j/N)]
        //
        // For i=0: linear = (0%N)*N + (0/N) = 0
        // For i=1: linear = (1%N)*N + (1/N) = 1*N + 0 = N  (when N=256, this is 256)
        // For i=2: linear = (2%N)*N + (2/N) = 2*N + 0 = 2N
        // ...
        // For i=N-1: linear = (N-1)*N + 0
        // For i=N: linear = 0*N + 1 = 1
        // For i=N+1: linear = 1*N + 1 = N+1
        //
        // So the traversal order is column-major: (0,0), (1,0), (2,0), ..., (N-1,0), (0,1), (1,1), ...
        // Which means first varies fastest, second varies slowest.
        // This is exactly what I had above. Let me redo it properly:

        let mut bigram_offsets = vec![0usize; N_CHARS * N_CHARS];
        let mut running = 0usize;
        for second in 0..N_CHARS {
            for first in 0..N_CHARS {
                let idx = first * N_CHARS + second;
                bigram_offsets[idx] = running;
                running += bigram_sizes[idx];
            }
        }

        let mut bigram_positions = vec![0usize; data_size];
        let mut fills = vec![0usize; N_CHARS * N_CHARS];
        for i in 1..data_size {
            let first = data[i - 1] as usize;
            let second = data[i] as usize;
            let idx = first * N_CHARS + second;
            let offset = bigram_offsets[idx] + fills[idx];
            bigram_positions[offset] = i - 1;
            fills[idx] += 1;
        }

        SubstringEnumerator {
            data,
            min_length,
            max_length,
            bigram_offsets,
            bigram_sizes,
            bigram_positions,
        }
    }

    pub fn for_each<F>(&self, pos: usize, mut callback: F)
    where
        F: FnMut(usize, usize),
    {
        if pos == 0 || pos == self.data.len() - 1 { return; }

        let first = self.data[pos] as usize;
        let second = self.data[pos + 1] as usize;
        let idx = first * N_CHARS + second;
        let offset = self.bigram_offsets[idx];
        let size = self.bigram_sizes[idx];

        for i in 0..size {
            let position = self.bigram_positions[offset + i];
            if position >= pos { break; }
            callback(position, 2);
            for j in 2..self.max_length {
                if j + pos >= self.data.len() { break; }
                if self.data[pos + j] != self.data[position + j] { break; }
                callback(position, 1 + j);
            }
        }
    }
}
