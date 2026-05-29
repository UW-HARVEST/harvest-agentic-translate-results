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

        let support = sorted.len() as u32;

        // If the number of distinct values can't fit in the declared
        // capacity, that's an error. (Matches the spirit of C's check.)
        if support > capacity {
            let max = sorted.last().copied().unwrap_or(0);
            return Err(InversionListError::ValueOutOfRange(max, capacity));
        }

        // If any value is out of range but support fits in capacity,
        // produce a "sentinel" empty set. The C code returns NULL in
        // this situation, which terminates varargs in the caller — we
        // mimic this by giving an empty set that acts as an identity
        // element for intersection / union.
        if sorted.last().map_or(false, |&v| v >= capacity) {
            return Ok(InversionList {
                capacity,
                support: 0,
                intervals: Vec::new(),
            });
        }

        let mut intervals: Vec<(u32, u32)> = Vec::new();
        if !sorted.is_empty() {
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
        // Binary search by start
        let mut lo: i64 = 0;
        let mut hi: i64 = self.intervals.len() as i64 - 1;
        while lo <= hi {
            let mid = ((lo + hi) / 2) as usize;
            let (s, e) = self.intervals[mid];
            if value < s {
                hi = mid as i64 - 1;
            } else if value >= e {
                lo = mid as i64 + 1;
            } else {
                return true;
            }
        }
        false
    }
    pub fn clone_list(&self) -> Self {
        self.clone()
    }
    pub fn complement(&self) -> Self {
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev_end: u32 = 0;
        for &(s, e) in &self.intervals {
            if s > prev_end {
                intervals.push((prev_end, s));
            }
            prev_end = e;
        }
        if prev_end < self.capacity {
            intervals.push((prev_end, self.capacity));
        }
        let support = self.capacity - self.support;
        InversionList {
            capacity: self.capacity,
            support,
            intervals,
        }
    }
    pub fn to_str(&self) -> String {
        let mut s = String::from("[");
        let mut first = true;
        for &(start, end) in &self.intervals {
            for v in start..end {
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
        if self.support != other.support {
            return false;
        }
        if self.intervals.len() != other.intervals.len() {
            return false;
        }
        self.intervals == other.intervals
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        // Replicate C `inversion_list_less` semantics:
        // returns true iff support(self) < support(other) AND `other` contains
        // at least one value in [0, last_upper_bound_of_self).
        if self.support >= other.support {
            return false;
        }
        if self.intervals.is_empty() {
            return false;
        }
        let max_val = self.intervals.last().unwrap().1;
        let mut i: u32 = 0;
        while i < max_val {
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
        // True disjoint: no common element.
        let mut i = 0usize;
        let mut j = 0usize;
        while i < self.intervals.len() && j < other.intervals.len() {
            let (a_s, a_e) = self.intervals[i];
            let (b_s, b_e) = other.intervals[j];
            let lo = a_s.max(b_s);
            let hi = a_e.min(b_e);
            if lo < hi {
                return false;
            }
            if a_e <= b_e {
                i += 1;
            } else {
                j += 1;
            }
        }
        true
    }
    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut all: Vec<(u32, u32)> = self
            .intervals
            .iter()
            .chain(other.intervals.iter())
            .cloned()
            .collect();
        all.sort();
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        for (s, e) in all {
            if let Some(last) = intervals.last_mut() {
                if s <= last.1 {
                    if e > last.1 {
                        last.1 = e;
                    }
                    continue;
                }
            }
            intervals.push((s, e));
        }
        let support: u32 = intervals.iter().map(|(s, e)| e - s).sum();
        InversionList {
            capacity: cap,
            support,
            intervals,
        }
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        // Treat an empty set as an identity element for intersection.
        // The C code's variadic interface terminates on the first NULL
        // argument (a failed `inversion_list_create` returns NULL), so a
        // set built from out-of-range values is effectively skipped.
        if self.intervals.is_empty() {
            return InversionList {
                capacity: cap,
                support: other.support,
                intervals: other.intervals.clone(),
            };
        }
        if other.intervals.is_empty() {
            return InversionList {
                capacity: cap,
                support: self.support,
                intervals: self.intervals.clone(),
            };
        }
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut i = 0usize;
        let mut j = 0usize;
        while i < self.intervals.len() && j < other.intervals.len() {
            let (a_s, a_e) = self.intervals[i];
            let (b_s, b_e) = other.intervals[j];
            let lo = a_s.max(b_s);
            let hi = a_e.min(b_e);
            if lo < hi {
                intervals.push((lo, hi));
            }
            if a_e <= b_e {
                i += 1;
            } else {
                j += 1;
            }
        }
        let support: u32 = intervals.iter().map(|(s, e)| e - s).sum();
        InversionList {
            capacity: cap,
            support,
            intervals,
        }
    }
    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        for &(a_s, a_e) in &self.intervals {
            let mut current_start = a_s;
            for &(b_s, b_e) in &other.intervals {
                if b_e <= current_start {
                    continue;
                }
                if b_s >= a_e {
                    break;
                }
                if b_s > current_start {
                    intervals.push((current_start, b_s));
                }
                if b_e > current_start {
                    current_start = b_e;
                }
                if current_start >= a_e {
                    break;
                }
            }
            if current_start < a_e {
                intervals.push((current_start, a_e));
            }
        }
        let support: u32 = intervals.iter().map(|(s, e)| e - s).sum();
        InversionList {
            capacity: cap,
            support,
            intervals,
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
        let current_value = list.intervals.first().map(|(s, _)| *s).unwrap_or(0);
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
            let (_, end) = self.list.intervals[self.interval_index];
            if self.current_value < end {
                let v = self.current_value;
                self.current_value += 1;
                return Some(v);
            }
            self.interval_index += 1;
            if self.interval_index < self.list.intervals.len() {
                let (s, _) = self.list.intervals[self.interval_index];
                self.current_value = s;
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
            let r = self.list.intervals[self.couple_index];
            self.couple_index += 1;
            Some(r)
        } else {
            None
        }
    }
}
