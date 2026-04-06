use std::fmt;

#[derive(Debug)]
pub enum InversionListError {
    ValueOutOfRange(u32, u32),
    Generic(String),
}

#[derive(Clone, Debug)]
pub struct InversionList {
    capacity: u32,
    support: u32,
    pub intervals: Vec<(u32, u32)>,
}

impl InversionList {
    pub fn new(capacity: u32, values: &[u32]) -> Result<Self, InversionListError> {
        if values.is_empty() {
            return Ok(Self { capacity, support: 0, intervals: Vec::new() });
        }
        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort_unstable();
        if *sorted.last().unwrap() >= capacity {
            return Err(InversionListError::ValueOutOfRange(*sorted.last().unwrap(), capacity));
        }
        // deduplicate
        sorted.dedup();
        let support = sorted.len() as u32;
        // build intervals
        let mut intervals = Vec::new();
        let mut start = sorted[0];
        let mut end = sorted[0] + 1;
        for &v in &sorted[1..] {
            if v == end {
                end = v + 1;
            } else {
                intervals.push((start, end));
                start = v;
                end = v + 1;
            }
        }
        intervals.push((start, end));
        Ok(Self { capacity, support, intervals })
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn support(&self) -> u32 {
        self.support
    }

    pub fn contains(&self, value: u32) -> bool {
        // binary search: find the last interval whose start <= value
        match self.intervals.binary_search_by(|&(lo, _)| lo.cmp(&value)) {
            Ok(i) => value < self.intervals[i].1,
            Err(0) => false,
            Err(i) => value < self.intervals[i - 1].1,
        }
    }

    pub fn clone_list(&self) -> Self {
        self.clone()
    }

    pub fn complement(&self) -> Self {
        let mut intervals = Vec::new();
        let mut pos = 0u32;
        for &(lo, hi) in &self.intervals {
            if pos < lo {
                intervals.push((pos, lo));
            }
            pos = hi;
        }
        if pos < self.capacity {
            intervals.push((pos, self.capacity));
        }
        let support = self.capacity - self.support;
        Self { capacity: self.capacity, support, intervals }
    }

    pub fn to_str(&self) -> String {
        format!("{}", self)
    }

    pub fn equal(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }

    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        if self.support >= other.support {
            return false;
        }
        // Match C behavior: check if other has at least one member in [0, self's max endpoint)
        let max = match self.intervals.last() {
            Some(&(_, hi)) => hi,
            None => return false,
        };
        (0..max).any(|i| other.contains(i))
    }

    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        !self.equal(other)
    }

    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut vals = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi { vals.push(v); }
        }
        for &(lo, hi) in &other.intervals {
            for v in lo..hi { vals.push(v); }
        }
        InversionList::new(cap, &vals).unwrap()
    }

    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut intervals = Vec::new();
        let (mut i, mut j) = (0, 0);
        while i < self.intervals.len() && j < other.intervals.len() {
            let (a_lo, a_hi) = self.intervals[i];
            let (b_lo, b_hi) = other.intervals[j];
            let lo = a_lo.max(b_lo);
            let hi = a_hi.min(b_hi);
            if lo < hi {
                intervals.push((lo, hi));
            }
            if a_hi < b_hi { i += 1; } else { j += 1; }
        }
        let support = intervals.iter().map(|&(lo, hi)| hi - lo).sum();
        Self { capacity: cap, support, intervals }
    }

    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut intervals = Vec::new();
        let mut j = 0;
        for &(lo, hi) in &self.intervals {
            while j < other.intervals.len() && other.intervals[j].1 <= lo {
                j += 1;
            }
            let mut k = j;
            let mut cur = lo;
            while k < other.intervals.len() && other.intervals[k].0 < hi {
                let (b_lo, b_hi) = other.intervals[k];
                if cur < b_lo {
                    intervals.push((cur, b_lo.min(hi)));
                }
                cur = b_hi;
                k += 1;
            }
            if cur < hi {
                intervals.push((cur, hi));
            }
        }
        let support = intervals.iter().map(|&(lo, hi)| hi - lo).sum();
        Self { capacity: cap, support, intervals }
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let u = self.union(other);
        let i = self.intersection(other);
        u.difference(&i)
    }

}

impl PartialEq for InversionList {
    fn eq(&self, other: &Self) -> bool {
        self.equal(other)
    }
}

impl Eq for InversionList {}

impl fmt::Display for InversionList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut first = true;
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if first { first = false; } else { write!(f, ", ")?; }
                write!(f, "{}", v)?;
            }
        }
        write!(f, "]")
    }
}

pub struct InversionListIterator<'a> {
    list: &'a InversionList,
    interval_index: usize,
    current_value: u32,
}

impl<'a> InversionListIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        let current_value = if list.intervals.is_empty() { 0 } else { list.intervals[0].0 };
        Self { list, interval_index: 0, current_value }
    }
}

impl<'a> Iterator for InversionListIterator<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.interval_index >= self.list.intervals.len() {
            return None;
        }
        let (_, hi) = self.list.intervals[self.interval_index];
        if self.current_value < hi {
            let val = self.current_value;
            self.current_value += 1;
            if self.current_value >= hi {
                self.interval_index += 1;
                if self.interval_index < self.list.intervals.len() {
                    self.current_value = self.list.intervals[self.interval_index].0;
                }
            }
            Some(val)
        } else {
            self.interval_index += 1;
            if self.interval_index < self.list.intervals.len() {
                self.current_value = self.list.intervals[self.interval_index].0;
                self.next()
            } else {
                None
            }
        }
    }
}

pub struct InversionListCoupleIterator<'a> {
    list: &'a InversionList,
    couple_index: usize,
}

impl<'a> InversionListCoupleIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        Self { list, couple_index: 0 }
    }
}

impl<'a> Iterator for InversionListCoupleIterator<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.couple_index >= self.list.intervals.len() {
            return None;
        }
        let item = self.list.intervals[self.couple_index];
        self.couple_index += 1;
        Some(item)
    }
}
