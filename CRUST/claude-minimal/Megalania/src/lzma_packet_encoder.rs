use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::{LZMAPacket, LZMAPacketType};
use crate::lzma_state::{
    lzma_state_promote_distance_at, lzma_state_push_distance, lzma_state_update_ctx_state,
    DistanceProbabilityModel, LZMAState, ALIGN_BITS, END_POS_MODEL_INDEX, HIGH_CODER_BITS,
    LOW_CODER_BITS, LOW_CODER_SYMBOLS, MID_CODER_BITS, MID_CODER_SYMBOLS, NUM_LEN_TO_POS_STATES,
    NUM_POS_BITS_MAX, POS_SLOT_BITS,
};
use crate::probability_model::{
    encode_bit, encode_bit_tree, encode_bit_tree_reverse, encode_direct_bits,
};

#[derive(Default)]
struct LZMAPacketHeader {
    is_match: bool,
    rep: bool,
    b3: bool,
    b4: bool,
    b5: bool,
}

fn lzma_encode_packet_header(
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    head: &LZMAPacketHeader,
) {
    let ctx_pos_bits: usize = 0; // todo: position context bits currently unsupported
    let ctx_state = lzma_state.ctx_state as usize;
    let ctx_pos_state = (ctx_state << NUM_POS_BITS_MAX) + ctx_pos_bits;

    let ctx_probs = &mut lzma_state.probs.ctx_state;
    encode_bit(head.is_match, &mut ctx_probs.is_match[ctx_pos_state], enc);
    if !head.is_match {
        return;
    }

    encode_bit(head.rep, &mut ctx_probs.is_rep[ctx_state], enc);
    if !head.rep {
        return;
    }

    encode_bit(head.b3, &mut ctx_probs.is_rep_g0[ctx_state], enc);
    if head.b3 {
        encode_bit(head.b4, &mut ctx_probs.is_rep_g1[ctx_state], enc);
        if head.b4 {
            encode_bit(head.b5, &mut ctx_probs.is_rep_g2[ctx_state], enc);
        }
    } else {
        encode_bit(head.b4, &mut ctx_probs.is_rep0_long[ctx_pos_state], enc);
    }
}

fn lzma_encode_length(
    probs: &mut crate::lzma_state::LengthProbabilityModel,
    enc: &mut dyn EncoderInterface,
    len: u32,
) {
    let ctx_pos_bits: usize = 0; // todo: position context bits currently unsupported
    assert!(len >= 2);
    let mut len = len - 2;

    if (len as usize) < LOW_CODER_SYMBOLS {
        encode_bit(false, &mut probs.choice_1, enc);
        encode_bit_tree(len, &mut probs.low_coder[ctx_pos_bits], LOW_CODER_BITS, enc);
    } else {
        len -= LOW_CODER_SYMBOLS as u32;
        encode_bit(true, &mut probs.choice_1, enc);
        if (len as usize) < MID_CODER_SYMBOLS {
            encode_bit(false, &mut probs.choice_2, enc);
            encode_bit_tree(len, &mut probs.mid_coder[ctx_pos_bits], MID_CODER_BITS, enc);
        } else {
            len -= MID_CODER_SYMBOLS as u32;
            encode_bit(true, &mut probs.choice_2, enc);
            encode_bit_tree(len, &mut probs.high_coder, HIGH_CODER_BITS, enc);
        }
    }
}

fn get_msb32(val: u32) -> u32 {
    32 - val.leading_zeros()
}

