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
        std::mem::size_of::<Self>() + SubstringEnumerator::memory_usage(data_size)
    }
    pub fn new(data: &'a [u8]) -> Self {
        let se = SubstringEnumerator::new(data, MIN_SUBSTRING, MAX_SUBSTRING);
        PacketEnumerator {
            data,
            substring_enumerator: Box::new(se),
        }
    }
    pub fn for_each<F>(&self, state: &LZMAState, callback: F)
    where
        F: Fn(&LZMAState, LZMAPacket),
    {
        // Always emit a literal packet first.
        callback(state, LZMAPacket::literal_packet());

        if state.position > 0 {
            let rep0_position = state.position - state.dists[0] as usize - 1;
            if self.data[state.position] == self.data[rep0_position] {
                callback(state, LZMAPacket::short_rep_packet());
            }
        }

        let pos = state.position;
        self.substring_enumerator.for_each(pos, |offset, length| {
            // C: dist = lzma_state->position - offset - 1
            let dist = (pos - offset - 1) as u32;
            callback(state, LZMAPacket::match_packet(dist, length as u32));
            for i in 0..4 {
                if dist == state.dists[i] {
                    callback(
                        state,
                        LZMAPacket::long_rep_packet(i as u32, length as u32),
                    );
                }
            }
        });
    }
}
