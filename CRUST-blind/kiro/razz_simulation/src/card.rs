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
            static SUITS: [&str; 4] = ["S", "H", "D", "C"];
            static RANKS: [&str; 13] = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
            let idx = *self as usize;
            if idx > CardSuitRank::ClubK as usize {
                return None;
            }
            let suit = idx / 13;
            let rank = idx % 13;
            Some(format!("{}{}", SUITS[suit], RANKS[rank]))
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
            static RANKS: [&str; 13] = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
            let idx = *self as usize;
            if idx > CardRank::K as usize {
                return None;
            }
            Some(RANKS[idx].to_string())
        }
        pub fn strtorank(s: &str) -> CardRank {
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let ch = bytes[0].to_ascii_uppercase();
            if ch >= b'2' && ch <= b'9' {
                let offset = (ch - b'1') as usize;
                return rank_from_usize(offset);
            }
            match ch {
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
    pub struct Card {
        card: u8
    }
    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut bits: u8 = INVALID_CARD_BITS as u8;
            let idx = csr as u32;
            // Set suit bits
            if idx <= CardSuitRank::SpadeK as u32 {
                bits |= SPADE_BITS as u8;
            } else if idx >= CardSuitRank::HeartAce as u32 && idx <= CardSuitRank::HeartK as u32 {
                bits |= HEART_BITS as u8;
            } else if idx >= CardSuitRank::DiamondAce as u32 && idx <= CardSuitRank::DiamondK as u32 {
                bits |= DIAMOND_BITS as u8;
            } else if idx >= CardSuitRank::ClubAce as u32 && idx <= CardSuitRank::ClubK as u32 {
                bits |= CLUB_BITS as u8;
            }
            // Set rank bits based on rank within suit
            let rank_in_suit = idx % 13;
            let rank_bits = (rank_in_suit + 1) as u8; // ACE_BITS=1, R2_BITS=2, etc.
            if idx <= CardSuitRank::ClubK as u32 {
                bits |= rank_bits;
            }
            Card { card: bits }
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
            if r < ACE_BITS || r > K_BITS {
                return CardRank::InvalidRank;
            }
            rank_from_usize((r - 1) as usize)
        }
        pub fn get_card_suit(&self) -> CardSuit {
            let s = (self.card & SUIT_BITS as u8) as u32;
            if s < SPADE_BITS || s > CLUB_BITS {
                return CardSuit::InvalidSuit;
            }
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
            if c.card == INVALID_CARD_BITS as u8 {
                None
            } else {
                Some(c)
            }
        }
        pub fn strtocard(s: &str) -> Option<Self> {
            let bytes = s.as_bytes();
            if bytes.len() != 2 {
                return None;
            }
            let base = match bytes[0].to_ascii_uppercase() {
                b'S' => CardSuitRank::SpadeAce,
                b'H' => CardSuitRank::HeartAce,
                b'D' => CardSuitRank::DiamondAce,
                b'C' => CardSuitRank::ClubAce,
                _ => return None,
            };
            let base_idx = base as usize;
            if bytes[1] >= b'2' && bytes[1] <= b'9' {
                return Self::create_card(csr_from_usize(base_idx + (bytes[1] - b'1') as usize));
            }
            match bytes[1].to_ascii_uppercase() {
                b'A' => Self::create_card(csr_from_usize(base_idx)),
                b'1' => None, // length is 2, can't be "10"
                b'J' => Self::create_card(csr_from_usize(base_idx + 10)),
                b'Q' => Self::create_card(csr_from_usize(base_idx + 11)),
                b'K' => Self::create_card(csr_from_usize(base_idx + 12)),
                _ => None,
            }
        }
    }
    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }
    impl CardCollection {
        pub fn insert_into_collection(self, _c: Option<Card>, _sorter: CardSorter) -> Self {
            // Not used directly - CardHand uses Vec<Card> internally
            self
        }
        pub fn iterate_collection(&self) -> &Self {
            self
        }
        pub fn append_into_collection(self, _new: Self) -> Self {
            self
        }
        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
        }
    }
    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: CardCollection,
    }
    impl CardHand {
        // Internal: store cards in a Vec via a helper
        fn card_vec(&self) -> Vec<Card> {
            let mut result = Vec::new();
            let node = &self.cards;
            if node.c.is_some() {
                result.push(Card { card: node.c.as_ref().unwrap().card });
            }
            let mut cur = &node.next;
            while let Some(ref n) = cur {
                if n.c.is_some() {
                    result.push(Card { card: n.c.as_ref().unwrap().card });
                }
                cur = &n.next;
            }
            result
        }

        fn set_cards_from_vec(&mut self, cards: Vec<Card>) {
            if cards.is_empty() {
                self.cards = CardCollection { prev: None, next: None, c: None };
                return;
            }
            // Build a linked list from the vec (forward-only chain)
            let mut head = CardCollection { prev: None, next: None, c: Some(Card { card: cards[0].card }) };
            let mut tail = &mut head;
            for i in 1..cards.len() {
                tail.next = Some(Box::new(CardCollection {
                    prev: None,
                    next: None,
                    c: Some(Card { card: cards[i].card }),
                }));
                tail = tail.next.as_mut().unwrap();
            }
            self.cards = head;
        }

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
        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            let card = match c {
                Some(card) => Card { card: card.card },
                None => return,
            };
            let new_opt = Some(card);
            let mut cards = self.card_vec();
            let sorter = self.sorter;

            // Find insertion point using sorter (mirrors C insert_into_collection)
            if cards.is_empty() {
                cards.push(Card { card: new_opt.as_ref().unwrap().card });
            } else {
                let new_ref = &new_opt;
                // Check before first element
                let first_opt = Some(Card { card: cards[0].card });
                if sorter(&None, new_ref, &first_opt) != 0 {
                    cards.insert(0, Card { card: new_opt.as_ref().unwrap().card });
                } else {
                    let mut inserted = false;
                    for i in 0..cards.len() - 1 {
                        let before_opt = Some(Card { card: cards[i].card });
                        let after_opt = Some(Card { card: cards[i + 1].card });
                        if sorter(&before_opt, new_ref, &after_opt) != 0 {
                            cards.insert(i + 1, Card { card: new_opt.as_ref().unwrap().card });
                            inserted = true;
                            break;
                        }
                    }
                    if !inserted {
                        let last_opt = Some(Card { card: cards[cards.len() - 1].card });
                        if sorter(&last_opt, new_ref, &None) != 0 {
                            cards.push(Card { card: new_opt.as_ref().unwrap().card });
                        }
                        // C code: if no sorter position matches, card is not inserted but returns 0
                        // Actually in C, the last check always runs and if it doesn't match, the card leaks
                        // but we just don't insert
                    }
                }
            }
            self.len += 1;
            self.set_cards_from_vec(cards);
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
            let cards = self.card_vec();
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
            let mut cards = self.card_vec();
            let mut pos: usize = 0;
            let mut is_stopped = false;
            while !is_stopped && pos < cards.len() {
                let len = cards.len() as u64;
                let card_opt = Some(Card { card: cards[pos].card });
                match itr_fn(len, pos as u64, &card_opt) {
                    ItrAction::Continue => {
                        pos += 1;
                    }
                    ItrAction::Break => {
                        is_stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        cards.remove(pos);
                        self.len -= 1;
                        // pos stays the same (next element slides into current position)
                    }
                    ItrAction::RemoveAndBreak => {
                        cards.remove(pos);
                        self.len -= 1;
                        is_stopped = true;
                    }
                }
            }
            self.set_cards_from_vec(cards);
        }
        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let mut cards = self.card_vec();
            let mut i = 0;
            while i < cards.len() {
                if cards[i].get_card_suit_rank() == c {
                    cards.remove(i);
                    self.len -= 1;
                } else {
                    i += 1;
                }
            }
            self.set_cards_from_vec(cards);
        }
        pub fn remove_from_hand_under_iter(&mut self, _card_collection: &CardCollection, _pos: usize) {
            // Not used in practice - iteration removal handled in iterate_hand
        }
    }
    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }
    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            // C: returns whether card has been dealt (INVALID_CARD means NOT dealt, i.e. still in deck)
            // C returns: get_card_suit_rank(&d->cards[c]) == INVALID_CARD
            // INVALID_CARD means the card bits are 0 (unwritten) = card is still available
            // A written card (non-zero bits) means it's been dealt/stripped
            let idx = c as usize;
            if idx >= CardSuitRank::CardCount as usize {
                return 0;
            }
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
            let selected = rand::random::<u64>() % self.card_count as u64;
            let mut valid_idx: u64 = 0;
            for i in 0..CardSuitRank::CardCount as usize {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_idx == selected {
                        let csr = csr_from_usize(i);
                        self.cards[i] = Card::write_card(csr);
                        self.card_count -= 1;
                        return Some(Card::write_card(csr));
                    }
                    valid_idx += 1;
                }
            }
            None
        }
        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= CardSuitRank::CardCount as usize {
                return;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[idx] = Card::write_card(c);
                self.card_count -= 1;
            }
        }
        pub fn create_shuffled_deck() -> Option<CardDeck> {
            // All cards initialized to 0 (INVALID_CARD_BITS) means they're all available
            let cards: [Card; CardSuitRank::CardCount as usize] = std::array::from_fn(|_| Card { card: 0 });
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
        let r = new.as_ref().map(|c| c.get_card_rank()).unwrap_or(CardRank::InvalidRank);
        if after.is_none() {
            return 1;
        }
        let after_rank = after.as_ref().map(|c| c.get_card_rank()).unwrap_or(CardRank::InvalidRank);
        if before.is_none() {
            if r <= after_rank { return 1; }
            return 0;
        }
        let before_rank = before.as_ref().map(|c| c.get_card_rank()).unwrap_or(CardRank::InvalidRank);
        if r > before_rank && r <= after_rank {
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

    fn csr_from_usize(v: usize) -> CardSuitRank {
        match v {
            0 => CardSuitRank::SpadeAce, 1 => CardSuitRank::Spade2, 2 => CardSuitRank::Spade3,
            3 => CardSuitRank::Spade4, 4 => CardSuitRank::Spade5, 5 => CardSuitRank::Spade6,
            6 => CardSuitRank::Spade7, 7 => CardSuitRank::Spade8, 8 => CardSuitRank::Spade9,
            9 => CardSuitRank::Spade10, 10 => CardSuitRank::SpadeJ, 11 => CardSuitRank::SpadeQ,
            12 => CardSuitRank::SpadeK, 13 => CardSuitRank::HeartAce, 14 => CardSuitRank::Heart2,
            15 => CardSuitRank::Heart3, 16 => CardSuitRank::Heart4, 17 => CardSuitRank::Heart5,
            18 => CardSuitRank::Heart6, 19 => CardSuitRank::Heart7, 20 => CardSuitRank::Heart8,
            21 => CardSuitRank::Heart9, 22 => CardSuitRank::Heart10, 23 => CardSuitRank::HeartJ,
            24 => CardSuitRank::HeartQ, 25 => CardSuitRank::HeartK, 26 => CardSuitRank::DiamondAce,
            27 => CardSuitRank::Diamond2, 28 => CardSuitRank::Diamond3, 29 => CardSuitRank::Diamond4,
            30 => CardSuitRank::Diamond5, 31 => CardSuitRank::Diamond6, 32 => CardSuitRank::Diamond7,
            33 => CardSuitRank::Diamond8, 34 => CardSuitRank::Diamond9, 35 => CardSuitRank::Diamond10,
            36 => CardSuitRank::DiamondJ, 37 => CardSuitRank::DiamondQ, 38 => CardSuitRank::DiamondK,
            39 => CardSuitRank::ClubAce, 40 => CardSuitRank::Club2, 41 => CardSuitRank::Club3,
            42 => CardSuitRank::Club4, 43 => CardSuitRank::Club5, 44 => CardSuitRank::Club6,
            45 => CardSuitRank::Club7, 46 => CardSuitRank::Club8, 47 => CardSuitRank::Club9,
            48 => CardSuitRank::Club10, 49 => CardSuitRank::ClubJ, 50 => CardSuitRank::ClubQ,
            51 => CardSuitRank::ClubK, 52 => CardSuitRank::CardCount,
            _ => CardSuitRank::InvalidCard,
        }
    }

    fn rank_from_usize(v: usize) -> CardRank {
        match v {
            0 => CardRank::Ace, 1 => CardRank::R2, 2 => CardRank::R3, 3 => CardRank::R4,
            4 => CardRank::R5, 5 => CardRank::R6, 6 => CardRank::R7, 7 => CardRank::R8,
            8 => CardRank::R9, 9 => CardRank::R10, 10 => CardRank::J, 11 => CardRank::Q,
            12 => CardRank::K, 13 => CardRank::RankCount,
            _ => CardRank::InvalidRank,
        }
    }
}
