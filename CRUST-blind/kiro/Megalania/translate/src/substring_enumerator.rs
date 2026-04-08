const N_CHARS: usize = 256;

pub struct SubstringEnumerator<'a> {
    data: &'a [u8],
    min_length: usize,
    max_length: usize,
    bigram_offsets: Box<[[usize; N_CHARS]; N_CHARS]>,
    bigram_sizes: Box<[[usize; N_CHARS]; N_CHARS]>,
    bigram_positions: Vec<usize>,
}
impl<'a> SubstringEnumerator<'a> {
    pub fn memory_usage(data_size: usize) -> usize {
        std::mem::size_of::<usize>() * data_size + std::mem::size_of::<Self>()
    }
    pub fn new(data: &'a [u8], min_length: usize, max_length: usize) -> Self {
        assert_eq!(min_length, 2, "only min_length = 2 is supported");
        let data_size = data.len();
        let mut bigram_sizes = Box::new([[0usize; N_CHARS]; N_CHARS]);
        let mut bigram_offsets = Box::new([[0usize; N_CHARS]; N_CHARS]);

        for i in 1..data_size {
            bigram_sizes[data[i - 1] as usize][data[i] as usize] += 1;
        }

        for i in 1..(N_CHARS * N_CHARS) {
            let j = i - 1;
            bigram_offsets[i % N_CHARS][i / N_CHARS] =
                bigram_offsets[j % N_CHARS][j / N_CHARS] + bigram_sizes[j % N_CHARS][j / N_CHARS];
        }

        let mut bigram_positions = vec![0usize; data_size];
        let mut fills = Box::new([[0usize; N_CHARS]; N_CHARS]);
        for i in 1..data_size {
            let first = data[i - 1] as usize;
            let second = data[i] as usize;
            let offset = bigram_offsets[first][second] + fills[first][second];
            bigram_positions[offset] = i - 1;
            fills[first][second] += 1;
        }

        SubstringEnumerator { data, min_length, max_length, bigram_offsets, bigram_sizes, bigram_positions }
    }
    pub fn for_each<F>(&self, pos: usize, mut callback: F)
    where
        F: FnMut(usize, usize),
    {
        if pos == 0 { return; }
        if pos == self.data.len() - 1 { return; }

        let first = self.data[pos] as usize;
        let second = self.data[pos + 1] as usize;
        let offset = self.bigram_offsets[first][second];
        let size = self.bigram_sizes[first][second];

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
