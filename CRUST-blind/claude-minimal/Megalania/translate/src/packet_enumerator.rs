use crate::lzma_packet::LZMAPacket;
use crate::lzma_state::LZMAState;
use crate::substring_enumerator::SubstringEnumerator;

const MIN_SUBSTRING: usize = 2;
const MAX_SUBSTRING: usize = 273;

pub struct PacketEnumerator<'a> {
    /// A slice of input data.
    pub data: &'a [u8],
    /// An owned substring enumerator built on the same data.
    pub substring_enumerator: Box<SubstringEnumerator<'a>>,
}
impl<'a> PacketEnumerator<'a> {
    pub fn memory_usage(data_size: usize) -> usize {
        std::mem::size_of::<PacketEnumerator>() + SubstringEnumerator::memory_usage(data_size)
    }
    pub fn new(data: &'a [u8]) -> Self {
        let substring_enumerator = Box::new(SubstringEnumerator::new(
            data,
            MIN_SUBSTRING,
            MAX_SUBSTRING,
        ));
        Self {
            data,
            substring_enumerator,
        }
    }
    pub fn for_each<F>(&self, state: &LZMAState, callback: F)
    where
        F: Fn(&LZMAState, LZMAPacket),
    {
        assert!(self.data.as_ptr() == state.data.as_ptr());

        callback(state, LZMAPacket::literal_packet());
        if state.position > 0 {
            let rep0_position = state.position - state.dists[0] as usize - 1;
            if self.data[state.position] == self.data[rep0_position] {
                callback(state, LZMAPacket::short_rep_packet());
            }
        }

        self.substring_enumerator.for_each(state.position, |offset, length| {
            let dist = (state.position - offset - 1) as u32;
            callback(state, LZMAPacket::match_packet(dist, length as u32));
            for i in 0..4u32 {
                if dist == state.dists[i as usize] {
                    callback(state, LZMAPacket::long_rep_packet(i, length as u32));
                }
            }
        });
    }
}
