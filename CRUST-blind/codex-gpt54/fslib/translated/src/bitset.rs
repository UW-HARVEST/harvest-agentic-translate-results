type WordT = u32;
const WORD_SIZE: usize = std::mem::size_of::<WordT>() * 8;

#[inline]
fn bindex(b: usize) -> usize {
    b / WORD_SIZE
}

#[inline]
fn boffset(b: usize) -> usize {
    b % WORD_SIZE
}

pub struct BitSet {
    words: Vec<WordT>,
}

impl Clone for BitSet {
    fn clone(&self) -> Self {
        Self {
            words: self.words.clone(),
        }
    }
}

impl BitSet {
    pub fn new(nbits: usize) -> Self {
        Self {
            words: vec![0; nbits / WORD_SIZE + 1],
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
        (self.words[bindex(b)] & (1u32 << boffset(b))) != 0
    }

    pub fn clear_all(&mut self) {
        for word in &mut self.words {
            *word = 0;
        }
    }

    pub fn set_all(&mut self) {
        for word in &mut self.words {
            *word = !0;
        }
    }

    pub fn union(&mut self, other: &BitSet) {
        assert_eq!(self.words.len(), other.words.len());
        for (lhs, rhs) in self.words.iter_mut().zip(&other.words) {
            *lhs |= *rhs;
        }
    }

    pub fn intersect(&mut self, other: &BitSet) {
        assert_eq!(self.words.len(), other.words.len());
        for (lhs, rhs) in self.words.iter_mut().zip(&other.words) {
            *lhs &= *rhs;
        }
    }

    pub fn toggle_all(&self) -> Self {
        let mut out = self.clone();
        for word in &mut out.words {
            *word ^= !0;
        }
        out
    }
}
