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
        let mut buffer: Vec<u32> = values.to_vec();
        let mut support: u32 = 0;
        let mut intervals: Vec<(u32, u32)> = Vec::new();

        if !buffer.is_empty() {
            buffer.sort();
            let last = buffer[buffer.len() - 1];
            if last >= capacity {
                return Err(InversionListError::ValueOutOfRange(last, capacity));
            }

            // Build intervals matching the C algorithm:
            // start with [buffer[0], buffer[0]+1), then for each next value:
            // - if equal to previous, skip
            // - if equal to current upper bound, extend
            // - otherwise (greater than upper bound), start new interval
            support += 1;
            let mut cur_lo = buffer[0];
            let mut cur_hi = buffer[0] + 1;
            for i in 1..buffer.len() {
                if buffer[i] == buffer[i - 1] {
                    continue;
                }
                support += 1;
                if buffer[i] == cur_hi {
                    cur_hi += 1;
                } else {
                    intervals.push((cur_lo, cur_hi));
                    cur_lo = buffer[i];
                    cur_hi = buffer[i] + 1;
                }
            }
            intervals.push((cur_lo, cur_hi));
        }

        Ok(InversionList {
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
        for &(lo, hi) in &self.intervals {
            if value < lo {
                return false;
            }
            if value < hi {
                return true;
            }
        }
        false
    }
    pub fn clone_list(&self) -> Self {
        InversionList {
            capacity: self.capacity,
            support: self.support,
            intervals: self.intervals.clone(),
        }
    }
    pub fn complement(&self) -> Self {
        let mut new_intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev: u32 = 0;
        for &(lo, hi) in &self.intervals {
            if lo > prev {
                new_intervals.push((prev, lo));
            }
            prev = hi;
        }
        if prev < self.capacity {
            new_intervals.push((prev, self.capacity));
        }
        let support = self.capacity.saturating_sub(self.support);
        InversionList {
            capacity: self.capacity,
            support,
            intervals: new_intervals,
        }
    }
    pub fn to_str(&self) -> String {
        let mut s = String::from("[");
        let mut first = true;
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if first {
                    first = false;
                } else {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
            }
        }
        s.push(']');
        s
    }
    pub fn equal(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        // Matches the C inversion_list_less behaviour:
        // returns false if support(self) >= support(other);
        // otherwise scans i in [0, max(self)) and returns true as soon as i is in `other`.
        if self.support >= other.support {
            return false;
        }
        let max = match self.intervals.last() {
            Some(&(_, hi)) => hi,
            None => return false,
        };
        let mut i: u32 = 0;
        while i < max {
            if other.contains(i) {
                return true;
            }
            i += 1;
        }
        false
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }
    pub fn is_disjoint(&self, other: &Self) -> bool {
        // Matches the C inversion_list_disjoint behaviour, which is `not_equal`.
        !self.equal(other)
    }
    pub fn union(&self, other: &Self) -> Self {
        let capacity = self.capacity.max(other.capacity);
        let mut all: Vec<(u32, u32)> = Vec::with_capacity(self.intervals.len() + other.intervals.len());
        all.extend(self.intervals.iter().copied());
        all.extend(other.intervals.iter().copied());
        all.sort();

        let mut merged: Vec<(u32, u32)> = Vec::new();
        for (lo, hi) in all {
            if let Some(last) = merged.last_mut() {
                if lo <= last.1 {
                    if hi > last.1 {
                        last.1 = hi;
                    }
                    continue;
                }
            }
            merged.push((lo, hi));
        }
        let support: u32 = merged.iter().map(|&(lo, hi)| hi - lo).sum();
        InversionList {
            capacity,
            support,
            intervals: merged,
        }
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let capacity = self.capacity.max(other.capacity);
        let mut result: Vec<(u32, u32)> = Vec::new();
        let mut i = 0usize;
        let mut j = 0usize;
        while i < self.intervals.len() && j < other.intervals.len() {
            let (a_lo, a_hi) = self.intervals[i];
            let (b_lo, b_hi) = other.intervals[j];
            let lo = a_lo.max(b_lo);
            let hi = a_hi.min(b_hi);
            if lo < hi {
                result.push((lo, hi));
            }
            if a_hi < b_hi {
                i += 1;
            } else if a_hi > b_hi {
                j += 1;
            } else {
                i += 1;
                j += 1;
            }
        }
        let support: u32 = result.iter().map(|&(lo, hi)| hi - lo).sum();
        InversionList {
            capacity,
            support,
            intervals: result,
        }
    }
    pub fn difference(&self, other: &Self) -> Self {
        let capacity = self.capacity.max(other.capacity);
        let mut result: Vec<(u32, u32)> = Vec::new();
        for &(a_lo, a_hi) in &self.intervals {
            let mut current_lo = a_lo;
            for &(b_lo, b_hi) in &other.intervals {
                if b_hi <= current_lo {
                    continue;
                }
                if b_lo >= a_hi {
                    break;
                }
                if b_lo > current_lo {
                    let upper = b_lo.min(a_hi);
                    if upper > current_lo {
                        result.push((current_lo, upper));
                    }
                }
                if b_hi > current_lo {
                    current_lo = b_hi;
                }
                if current_lo >= a_hi {
                    break;
                }
            }
            if current_lo < a_hi {
                result.push((current_lo, a_hi));
            }
        }
        let support: u32 = result.iter().map(|&(lo, hi)| hi - lo).sum();
        InversionList {
            capacity,
            support,
            intervals: result,
        }
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
        f.write_str(&self.to_str())
    }
}
pub struct InversionListIterator<'a> {
    list: &'a InversionList,
    interval_index: usize,
    current_value: u32,
}
impl<'a> InversionListIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        let current_value = list
            .intervals
            .first()
            .map(|&(lo, _)| lo)
            .unwrap_or(0);
        InversionListIterator {
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
            if self.interval_index < self.list.intervals.len() {
                self.current_value = self.list.intervals[self.interval_index].0;
            }
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
        InversionListCoupleIterator {
            list,
            couple_index: 0,
        }
    }
}
impl<'a> Iterator for InversionListCoupleIterator<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.couple_index < self.list.intervals.len() {
            let pair = self.list.intervals[self.couple_index];
            self.couple_index += 1;
            Some(pair)
        } else {
            None
        }
    }
}
