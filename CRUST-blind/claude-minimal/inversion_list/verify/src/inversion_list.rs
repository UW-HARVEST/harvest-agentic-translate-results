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
        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort();
        sorted.dedup();

        if let Some(&max) = sorted.last() {
            if max >= capacity {
                return Err(InversionListError::ValueOutOfRange(max, capacity));
            }
        }

        let support = sorted.len() as u32;
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        if !sorted.is_empty() {
            let mut start = sorted[0];
            let mut end = sorted[0] + 1;
            for &v in &sorted[1..] {
                if v == end {
                    end += 1;
                } else {
                    intervals.push((start, end));
                    start = v;
                    end = v + 1;
                }
            }
            intervals.push((start, end));
        }

        Ok(Self {
            capacity,
            support,
            intervals,
        })
    }
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    pub fn support(&self) -> u32 {
        self.support
    }
    pub fn contains(&self, value: u32) -> bool {
        self.intervals
            .iter()
            .any(|&(lo, hi)| value >= lo && value < hi)
    }
    pub fn clone_list(&self) -> Self {
        self.clone()
    }
    pub fn complement(&self) -> Self {
        let mut result_intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev_end: u32 = 0;
        for &(lo, hi) in &self.intervals {
            if lo > prev_end {
                result_intervals.push((prev_end, lo));
            }
            prev_end = hi;
        }
        if prev_end < self.capacity {
            result_intervals.push((prev_end, self.capacity));
        }
        Self {
            capacity: self.capacity,
            support: self.capacity - self.support,
            intervals: result_intervals,
        }
    }
    pub fn to_str(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                parts.push(v.to_string());
            }
        }
        format!("[{}]", parts.join(", "))
    }
    pub fn equal(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        if self.support >= other.support {
            return false;
        }
        let max = match self.intervals.last() {
            Some(&(_, hi)) => hi,
            None => return true,
        };
        for i in 0..max {
            if other.contains(i) {
                return true;
            }
        }
        false
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }
    pub fn is_disjoint(&self, other: &Self) -> bool {
        !self.equal(other)
    }
    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let max_iter = match (self.intervals.last(), other.intervals.last()) {
            (Some(&(_, h1)), Some(&(_, h2))) => h1.max(h2),
            (Some(&(_, h)), None) => h,
            (None, Some(&(_, h))) => h,
            (None, None) => 0,
        };
        let mut values: Vec<u32> = Vec::new();
        for i in 0..max_iter {
            if self.contains(i) || other.contains(i) {
                values.push(i);
            }
        }
        Self::new(cap, &values).expect("union should succeed")
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let max_iter = match (self.intervals.last(), other.intervals.last()) {
            (Some(&(_, h1)), Some(&(_, h2))) => h1.min(h2),
            _ => 0,
        };
        let mut values: Vec<u32> = Vec::new();
        for i in 0..max_iter {
            if self.contains(i) && other.contains(i) {
                values.push(i);
            }
        }
        Self::new(cap, &values).expect("intersection should succeed")
    }
    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        if let Some(&(_, max)) = self.intervals.last() {
            for i in 0..max {
                if self.contains(i) && !other.contains(i) {
                    values.push(i);
                }
            }
        }
        Self::new(cap, &values).expect("difference should succeed")
    }
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let u = self.union(other);
        let i = self.intersection(other);
        u.difference(&i)
    }
}
impl PartialEq for InversionList {
    fn eq(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }
}
impl Eq for InversionList {}
impl fmt::Display for InversionList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}
pub struct InversionListIterator<'a> {
    list: &'a InversionList,
    interval_index: usize,
    current_value: u32,
}
impl<'a> InversionListIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        let current_value = if list.intervals.is_empty() {
            0
        } else {
            list.intervals[0].0
        };
        Self {
            list,
            interval_index: 0,
            current_value,
        }
    }
}
impl<'a> Iterator for InversionListIterator<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        while self.interval_index < self.list.intervals.len() {
            let (lo, hi) = self.list.intervals[self.interval_index];
            if self.current_value < lo {
                self.current_value = lo;
            }
            if self.current_value < hi {
                let v = self.current_value;
                self.current_value += 1;
                return Some(v);
            }
            self.interval_index += 1;
        }
        None
    }
}
pub struct InversionListCoupleIterator<'a> {
    list: &'a InversionList,
    couple_index: usize,
}
impl<'a> InversionListCoupleIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        Self {
            list,
            couple_index: 0,
        }
    }
}
impl<'a> Iterator for InversionListCoupleIterator<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.couple_index < self.list.intervals.len() {
            let v = self.list.intervals[self.couple_index];
            self.couple_index += 1;
            Some(v)
        } else {
            None
        }
    }
}