fn lzma_encode_distance(
    probs: &mut DistanceProbabilityModel,
    enc: &mut dyn EncoderInterface,
    dist: u32,
    len: u32,
) {
    let mut len_ctx = (len - 2) as usize;
    if len_ctx >= NUM_LEN_TO_POS_STATES {
        len_ctx = NUM_LEN_TO_POS_STATES - 1;
    }

    if dist < 4 {
        encode_bit_tree(
            dist,
            &mut probs.pos_slot_coder[len_ctx],
            POS_SLOT_BITS,
            enc,
        );
        return;
    }

    let mut num_low_bits = (get_msb32(dist) - 2) as usize;
    let mut low_bits = dist & ((1u32 << num_low_bits) - 1);
    let high_bits = dist >> num_low_bits;
    let pos_slot = (num_low_bits as u32) * 2 + high_bits;
    encode_bit_tree(
        pos_slot,
        &mut probs.pos_slot_coder[len_ctx],
        POS_SLOT_BITS,
        enc,
    );

    if (pos_slot as usize) < END_POS_MODEL_INDEX {
        let pos_coder_ctx = ((high_bits as usize) << num_low_bits) - (pos_slot as usize);
        encode_bit_tree_reverse(
            low_bits,
            &mut probs.pos_coder[pos_coder_ctx..],
            num_low_bits,
            enc,
        );
        return;
    }

    let num_direct_bits = num_low_bits - ALIGN_BITS;
    num_low_bits = ALIGN_BITS;
    let direct_bits = low_bits >> ALIGN_BITS;
    low_bits = low_bits & ((1u32 << num_low_bits) - 1);

    encode_direct_bits(direct_bits, num_direct_bits, enc);
    encode_bit_tree_reverse(low_bits, &mut probs.align_coder, num_low_bits, enc);
}

fn lzma_encode_literal(lzma_state: &mut LZMAState, enc: &mut dyn EncoderInterface) {
    let head = LZMAPacketHeader {
        is_match: false,
        ..Default::default()
    };
    lzma_encode_packet_header(lzma_state, enc, &head);

    let lit_ctx: usize = 0; // todo: literal context bits currently unsupported
    let mut symbol: usize = 1;

    let lit = lzma_state.data[lzma_state.position];
    let mut matched = lzma_state.ctx_state >= 7;
    let match_byte = if matched {
        lzma_state.data[lzma_state.position - lzma_state.dists[0] as usize - 1]
    } else {
        0
    };
    let lit_probs = &mut lzma_state.probs.lit[0x300 * lit_ctx..];
    for i in (0..8).rev() {
        let bit = ((lit >> i) & 1) != 0;
        let mut context = symbol;

        if matched {
            let match_bit = ((match_byte >> i) & 1) as usize;
            context += (1 + match_bit) << 8;
            matched = (match_bit != 0) == bit;
        }

        encode_bit(bit, &mut lit_probs[context], enc);
        symbol = (symbol << 1) | (bit as usize);
    }
}

fn lzma_encode_match(
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    dist: u32,
    len: u32,
) {
    let head = LZMAPacketHeader {
        is_match: true,
        rep: false,
        ..Default::default()
    };
    lzma_encode_packet_header(lzma_state, enc, &head);

    lzma_state_push_distance(lzma_state, dist);
    lzma_encode_length(&mut lzma_state.probs.len, enc, len);
    lzma_encode_distance(&mut lzma_state.probs.dist, enc, dist, len);
}

fn lzma_encode_short_rep(lzma_state: &mut LZMAState, enc: &mut dyn EncoderInterface) {
    let head = LZMAPacketHeader {
        is_match: true,
        rep: true,
        b3: false,
        b4: false,
        ..Default::default()
    };
    lzma_encode_packet_header(lzma_state, enc, &head);
}

fn lzma_encode_long_rep(
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    dist_index: u32,
    len: u32,
) {
    let head = LZMAPacketHeader {
        is_match: true,
        rep: true,
        b3: dist_index != 0,
        b4: dist_index != 1,
        b5: dist_index != 2,
    };
    lzma_encode_packet_header(lzma_state, enc, &head);

    lzma_state_promote_distance_at(lzma_state, dist_index);
    lzma_encode_length(&mut lzma_state.probs.rep_len, enc, len);
}

pub fn lzma_encode_packet(
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    packet: LZMAPacket,
) {
    let len = packet.len as u32;
    let dist = packet.dist;
    match packet.packet_type {
        LZMAPacketType::Literal => lzma_encode_literal(lzma_state, enc),
        LZMAPacketType::Match => lzma_encode_match(lzma_state, enc, dist, len),
        LZMAPacketType::ShortRep => lzma_encode_short_rep(lzma_state, enc),
        LZMAPacketType::LongRep => lzma_encode_long_rep(lzma_state, enc, dist, len),
        LZMAPacketType::Invalid => panic!("invalid packet type"),
    }
    lzma_state_update_ctx_state(lzma_state, packet.packet_type as u32);
    lzma_state.position += len as usize;
}
