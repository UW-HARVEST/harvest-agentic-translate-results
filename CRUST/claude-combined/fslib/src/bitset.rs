#[allow(non_camel_case_types)]
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
    pub words: Vec<word_t>,
}
impl BitSet {
    pub fn new(nbits: usize) -> Self {
        let n_words = nbits / WORD_SIZE + 1;
        BitSet {
            words: vec![0; n_words],
        }
    }
    pub fn remove(&mut self) {
        self.words.clear();
    }
    pub fn set(&mut self, b: usize) {
        self.words[bindex(b)] |= 1u32 << boffset(b);
    }
    pub fn clear(&mut self, b: usize) {
        self.words[bindex(b)] &= !(1u32 << boffset(b));
    }
    pub fn get(&self, b: usize) -> bool {
        if bindex(b) >= self.words.len() {
            return false;
        }
        (self.words[bindex(b)] & (1u32 << boffset(b))) != 0
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
        for i in 0..self.words.len() {
            self.words[i] |= other.words[i];
        }
    }
    pub fn intersect(&mut self, other: &BitSet) {
        assert_eq!(self.words.len(), other.words.len());
        for i in 0..self.words.len() {
            self.words[i] &= other.words[i];
        }
    }
    pub fn toggle_all(&self) -> Self {
        let mut new_words = Vec::with_capacity(self.words.len());
        for w in self.words.iter() {
            new_words.push(*w ^ !0u32);
        }
        BitSet { words: new_words }
    }
    pub fn toggle_in_place(&mut self) {
        for w in self.words.iter_mut() {
            *w ^= !0u32;
        }
    }
}
