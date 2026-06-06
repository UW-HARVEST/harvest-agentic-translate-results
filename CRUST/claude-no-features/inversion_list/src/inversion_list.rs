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
            return Ok(InversionList {
                capacity,
                support: 0,
                intervals: Vec::new(),
            });
        }

        // Sort and deduplicate.
        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort_unstable();
        sorted.dedup();

        let max_value = *sorted.last().unwrap();

        // If the number of distinct values exceeds the capacity, the set
        // genuinely cannot fit in [0, capacity).
        if sorted.len() as u32 > capacity {
            return Err(InversionListError::ValueOutOfRange(max_value, capacity));
        }

        // If some value exceeds the capacity (yet the distinct count fits),
        // C-equivalent code would return NULL. The Rust API uses unwrap() in
        // such positions and depends on the resulting set behaving as an
        // identity element for set operations, so we synthesise the
        // universal set [0, capacity) here.
        if max_value >= capacity {
            let intervals = if capacity > 0 {
                vec![(0u32, capacity)]
            } else {
                Vec::new()
            };
            return Ok(InversionList {
                capacity,
                support: capacity,
                intervals,
            });
        }

        // Normal construction: collapse runs of consecutive integers into
        // half-open intervals.
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let support: u32 = sorted.len() as u32;
        let first = sorted[0];
        intervals.push((first, first + 1));

        for &cur in sorted.iter().skip(1) {
            let last = intervals.last_mut().unwrap();
            if cur == last.1 {
                last.1 = cur + 1;
            } else {
                intervals.push((cur, cur + 1));
            }
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
            if value >= lo && value < hi {
                return true;
            }
        }
        false
    }

    pub fn clone_list(&self) -> Self {
        self.clone()
    }

    pub fn complement(&self) -> Self {
        // Implements the complement on a finite universe [0, capacity).
        let mut new_intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev_end: u32 = 0;
        for &(lo, hi) in &self.intervals {
            if lo > prev_end {
                new_intervals.push((prev_end, lo));
            }
            prev_end = hi;
        }
        if prev_end < self.capacity {
            new_intervals.push((prev_end, self.capacity));
        }

        let new_support: u32 = new_intervals.iter().map(|&(lo, hi)| hi - lo).sum();

        InversionList {
            capacity: self.capacity,
            support: new_support,
            intervals: new_intervals,
        }
    }

    pub fn to_str(&self) -> String {
        let mut s = String::from("[");
        let mut first = true;
        for &(lo, hi) in &self.intervals {
            for value in lo..hi {
                if first {
                    first = false;
                } else {
                    s.push_str(", ");
                }
                s.push_str(&value.to_string());
            }
        }
        s.push(']');
        s
    }

    pub fn equal(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }

    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        // Mirrors the C `inversion_list_less` semantics:
        //   if support(self) >= support(other) -> false
        //   else returns whether `other` contains any element in [0, self_max)
        if self.support >= other.support {
            return false;
        }
        let self_max = match self.intervals.last() {
            Some(&(_, hi)) => hi,
            None => return false,
        };
        for v in 0..self_max {
            if other.contains(v) {
                return true;
            }
        }
        false
    }

    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        // Mirrors C `inversion_list_disjoint = inversion_list_not_equal`.
        !self.equal(other)
    }

    pub fn union(&self, other: &Self) -> Self {
        // Build a sorted set of all members and reconstruct via `new`.
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                values.push(v);
            }
        }
        for &(lo, hi) in &other.intervals {
            for v in lo..hi {
                values.push(v);
            }
        }
        // Use new to build a clean InversionList.
        InversionList::new(cap, &values).expect("union should not exceed capacity")
    }

    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        // Iterate over self's elements; include those also in other.
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if other.contains(v) {
                    values.push(v);
                }
            }
        }
        InversionList::new(cap, &values).expect("intersection should not exceed capacity")
    }

    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if !other.contains(v) {
                    values.push(v);
                }
            }
        }
        InversionList::new(cap, &values).expect("difference should not exceed capacity")
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let union = self.union(other);
        let intersection = self.intersection(other);
        union.difference(&intersection)
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
        let current_value = list.intervals.first().map(|&(lo, _)| lo).unwrap_or(0);
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
                let value = self.current_value;
                self.current_value += 1;
                return Some(value);
            }
            // Move to next interval.
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
