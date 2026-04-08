pub mod card {
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
    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            static NAMES: [[&str; 13]; 4] = [
                ["SA","S2","S3","S4","S5","S6","S7","S8","S9","S10","SJ","SQ","SK"],
                ["HA","H2","H3","H4","H5","H6","H7","H8","H9","H10","HJ","HQ","HK"],
                ["DA","D2","D3","D4","D5","D6","D7","D8","D9","D10","DJ","DQ","DK"],
                ["CA","C2","C3","C4","C5","C6","C7","C8","C9","C10","CJ","CQ","CK"],
            ];
            let idx = *self as usize;
            if idx > CardSuitRank::ClubK as usize { return None; }
            Some(NAMES[idx / 13][idx % 13].to_string())
        }
    }
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardRank {
        Ace, R2, R3, R4, R5, R6, R7, R8, R9, R10, J, Q, K,
        RankCount,
        InvalidRank,
    }
    impl CardRank {
        pub fn ranktostr(&self) -> Option<String> {
            static NAMES: [&str; 13] = ["A","2","3","4","5","6","7","8","9","10","J","Q","K"];
            let idx = *self as usize;
            if idx > CardRank::K as usize { return None; }
            Some(NAMES[idx].to_string())
        }
        pub fn strtorank(s: &str) -> CardRank {
            let bytes = s.as_bytes();
            if bytes.is_empty() { return CardRank::InvalidRank; }
            let ch = bytes[0];
            if ch >= b'2' && ch <= b'9' {
                return Self::from_usize((ch - b'1') as usize);
            }
            match ch.to_ascii_uppercase() {
                b'A' => CardRank::Ace,
                b'1' => {
                    if bytes.len() > 1 && bytes[1] == b'0' { CardRank::R10 }
                    else { CardRank::InvalidRank }
                }
                b'J' => CardRank::J,
                b'Q' => CardRank::Q,
                b'K' => CardRank::K,
                _ => CardRank::InvalidRank,
            }
        }
        fn from_usize(v: usize) -> CardRank {
            match v {
                0 => CardRank::Ace, 1 => CardRank::R2, 2 => CardRank::R3,
                3 => CardRank::R4, 4 => CardRank::R5, 5 => CardRank::R6,
                6 => CardRank::R7, 7 => CardRank::R8, 8 => CardRank::R9,
                9 => CardRank::R10, 10 => CardRank::J, 11 => CardRank::Q,
                12 => CardRank::K, _ => CardRank::InvalidRank,
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
        card: u8
    }
    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut c = Card { card: INVALID_CARD_BITS as u8 };
            let idx = csr as u32;
            // Set suit bits
            if idx <= CardSuitRank::SpadeK as u32 {
                c.card |= SPADE_BITS as u8;
            } else if idx <= CardSuitRank::HeartK as u32 {
                c.card |= HEART_BITS as u8;
            } else if idx <= CardSuitRank::DiamondK as u32 {
                c.card |= DIAMOND_BITS as u8;
            } else if idx <= CardSuitRank::ClubK as u32 {
                c.card |= CLUB_BITS as u8;
            } else {
                return c;
            }
            // Set rank bits: rank within suit is idx % 13, bits = rank + 1
            let rank_in_suit = idx % 13;
            c.card |= (rank_in_suit + 1) as u8;
            c
        }
        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let base = match cs {
                CardSuit::Spade => CardSuitRank::SpadeAce as usize,
                CardSuit::Heart => CardSuitRank::HeartAce as usize,
                CardSuit::Diamond => CardSuitRank::DiamondAce as usize,
                CardSuit::Club => CardSuitRank::ClubAce as usize,
                _ => return CardSuitRank::InvalidCard,
            };
            csr_from_usize(base + cr as usize)
        }
        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card & RANK_BITS as u8) as u32;
            if r < ACE_BITS || r > K_BITS { return CardRank::InvalidRank; }
            CardRank::from_usize((r - 1) as usize)
        }
        pub fn get_card_suit(&self) -> CardSuit {
            let s = (self.card & SUIT_BITS as u8) as u32;
            if s < SPADE_BITS || s > CLUB_BITS { return CardSuit::InvalidSuit; }
            match s >> 5 {
                1 => CardSuit::Spade,
                2 => CardSuit::Heart,
                3 => CardSuit::Diamond,
                4 => CardSuit::Club,
                _ => CardSuit::InvalidSuit,
            }
        }
        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let c = Self::write_card(csr);
            if c.card == INVALID_CARD_BITS as u8 { None } else { Some(c) }
        }
        pub fn strtocard(s: &str) -> Option<Self> {
            if s.len() != 2 { return None; }
            let bytes = s.as_bytes();
            let base = match bytes[0].to_ascii_uppercase() {
                b'S' => CardSuitRank::SpadeAce as u32,
                b'H' => CardSuitRank::HeartAce as u32,
                b'D' => CardSuitRank::DiamondAce as u32,
                b'C' => CardSuitRank::ClubAce as u32,
                _ => return None,
            };
            let ch = bytes[1];
            if ch >= b'2' && ch <= b'9' {
                return Self::create_card(csr_from_usize((base + (ch - b'1') as u32) as usize));
            }
            match ch.to_ascii_uppercase() {
                b'A' => Self::create_card(csr_from_usize(base as usize)),
                b'1' => None, // len==2, can't be "10"
                b'J' => Self::create_card(csr_from_usize((base + 10) as usize)),
                b'Q' => Self::create_card(csr_from_usize((base + 11) as usize)),
                b'K' => Self::create_card(csr_from_usize((base + 12) as usize)),
                _ => None,
            }
        }
    }

    fn csr_from_usize(v: usize) -> CardSuitRank {
        match v {
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

    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }
    impl CardCollection {
        pub fn insert_into_collection(self, c: Option<Card>, sorter: CardSorter) -> Self {
            // Not used directly - CardHand uses Vec internally
            self
        }
        pub fn iterate_collection(&self) -> &Self {
            self
        }
        pub fn append_into_collection(self, new: Self) -> Self {
            self
        }
        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
        }
    }

    // Internal representation: CardHand uses a Vec<Card> for the card list
    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: CardCollection,
    }

    // We store actual cards in a separate Vec inside CardHand via a helper
    // Since we can't change the struct, we'll use the CardCollection's nested boxes
    // as a linked list. But that's impractical. Instead, we'll use a thread_local
    // approach... No, let's just store cards in a Vec hidden inside CardCollection.
    //
    // Actually, the simplest approach: repurpose CardHand to use a Vec<Option<Card>>
    // stored as a flat list. We can't change the struct fields, but we CAN use
    // the CardCollection field creatively. Let's store the actual card list in a
    // thread-local or use the `cards` field as a dummy and add a helper.
    //
    // Wait - we CAN'T change struct definitions. But we can use the `cards` field
    // and chain through `next` pointers to form a singly-linked list.

    impl CardHand {
        pub fn create_hand(max: u8, sorter: CardSorter) -> Option<CardHand> {
            Some(CardHand {
                max,
                len: 0,
                sorter,
                cards: CardCollection { prev: None, next: None, c: None },
            })
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = CardCollection { prev: None, next: None, c: None };
        }

        // Helper: collect all cards from the linked list into a Vec
        fn collect_cards(&self) -> Vec<Card> {
            let mut result = Vec::new();
            if self.len == 0 { return result; }
            // The first card is in self.cards.c, subsequent in next chain
            if let Some(ref card) = self.cards.c {
                result.push(Card { card: card.card });
            }
            let mut cur = &self.cards.next;
            while let Some(ref node) = cur {
                if let Some(ref card) = node.c {
                    result.push(Card { card: card.card });
                }
                cur = &node.next;
            }
            result
        }

        fn rebuild_list(&mut self, cards: Vec<Card>) {
            if cards.is_empty() {
                self.cards = CardCollection { prev: None, next: None, c: None };
                self.len = 0;
                return;
            }
            self.len = cards.len() as u8;
            let mut iter = cards.into_iter().rev();
            let last = iter.next().unwrap();
            let mut current = CardCollection {
                prev: None,
                next: None,
                c: Some(last),
            };
            for card in iter {
                let node = CardCollection {
                    prev: None,
                    next: Some(Box::new(current)),
                    c: Some(card),
                };
                current = node;
            }
            self.cards = current;
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len { return; }
            let card_byte = match c {
                Some(card) => card.card,
                None => return,
            };
            let new_card_opt = Some(Card { card: card_byte });
            let mut cards = self.collect_cards();
            let sorter = self.sorter;

            if cards.is_empty() {
                cards.push(Card { card: card_byte });
            } else {
                let mut insert_pos = None;
                // Check before first
                let after_opt = Some(Card { card: cards[0].card });
                if sorter(&None, &new_card_opt, &after_opt) != 0 {
                    insert_pos = Some(0);
                }
                if insert_pos.is_none() {
                    for i in 0..cards.len() - 1 {
                        let before_opt = Some(Card { card: cards[i].card });
                        let after_opt = Some(Card { card: cards[i + 1].card });
                        if sorter(&before_opt, &new_card_opt, &after_opt) != 0 {
                            insert_pos = Some(i + 1);
                            break;
                        }
                    }
                }
                if insert_pos.is_none() {
                    let before_opt = Some(Card { card: cards.last().unwrap().card });
                    if sorter(&before_opt, &new_card_opt, &None) != 0 {
                        insert_pos = Some(cards.len());
                    }
                }
                if let Some(pos) = insert_pos {
                    cards.insert(pos, Card { card: card_byte });
                }
            }
            self.rebuild_list(cards);
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.len as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            if self.len == 0 { return CardRank::InvalidRank; }
            let cards = self.collect_cards();
            let mut max_rank = CardRank::InvalidRank;
            for c in &cards {
                let r = c.get_card_rank();
                if max_rank == CardRank::InvalidRank || r > max_rank {
                    max_rank = r;
                }
            }
            max_rank
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let mut cards = self.collect_cards();
            let mut i = 0usize;
            let mut stopped = false;
            while !stopped && i < cards.len() {
                let len = cards.len() as u64;
                let card_opt = Some(Card { card: cards[i].card });
                match itr_fn(len, i as u64, &card_opt) {
                    ItrAction::Continue => { i += 1; }
                    ItrAction::Break => { stopped = true; }
                    ItrAction::RemoveAndContinue => {
                        cards.remove(i);
                        // pos stays same (next element slides into i), but C code
                        // decrements pos then increments, so net effect: same index
                        // Actually in C: pos -= 1 then pos++ at end of loop = same index
                    }
                    ItrAction::RemoveAndBreak => {
                        cards.remove(i);
                        stopped = true;
                    }
                }
            }
            self.rebuild_list(cards);
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let mut cards = self.collect_cards();
            cards.retain(|card| card.get_card_suit_rank() != c);
            self.rebuild_list(cards);
        }

        pub fn remove_from_hand_under_iter(&mut self, _collection: &CardCollection, _pos: usize) {
            // Not used directly in our implementation
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }
    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            // C returns: get_card_suit_rank(&d->cards[c]) == INVALID_CARD
            // In C deck, cards are zeroed (INVALID_CARD_BITS=0). A card is "in deck"
            // when its bits are 0 (invalid). is_card_in_deck returns non-zero if dealt.
            // Wait, the C doc says "return 0 if the card has been dealt or non-zero if
            // the card has been dealt" - that's a doc bug. Looking at the code:
            // return get_card_suit_rank(&d->cards[c]) == INVALID_CARD;
            // Cards start zeroed (INVALID_CARD_BITS=0), so get_card_suit_rank returns
            // INVALID_CARD for undealt cards. So is_card_in_deck returns 1 for cards
            // still in deck (not yet dealt/stripped).
            let idx = c as usize;
            if idx >= CardSuitRank::CardCount as usize { return 0; }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard { 1 } else { 0 }
        }

        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 { return None; }
            let selected = rand::random::<u64>() % self.card_count as u64;
            let mut valid_idx = 0u64;
            for i in 0..CardSuitRank::CardCount as usize {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_idx == selected {
                        self.cards[i] = Card::write_card(csr_from_usize(i));
                        self.card_count -= 1;
                        return Some(Card { card: self.cards[i].card });
                    }
                    valid_idx += 1;
                }
            }
            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= CardSuitRank::CardCount as usize { return; }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[idx] = Card::write_card(c);
                self.card_count -= 1;
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            let cards: [Card; CardSuitRank::CardCount as usize] =
                std::array::from_fn(|_| Card { card: 0 });
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
                cards,
            })
        }
    }

    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(_before: &Option<Card>, _new: &Option<Card>, after: &Option<Card>) -> i32 {
        if after.is_none() { 1 } else { 0 }
    }

    pub fn sort_card_by_rank(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let r = match new {
            Some(c) => c.get_card_rank(),
            None => return 0,
        };
        if after.is_none() {
            return 1;
        }
        let after_rank = after.as_ref().unwrap().get_card_rank();
        let before_ok = match before {
            None => true,
            Some(b) => r > b.get_card_rank(),
        };
        if before_ok && r <= after_rank { 1 } else { 0 }
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
