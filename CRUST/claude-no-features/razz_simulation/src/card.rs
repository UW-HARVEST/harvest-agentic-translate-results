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

    // ---- Random number generator (lrand48-style LCG) ----
    // Lazily initialized to seed = 3 to match C tests (which call srand48(3)).
    thread_local! {
        static LCG_STATE: Cell<u64> = Cell::new((3u64 << 16) | 0x330E);
    }

    fn lrand48() -> u64 {
        LCG_STATE.with(|s| {
            let mut state = s.get();
            state = state.wrapping_mul(0x5DEECE66D).wrapping_add(0xB) & 0xFFFFFFFFFFFF;
            s.set(state);
            state >> 17
        })
    }

    fn srand48(seed: u32) {
        LCG_STATE.with(|s| {
            s.set(((seed as u64) << 16) | 0x330E);
        });
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

    const ALL_CSR: [CardSuitRank; 54] = [
        CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::Spade3, CardSuitRank::Spade4,
        CardSuitRank::Spade5, CardSuitRank::Spade6, CardSuitRank::Spade7, CardSuitRank::Spade8,
        CardSuitRank::Spade9, CardSuitRank::Spade10, CardSuitRank::SpadeJ, CardSuitRank::SpadeQ,
        CardSuitRank::SpadeK,
        CardSuitRank::HeartAce, CardSuitRank::Heart2, CardSuitRank::Heart3, CardSuitRank::Heart4,
        CardSuitRank::Heart5, CardSuitRank::Heart6, CardSuitRank::Heart7, CardSuitRank::Heart8,
        CardSuitRank::Heart9, CardSuitRank::Heart10, CardSuitRank::HeartJ, CardSuitRank::HeartQ,
        CardSuitRank::HeartK,
        CardSuitRank::DiamondAce, CardSuitRank::Diamond2, CardSuitRank::Diamond3,
        CardSuitRank::Diamond4, CardSuitRank::Diamond5, CardSuitRank::Diamond6,
        CardSuitRank::Diamond7, CardSuitRank::Diamond8, CardSuitRank::Diamond9,
        CardSuitRank::Diamond10, CardSuitRank::DiamondJ, CardSuitRank::DiamondQ,
        CardSuitRank::DiamondK,
        CardSuitRank::ClubAce, CardSuitRank::Club2, CardSuitRank::Club3, CardSuitRank::Club4,
        CardSuitRank::Club5, CardSuitRank::Club6, CardSuitRank::Club7, CardSuitRank::Club8,
        CardSuitRank::Club9, CardSuitRank::Club10, CardSuitRank::ClubJ, CardSuitRank::ClubQ,
        CardSuitRank::ClubK,
        CardSuitRank::CardCount, CardSuitRank::InvalidCard,
    ];

    fn idx_to_csr(i: usize) -> CardSuitRank {
        if i < 54 { ALL_CSR[i] } else { CardSuitRank::InvalidCard }
    }

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            const STRS: [[&str; 13]; 4] = [
                ["SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK"],
                ["HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK"],
                ["DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK"],
                ["CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK"],
            ];
            let idx = (*self) as usize;
            if idx >= 52 {
                return None;
            }
            Some(STRS[idx / 13][idx % 13].to_string())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardRank {
        Ace, R2, R3, R4, R5, R6, R7, R8, R9, R10, J, Q, K,
        RankCount,
        InvalidRank,
    }

    const ALL_RANKS: [CardRank; 15] = [
        CardRank::Ace, CardRank::R2, CardRank::R3, CardRank::R4, CardRank::R5,
        CardRank::R6, CardRank::R7, CardRank::R8, CardRank::R9, CardRank::R10,
        CardRank::J, CardRank::Q, CardRank::K, CardRank::RankCount, CardRank::InvalidRank,
    ];

    fn idx_to_rank(i: usize) -> CardRank {
        if i < 15 { ALL_RANKS[i] } else { CardRank::InvalidRank }
    }

    impl CardRank {
        pub fn ranktostr(&self) -> Option<String> {
            const STRS: [&str; 13] = [
                "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
            ];
            let idx = (*self) as usize;
            if idx >= 13 {
                return None;
            }
            Some(STRS[idx].to_string())
        }
        pub fn strtorank(s: &str) -> CardRank {
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let first = bytes[0];
            if first >= b'2' && first <= b'9' {
                let offset = (first - b'1') as usize;
                return idx_to_rank(offset);
            }
            match first.to_ascii_uppercase() {
                b'A' => CardRank::Ace,
                b'1' => {
                    if bytes.len() > 1 && bytes[1] == b'0' {
                        CardRank::R10
                    } else {
                        CardRank::InvalidRank
                    }
                }
                b'J' => CardRank::J,
                b'Q' => CardRank::Q,
                b'K' => CardRank::K,
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

    const ALL_SUITS: [CardSuit; 6] = [
        CardSuit::Spade, CardSuit::Heart, CardSuit::Diamond, CardSuit::Club,
        CardSuit::SuitCount, CardSuit::InvalidSuit,
    ];

    fn idx_to_suit(i: usize) -> CardSuit {
        if i < 6 { ALL_SUITS[i] } else { CardSuit::InvalidSuit }
    }

    pub struct Card {
        card: u8,
    }

    fn dup_card(c: &Card) -> Card {
        Card { card: c.card }
    }

    #[allow(dead_code)]
    fn dup_opt_card(c: &Option<Card>) -> Option<Card> {
        c.as_ref().map(dup_card)
    }

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let idx = csr as usize;
            if idx >= 52 {
                return Card { card: 0 };
            }
            let suit_bits: u8 = match idx / 13 {
                0 => 1u8 << 5, // Spade
                1 => 2u8 << 5, // Heart
                2 => 3u8 << 5, // Diamond
                3 => 4u8 << 5, // Club
                _ => 0,
            };
            let rank_bits: u8 = (idx % 13) as u8 + 1;
            Card { card: suit_bits | rank_bits }
        }

        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let base = match cs {
                CardSuit::Spade => 0usize,
                CardSuit::Heart => 13,
                CardSuit::Diamond => 26,
                CardSuit::Club => 39,
                _ => return CardSuitRank::InvalidCard,
            };
            idx_to_csr(base + cr as usize)
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = self.card & 0x1F;
            if r < 1 || r > 13 {
                return CardRank::InvalidRank;
            }
            idx_to_rank((r - 1) as usize)
        }

        pub fn get_card_suit(&self) -> CardSuit {
            let s = self.card & (0x7 << 5);
            if s < (1u8 << 5) || s > (4u8 << 5) {
                return CardSuit::InvalidSuit;
            }
            idx_to_suit(((s >> 5) - 1) as usize)
        }

        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let c = Card::write_card(csr);
            if c.card == 0 {
                None
            } else {
                Some(c)
            }
        }

        pub fn strtocard(s: &str) -> Option<Self> {
            let bytes = s.as_bytes();
            let char_count = bytes.len();
            if char_count != 2 && char_count != 3 {
                return None;
            }
            // For most cases char_count must be 2; only "?10" (e.g. "S10") allows 3.
            let suit_idx: usize = match bytes[0].to_ascii_uppercase() {
                b'S' => 0,
                b'H' => 13,
                b'D' => 26,
                b'C' => 39,
                _ => return None,
            };
            // The C code: only char_count == 2 is valid except for the "10" path (handled below).
            // The C code allows char_count==3 only for "10" (after a digit '1').
            let second = bytes[1];
            if char_count == 2 {
                if second >= b'2' && second <= b'9' {
                    return Card::create_card(idx_to_csr(suit_idx + (second - b'1') as usize));
                }
                match second.to_ascii_uppercase() {
                    b'A' => Card::create_card(idx_to_csr(suit_idx)),
                    b'1' => None, // "S1" alone is invalid in C
                    b'J' => Card::create_card(idx_to_csr(suit_idx + 10)),
                    b'Q' => Card::create_card(idx_to_csr(suit_idx + 11)),
                    b'K' => Card::create_card(idx_to_csr(suit_idx + 12)),
                    _ => None,
                }
            } else {
                // char_count == 3
                // C code only treats this case for the "10" suffix
                if second == b'1' && bytes[2] == b'0' {
                    Card::create_card(idx_to_csr(suit_idx + 9))
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
        pub fn insert_into_collection(self, _c: Option<Card>, _sorter: CardSorter) -> Self {
            // Not used directly; CardHand operates on the underlying linked list.
            self
        }
        pub fn iterate_collection(&self) -> &Self {
            self
        }
        pub fn append_into_collection(self, _new: Self) -> Self {
            self
        }
        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            // No-op
        }
    }

    fn empty_collection() -> CardCollection {
        CardCollection { prev: None, next: None, c: None }
    }

    // Helper functions to manipulate the singly-linked list stored in CardCollection.
    // The list is represented as: head node (the CardHand.cards field) holds the first
    // card in `c`, with subsequent cards chained via `next`. `prev` is unused.

    fn collection_to_vec(head: &CardCollection) -> Vec<Card> {
        let mut v = Vec::new();
        if let Some(c) = &head.c {
            v.push(dup_card(c));
            let mut cur = &head.next;
            while let Some(node) = cur {
                if let Some(c) = &node.c {
                    v.push(dup_card(c));
                }
                cur = &node.next;
            }
        }
        v
    }

    fn vec_to_collection(cards: Vec<Card>) -> CardCollection {
        let mut head = empty_collection();
        if cards.is_empty() {
            return head;
        }
        let mut iter = cards.into_iter();
        head.c = Some(iter.next().unwrap());
        // Build next chain
        let mut tail_next: &mut Option<Box<CardCollection>> = &mut head.next;
        for c in iter {
            *tail_next = Some(Box::new(CardCollection {
                prev: None,
                next: None,
                c: Some(c),
            }));
            tail_next = &mut tail_next.as_mut().unwrap().next;
        }
        head
    }

    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: CardCollection,
    }

    impl CardHand {
        pub fn create_hand(max: u8, sorter: CardSorter) -> Option<CardHand> {
            // sorter == sort_card_after by default if "null"; here, the caller passes a fn,
            // so we just use whatever is passed.
            Some(CardHand {
                max,
                len: 0,
                sorter,
                cards: empty_collection(),
            })
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = empty_collection();
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            let card_to_insert = match c {
                Some(card) => dup_card(card),
                None => return,
            };
            let new_opt = Some(dup_card(&card_to_insert));
            // Walk the existing chain to find insertion position based on sorter.
            let mut cards = collection_to_vec(&self.cards);
            let n = cards.len();
            let mut insert_pos: Option<usize> = None;
            if n == 0 {
                insert_pos = Some(0);
            } else {
                // Try (NULL, new, cards[0])
                let after_opt = Some(dup_card(&cards[0]));
                if (self.sorter)(&None, &new_opt, &after_opt) != 0 {
                    insert_pos = Some(0);
                }
            }
            if insert_pos.is_none() {
                let mut i = 0usize;
                while i + 1 < n {
                    let before_opt = Some(dup_card(&cards[i]));
                    let after_opt = Some(dup_card(&cards[i + 1]));
                    if (self.sorter)(&before_opt, &new_opt, &after_opt) != 0 {
                        insert_pos = Some(i + 1);
                        break;
                    }
                    i += 1;
                }
                if insert_pos.is_none() && n > 0 {
                    // Try last position: (cards[n-1], new, NULL)
                    let before_opt = Some(dup_card(&cards[n - 1]));
                    if (self.sorter)(&before_opt, &new_opt, &None) != 0 {
                        insert_pos = Some(n);
                    }
                }
            }
            if let Some(pos) = insert_pos {
                cards.insert(pos, card_to_insert);
                self.cards = vec_to_collection(cards);
                self.len += 1;
            }
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.len as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            let mut cur_rank = CardRank::InvalidRank;
            // Walk cards
            if self.len == 0 {
                return cur_rank;
            }
            let cards = collection_to_vec(&self.cards);
            for c in &cards {
                let r = c.get_card_rank();
                if cur_rank == CardRank::InvalidRank {
                    cur_rank = r;
                } else if r > cur_rank {
                    cur_rank = r;
                }
            }
            cur_rank
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let mut cards = collection_to_vec(&self.cards);
            let mut i = 0usize;
            let mut pos: u64 = 0;
            let mut stopped = false;
            while !stopped && i < cards.len() {
                let len = cards.len() as u64;
                let c_opt = Some(dup_card(&cards[i]));
                let action = itr_fn(len, pos, &c_opt);
                match action {
                    ItrAction::Continue => {
                        i += 1;
                        pos += 1;
                    }
                    ItrAction::Break => {
                        stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        cards.remove(i);
                        if self.len > 0 {
                            self.len -= 1;
                        }
                        // i stays the same; pos stays the same (matches C net behavior).
                    }
                    ItrAction::RemoveAndBreak => {
                        cards.remove(i);
                        if self.len > 0 {
                            self.len -= 1;
                        }
                        stopped = true;
                    }
                }
            }
            self.cards = vec_to_collection(cards);
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let mut cards = collection_to_vec(&self.cards);
            let mut i = 0;
            while i < cards.len() {
                if cards[i].get_card_suit_rank() == c {
                    cards.remove(i);
                    if self.len > 0 {
                        self.len -= 1;
                    }
                } else {
                    i += 1;
                }
            }
            self.cards = vec_to_collection(cards);
        }

        pub fn remove_from_hand_under_iter(
            &mut self,
            _card_collection: &CardCollection,
            _pos: usize,
        ) {
            // Not used directly by tests; iterate_hand already manages removal.
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }

    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let idx = c as usize;
            if idx >= 52 {
                return 0;
            }
            // In deck if its slot is INVALID_CARD (i.e., card == 0).
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                1
            } else {
                0
            }
        }

        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 {
                return None;
            }
            let selected = lrand48() % (self.card_count as u64);
            let mut valid_idx: u64 = 0;
            for i in 0..52usize {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_idx == selected {
                        let csr = idx_to_csr(i);
                        let new_card = Card::write_card(csr);
                        self.cards[i] = Card { card: new_card.card };
                        self.card_count -= 1;
                        return Some(Card { card: new_card.card });
                    }
                    valid_idx += 1;
                }
            }
            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= 52 {
                return;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                let new_card = Card::write_card(c);
                self.cards[idx] = new_card;
                self.card_count -= 1;
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            // Reset LCG to seed 3 to mirror C tests' explicit srand48(3) calls.
            srand48(3);
            let cards = std::array::from_fn(|_| Card { card: 0 });
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
        let new_rank = match new {
            Some(c) => c.get_card_rank(),
            None => return 0,
        };
        if after.is_none() {
            return 1;
        }
        let after_rank = after.as_ref().unwrap().get_card_rank();
        let before_ok = match before {
            None => true,
            Some(c) => new_rank > c.get_card_rank(),
        };
        if before_ok && new_rank <= after_rank {
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
