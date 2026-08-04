type word_t = u32;
const WORD_SIZE: usize = std::mem::size_of::<word_t>() * 8;
#[inline]
fn bindex(b: usize) -> usize {
    b / WORD_SIZE
}
#[inline]
fn boffset(b: usize) -> usize {
    b % WORD_SIZE
}
pub struct BitSet {
    words: Vec<word_t>,
}
impl BitSet {
    pub fn new(nbits: usize) -> Self {
        let n_words = nbits / WORD_SIZE + 1;
        BitSet { words: vec![0; n_words] }
    }
    pub fn remove(&mut self) {
        self.words.clear();
    }
    pub fn set(&mut self, b: usize) {
        self.words[bindex(b)] |= 1 << boffset(b);
    }
    pub fn clear(&mut self, b: usize) {
        self.words[bindex(b)] &= !(1 << boffset(b));
    }
    pub fn get(&self, b: usize) -> bool {
        (self.words[bindex(b)] & (1 << boffset(b))) != 0
    }
    pub fn clear_all(&mut self) {
        for w in self.words.iter_mut() {
            *w = 0;
        }
    }
    pub fn set_all(&mut self) {
        for w in self.words.iter_mut() {
            *w = !0;
        }
    }
    pub fn union(&mut self, other: &BitSet) {
        assert_eq!(self.words.len(), other.words.len());
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            *a |= *b;
        }
    }
    pub fn intersect(&mut self, other: &BitSet) {
        assert_eq!(self.words.len(), other.words.len());
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            *a &= *b;
        }
    }
    pub fn toggle_all(&self) -> Self {
        BitSet {
            words: self.words.iter().map(|w| !w).collect(),
        }
    }
}
