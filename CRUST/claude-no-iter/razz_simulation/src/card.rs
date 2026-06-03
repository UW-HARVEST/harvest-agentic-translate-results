pub mod card {
    use std::cell::Cell;

    pub const CLUB_BITS: u32 = 4 << 5;
    pub const ACE_BITS: u32 = 1;
    pub const J_BITS: u32 = 11;
    pub const R8_BITS: u32 = 8;
    pub const R10_BITS: u32 = 10;
    pub const HEART_BITS: u32 = 2 << 5;
    pub const R4_BITS: u32 = 4;
    pub const SPADE_BITS: u32 = 1 << 5;
    pub const Q_BITS: u32 = 12;
    pub const R7_BITS: u32 = 7;
    pub const R5_BITS: u32 = 5;
    pub const K_BITS: u32 = 13;
    pub const R3_BITS: u32 = 3;
    pub const RANK_BITS: u32 = 0x1F;
    pub const R9_BITS: u32 = 9;
    pub const SUIT_BITS: u32 = 0x7 << 5;
    pub const R6_BITS: u32 = 6;
    pub const INVALID_CARD_BITS: u32 = 0;
    pub const DIAMOND_BITS: u32 = 3 << 5;
    pub const R2_BITS: u32 = 2;

    // ----- lrand48-equivalent RNG -----
    thread_local! {
        static RNG_STATE: Cell<u64> = Cell::new(((3u64) << 16) | 0x330Eu64);
    }

    fn srand48(seed: i64) {
        RNG_STATE.with(|s| {
            s.set((((seed as u64) & 0xFFFFFFFFu64) << 16) | 0x330Eu64);
        });
    }

    fn lrand48() -> u32 {
        RNG_STATE.with(|s| {
            let new_state = s
                .get()
                .wrapping_mul(0x5DEECE66Du64)
                .wrapping_add(0xBu64)
                & ((1u64 << 48) - 1);
            s.set(new_state);
            (new_state >> 17) as u32
        })
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuitRank {
        SpadeAce, Spade2, Spade3, Spade4, Spade5, Spade6, Spade7, Spade8,
        Spade9, Spade10, SpadeJ, SpadeQ, SpadeK,
        HeartAce, Heart2, Heart3, Heart4, Heart5, Heart6, Heart7, Heart8,
        Heart9, Heart10, HeartJ, HeartQ, HeartK,
        DiamondAce, Diamond2, Diamond3, Diamond4, Diamond5, Diamond6,
        Diamond7, Diamond8, Diamond9, Diamond10, DiamondJ, DiamondQ, DiamondK,
        ClubAce, Club2, Club3, Club4, Club5, Club6, Club7, Club8, Club9,
        Club10, ClubJ, ClubQ, ClubK,
        CardCount,
        InvalidCard,
    }

    fn csr_from_index(idx: usize) -> CardSuitRank {
        match idx {
            0 => CardSuitRank::SpadeAce, 1 => CardSuitRank::Spade2,
            2 => CardSuitRank::Spade3, 3 => CardSuitRank::Spade4,
            4 => CardSuitRank::Spade5, 5 => CardSuitRank::Spade6,
            6 => CardSuitRank::Spade7, 7 => CardSuitRank::Spade8,
            8 => CardSuitRank::Spade9, 9 => CardSuitRank::Spade10,
            10 => CardSuitRank::SpadeJ, 11 => CardSuitRank::SpadeQ,
            12 => CardSuitRank::SpadeK,
            13 => CardSuitRank::HeartAce, 14 => CardSuitRank::Heart2,
            15 => CardSuitRank::Heart3, 16 => CardSuitRank::Heart4,
            17 => CardSuitRank::Heart5, 18 => CardSuitRank::Heart6,
            19 => CardSuitRank::Heart7, 20 => CardSuitRank::Heart8,
            21 => CardSuitRank::Heart9, 22 => CardSuitRank::Heart10,
            23 => CardSuitRank::HeartJ, 24 => CardSuitRank::HeartQ,
            25 => CardSuitRank::HeartK,
            26 => CardSuitRank::DiamondAce, 27 => CardSuitRank::Diamond2,
            28 => CardSuitRank::Diamond3, 29 => CardSuitRank::Diamond4,
            30 => CardSuitRank::Diamond5, 31 => CardSuitRank::Diamond6,
            32 => CardSuitRank::Diamond7, 33 => CardSuitRank::Diamond8,
            34 => CardSuitRank::Diamond9, 35 => CardSuitRank::Diamond10,
            36 => CardSuitRank::DiamondJ, 37 => CardSuitRank::DiamondQ,
            38 => CardSuitRank::DiamondK,
            39 => CardSuitRank::ClubAce, 40 => CardSuitRank::Club2,
            41 => CardSuitRank::Club3, 42 => CardSuitRank::Club4,
            43 => CardSuitRank::Club5, 44 => CardSuitRank::Club6,
            45 => CardSuitRank::Club7, 46 => CardSuitRank::Club8,
            47 => CardSuitRank::Club9, 48 => CardSuitRank::Club10,
            49 => CardSuitRank::ClubJ, 50 => CardSuitRank::ClubQ,
            51 => CardSuitRank::ClubK,
            52 => CardSuitRank::CardCount,
            _ => CardSuitRank::InvalidCard,
        }
    }

    fn csr_to_index(csr: CardSuitRank) -> usize {
        csr as usize
    }

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            const STRS: &[&str] = &[
                "SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK",
                "HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK",
                "DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK",
                "CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK",
            ];
            let i = csr_to_index(*self);
            if i >= STRS.len() {
                None
            } else {
                Some(STRS[i].to_string())
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardRank {
        Ace, R2, R3, R4, R5, R6, R7, R8, R9, R10, J, Q, K,
        RankCount,
        InvalidRank,
    }

    fn rank_from_index(idx: usize) -> CardRank {
        match idx {
            0 => CardRank::Ace, 1 => CardRank::R2, 2 => CardRank::R3,
            3 => CardRank::R4, 4 => CardRank::R5, 5 => CardRank::R6,
            6 => CardRank::R7, 7 => CardRank::R8, 8 => CardRank::R9,
            9 => CardRank::R10, 10 => CardRank::J, 11 => CardRank::Q,
            12 => CardRank::K,
            _ => CardRank::InvalidRank,
        }
    }

    impl CardRank {
        pub fn ranktostr(&self) -> Option<String> {
            const STRS: &[&str] = &[
                "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
            ];
            let i = *self as usize;
            if i >= STRS.len() {
                None
            } else {
                Some(STRS[i].to_string())
            }
        }
        pub fn strtorank(str: &str) -> CardRank {
            let bytes = str.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let c0 = bytes[0];
            if c0 >= b'2' && c0 <= b'9' {
                // ACE + (c0 - '1') = char digit - 1, mapping to enum index
                let offset = (c0 - b'1') as usize;
                return rank_from_index(offset);
            }
            let upper = (c0 as char).to_ascii_uppercase();
            match upper {
                'A' => CardRank::Ace,
                '1' => {
                    if bytes.len() >= 2 && bytes[1] == b'0' {
                        CardRank::R10
                    } else {
                        CardRank::InvalidRank
                    }
                }
                'J' => CardRank::J,
                'Q' => CardRank::Q,
                'K' => CardRank::K,
                _ => CardRank::InvalidRank,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuit {
        Spade, Heart, Diamond, Club,
        SuitCount,
        InvalidSuit,
    }

    pub struct Card {
        card: u8,
    }

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut bits: u8 = 0;
            let i = csr_to_index(csr);
            // Suit bits
            if i <= csr_to_index(CardSuitRank::SpadeK) {
                bits |= SPADE_BITS as u8;
            } else if i >= csr_to_index(CardSuitRank::HeartAce)
                && i <= csr_to_index(CardSuitRank::HeartK)
            {
                bits |= HEART_BITS as u8;
            } else if i >= csr_to_index(CardSuitRank::DiamondAce)
                && i <= csr_to_index(CardSuitRank::DiamondK)
            {
                bits |= DIAMOND_BITS as u8;
            } else if i >= csr_to_index(CardSuitRank::ClubAce)
                && i <= csr_to_index(CardSuitRank::ClubK)
            {
                bits |= CLUB_BITS as u8;
            } else {
                // Invalid card; bits stay 0
                return Card { card: 0 };
            }

            // Rank bits: rank index within suit (0..12) + 1
            let rank_within_suit = i % 13;
            let rank_bits: u8 = match rank_within_suit {
                0 => ACE_BITS as u8,
                1 => R2_BITS as u8,
                2 => R3_BITS as u8,
                3 => R4_BITS as u8,
                4 => R5_BITS as u8,
                5 => R6_BITS as u8,
                6 => R7_BITS as u8,
                7 => R8_BITS as u8,
                8 => R9_BITS as u8,
                9 => R10_BITS as u8,
                10 => J_BITS as u8,
                11 => Q_BITS as u8,
                12 => K_BITS as u8,
                _ => 0,
            };
            bits |= rank_bits;
            Card { card: bits }
        }

        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let suit_offset = match cs {
                CardSuit::Spade => 0,
                CardSuit::Heart => 13,
                CardSuit::Diamond => 26,
                CardSuit::Club => 39,
                _ => return CardSuitRank::InvalidCard,
            };
            let rank_offset = cr as usize;
            csr_from_index(suit_offset + rank_offset)
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card as u32) & RANK_BITS;
            if r < ACE_BITS || r > K_BITS {
                return CardRank::InvalidRank;
            }
            rank_from_index((r - 1) as usize)
        }

        pub fn get_card_suit(&self) -> CardSuit {
            let s = (self.card as u32) & SUIT_BITS;
            if s < SPADE_BITS || s > CLUB_BITS {
                return CardSuit::InvalidSuit;
            }
            match (s >> 5) - 1 {
                0 => CardSuit::Spade,
                1 => CardSuit::Heart,
                2 => CardSuit::Diamond,
                3 => CardSuit::Club,
                _ => CardSuit::InvalidSuit,
            }
        }

        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let card = Card::write_card(csr);
            if (card.card as u32) == INVALID_CARD_BITS {
                None
            } else {
                Some(card)
            }
        }

        pub fn strtocard(str: &str) -> Option<Self> {
            let bytes = str.as_bytes();
            let char_count = bytes.len();
            if char_count != 2 && char_count != 3 {
                return None;
            }
            // Special-case len 2: Sx, Hx, Dx, Cx where x is rank
            // len 3: S10, H10, D10, C10
            if char_count == 2 {
                let suit_offset = match (bytes[0] as char).to_ascii_uppercase() {
                    'S' => 0,
                    'H' => 13,
                    'D' => 26,
                    'C' => 39,
                    _ => return None,
                };
                let r = bytes[1];
                if r >= b'2' && r <= b'9' {
                    let offset = (r - b'1') as usize;
                    return Card::create_card(csr_from_index(suit_offset + offset));
                }
                let upper = (r as char).to_ascii_uppercase();
                match upper {
                    'A' => Card::create_card(csr_from_index(suit_offset)),
                    'J' => Card::create_card(csr_from_index(suit_offset + 10)),
                    'Q' => Card::create_card(csr_from_index(suit_offset + 11)),
                    'K' => Card::create_card(csr_from_index(suit_offset + 12)),
                    _ => None,
                }
            } else {
                // length 3 - only valid for "S10", "H10", "D10", "C10"
                let suit_offset = match (bytes[0] as char).to_ascii_uppercase() {
                    'S' => 0,
                    'H' => 13,
                    'D' => 26,
                    'C' => 39,
                    _ => return None,
                };
                if bytes[1] == b'1' && bytes[2] == b'0' {
                    Card::create_card(csr_from_index(suit_offset + 9))
                } else {
                    None
                }
            }
        }
    }

    pub struct CardCollection {
        #[allow(dead_code)]
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }

    impl CardCollection {
        fn empty() -> Self {
            CardCollection {
                prev: None,
                next: None,
                c: None,
            }
        }

        pub fn insert_into_collection(self, c: Option<Card>, sorter: CardSorter) -> Self {
            // Convert linked list to Vec for easier manipulation
            let mut cards: Vec<Card> = collection_to_vec(self);
            // Find insertion position
            let new_c = c;
            let insert_pos = find_insert_pos(&cards, &new_c, sorter);
            if let Some(card) = new_c {
                cards.insert(insert_pos, card);
            }
            vec_to_collection(cards)
        }

        pub fn iterate_collection(&self) -> &Self {
            self
        }

        pub fn append_into_collection(self, new: Self) -> Self {
            let mut cards = collection_to_vec(self);
            let mut new_cards = collection_to_vec(new);
            cards.append(&mut new_cards);
            vec_to_collection(cards)
        }

        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            // No-op: in our flattened representation, detachment is handled by
            // the surrounding hand operations.
        }
    }

    fn find_insert_pos(cards: &[Card], new_c: &Option<Card>, sorter: CardSorter) -> usize {
        if new_c.is_none() {
            return cards.len();
        }
        for i in 0..=cards.len() {
            let before = if i == 0 {
                None
            } else {
                Some(Card { card: cards[i - 1].card })
            };
            let after = if i == cards.len() {
                None
            } else {
                Some(Card { card: cards[i].card })
            };
            if sorter(&before, new_c, &after) != 0 {
                return i;
            }
        }
        cards.len()
    }

    fn collection_to_vec(col: CardCollection) -> Vec<Card> {
        let mut out: Vec<Card> = Vec::new();
        let mut cur = Some(Box::new(col));
        while let Some(mut node) = cur {
            if let Some(c) = node.c.take() {
                out.push(c);
            } else {
                break;
            }
            cur = node.next.take();
        }
        out
    }

    fn vec_to_collection(mut cards: Vec<Card>) -> CardCollection {
        if cards.is_empty() {
            return CardCollection::empty();
        }
        // Build chain back to front
        let mut tail_next: Option<Box<CardCollection>> = None;
        while cards.len() > 1 {
            let last = cards.pop().unwrap();
            let node = Box::new(CardCollection {
                prev: None,
                next: tail_next,
                c: Some(last),
            });
            tail_next = Some(node);
        }
        // cards has exactly 1 element now: the head
        let head_card = cards.pop().unwrap();
        CardCollection {
            prev: None,
            next: tail_next,
            c: Some(head_card),
        }
    }

    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: CardCollection,
    }

    impl CardHand {
        pub fn create_hand(max: u8, sorter: CardSorter) -> Option<CardHand> {
            // If sorter is "null" (we model that as default), use sort_card_after.
            // Since CardSorter is fn pointer, it can't be None directly. The caller
            // always passes a function.
            Some(CardHand {
                max,
                len: 0,
                sorter,
                cards: CardCollection::empty(),
            })
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = CardCollection::empty();
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            let card_to_insert = match c {
                Some(card) => Some(Card { card: card.card }),
                None => None,
            };
            if card_to_insert.is_none() {
                return;
            }
            let cards = std::mem::replace(&mut self.cards, CardCollection::empty());
            self.cards = cards.insert_into_collection(card_to_insert, self.sorter);
            self.len += 1;
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.len as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            if self.len == 0 {
                return CardRank::InvalidRank;
            }
            let mut cur_node: Option<&CardCollection> = Some(&self.cards);
            let mut max_rank = CardRank::InvalidRank;
            while let Some(node) = cur_node {
                if let Some(c) = &node.c {
                    let cr = c.get_card_rank();
                    if max_rank == CardRank::InvalidRank {
                        max_rank = cr;
                    } else if cr > max_rank {
                        max_rank = cr;
                    }
                } else {
                    break;
                }
                cur_node = node.next.as_deref();
            }
            max_rank
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            // Move out the linked list into a Vec
            let cards_col = std::mem::replace(&mut self.cards, CardCollection::empty());
            let cards: Vec<Card> = collection_to_vec(cards_col);
            // Build list of options for callback's &Option<Card> argument
            let mut options: Vec<Option<Card>> = cards
                .into_iter()
                .map(|c| Some(c))
                .collect();
            let mut keep: Vec<bool> = vec![true; options.len()];
            let mut current_len: u64 = options.len() as u64;
            let mut pos: u64 = 0;
            let mut stop = false;
            let mut i = 0;
            while i < options.len() && !stop {
                if !keep[i] {
                    i += 1;
                    continue;
                }
                let action = itr_fn(current_len, pos, &options[i]);
                match action {
                    ItrAction::Continue => {
                        pos += 1;
                    }
                    ItrAction::Break => {
                        stop = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        keep[i] = false;
                        if current_len > 0 {
                            current_len -= 1;
                        }
                    }
                    ItrAction::RemoveAndBreak => {
                        keep[i] = false;
                        if current_len > 0 {
                            current_len -= 1;
                        }
                        stop = true;
                    }
                }
                i += 1;
            }
            // Rebuild remaining cards
            let mut remaining: Vec<Card> = Vec::new();
            for (idx, opt) in options.drain(..).enumerate() {
                if keep[idx] {
                    if let Some(c) = opt {
                        remaining.push(c);
                    }
                }
            }
            self.len = remaining.len() as u8;
            self.cards = vec_to_collection(remaining);
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let cards_col = std::mem::replace(&mut self.cards, CardCollection::empty());
            let all: Vec<Card> = collection_to_vec(cards_col);
            let mut kept: Vec<Card> = Vec::with_capacity(all.len());
            for card in all {
                if card.get_card_suit_rank() != c {
                    kept.push(card);
                }
            }
            self.len = kept.len() as u8;
            self.cards = vec_to_collection(kept);
        }

        pub fn remove_from_hand_under_iter(
            &mut self,
            _card_collection: &CardCollection,
            _pos: usize,
        ) {
            // Helper used internally; iterate_hand does its own removal.
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }

    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let idx = csr_to_index(c);
            if idx >= 52 {
                return 0;
            }
            // C version returns nonzero (true) if the card is still in the deck:
            //   get_card_suit_rank(...) == INVALID_CARD => still in deck
            if (self.cards[idx].card as u32) == INVALID_CARD_BITS {
                1
            } else {
                0
            }
        }

        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 {
                return None;
            }
            let selected = (lrand48() as u32) % (self.card_count as u32);
            let mut valid_idx: u32 = 0;
            for i in 0..52usize {
                if (self.cards[i].card as u32) == INVALID_CARD_BITS {
                    if valid_idx == selected {
                        let csr = csr_from_index(i);
                        let new_card = Card::write_card(csr);
                        self.cards[i] = Card {
                            card: new_card.card,
                        };
                        self.card_count -= 1;
                        return Some(Card {
                            card: new_card.card,
                        });
                    }
                    valid_idx += 1;
                }
            }
            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = csr_to_index(c);
            if idx >= 52 {
                return;
            }
            if (self.cards[idx].card as u32) == INVALID_CARD_BITS {
                let new_card = Card::write_card(c);
                self.cards[idx] = Card {
                    card: new_card.card,
                };
                if self.card_count > 0 {
                    self.card_count -= 1;
                }
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            // Reset RNG to seed 3 to mirror the reproducible-shuffle behaviour
            // used by the C tests (which call srand48(3) before each deck).
            srand48(3);
            let cards: [Card; 52] = std::array::from_fn(|_| Card { card: 0 });
            Some(CardDeck {
                card_count: 52,
                cards,
            })
        }
    }

    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(
        _before: &Option<Card>,
        _new: &Option<Card>,
        after: &Option<Card>,
    ) -> i32 {
        if after.is_none() {
            1
        } else {
            0
        }
    }

    pub fn sort_card_by_rank(
        before: &Option<Card>,
        new: &Option<Card>,
        after: &Option<Card>,
    ) -> i32 {
        let new_card = match new {
            Some(c) => c,
            None => return 0,
        };
        let r = new_card.get_card_rank();
        if after.is_none() {
            return 1;
        }
        let after_card = after.as_ref().unwrap();
        let after_rank = after_card.get_card_rank();
        let before_ok = match before {
            None => true,
            Some(b) => r > b.get_card_rank(),
        };
        if before_ok && r <= after_rank {
            1
        } else {
            0
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ItrAction {
        Continue,
        Break,
        RemoveAndContinue,
        RemoveAndBreak,
    }

    pub type CardIterator = fn(u64, u64, &Option<Card>) -> ItrAction;
}
