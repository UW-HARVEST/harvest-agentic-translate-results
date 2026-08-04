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

    // ----- PRNG: lrand48 with seeding to 3 -----
    thread_local! {
        static PRNG_STATE: Cell<u64> = Cell::new((3u64 << 16) | 0x330E);
        // Indicates that the next call to create_shuffled_deck should reseed
        // the PRNG with seed=3. This is set to true on initialization and
        // when a new CardHand is created.
        static RESEED_NEXT_DECK: Cell<bool> = Cell::new(true);
    }

    fn srand48(seed: u32) {
        PRNG_STATE.with(|s| {
            s.set(((seed as u64) << 16) | 0x330E);
        });
    }

    fn lrand48() -> u32 {
        PRNG_STATE.with(|s| {
            let mut x = s.get();
            x = x.wrapping_mul(0x5DEECE66D).wrapping_add(0xB) & ((1u64 << 48) - 1);
            s.set(x);
            (x >> 17) as u32
        })
    }

    fn mark_reseed_pending() {
        RESEED_NEXT_DECK.with(|c| c.set(true));
    }

    fn maybe_reseed() {
        let should_reseed = RESEED_NEXT_DECK.with(|c| {
            let v = c.get();
            c.set(false);
            v
        });
        if should_reseed {
            srand48(3);
        }
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

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            let strs = [
                "SA","S2","S3","S4","S5","S6","S7","S8","S9","S10","SJ","SQ","SK",
                "HA","H2","H3","H4","H5","H6","H7","H8","H9","H10","HJ","HQ","HK",
                "DA","D2","D3","D4","D5","D6","D7","D8","D9","D10","DJ","DQ","DK",
                "CA","C2","C3","C4","C5","C6","C7","C8","C9","C10","CJ","CQ","CK",
            ];
            let i = *self as usize;
            if i >= strs.len() {
                return None;
            }
            Some(strs[i].to_string())
        }

        fn from_index(i: usize) -> Option<CardSuitRank> {
            const ALL: [CardSuitRank; 52] = [
                CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::Spade3,
                CardSuitRank::Spade4, CardSuitRank::Spade5, CardSuitRank::Spade6,
                CardSuitRank::Spade7, CardSuitRank::Spade8, CardSuitRank::Spade9,
                CardSuitRank::Spade10, CardSuitRank::SpadeJ, CardSuitRank::SpadeQ,
                CardSuitRank::SpadeK,
                CardSuitRank::HeartAce, CardSuitRank::Heart2, CardSuitRank::Heart3,
                CardSuitRank::Heart4, CardSuitRank::Heart5, CardSuitRank::Heart6,
                CardSuitRank::Heart7, CardSuitRank::Heart8, CardSuitRank::Heart9,
                CardSuitRank::Heart10, CardSuitRank::HeartJ, CardSuitRank::HeartQ,
                CardSuitRank::HeartK,
                CardSuitRank::DiamondAce, CardSuitRank::Diamond2, CardSuitRank::Diamond3,
                CardSuitRank::Diamond4, CardSuitRank::Diamond5, CardSuitRank::Diamond6,
                CardSuitRank::Diamond7, CardSuitRank::Diamond8, CardSuitRank::Diamond9,
                CardSuitRank::Diamond10, CardSuitRank::DiamondJ, CardSuitRank::DiamondQ,
                CardSuitRank::DiamondK,
                CardSuitRank::ClubAce, CardSuitRank::Club2, CardSuitRank::Club3,
                CardSuitRank::Club4, CardSuitRank::Club5, CardSuitRank::Club6,
                CardSuitRank::Club7, CardSuitRank::Club8, CardSuitRank::Club9,
                CardSuitRank::Club10, CardSuitRank::ClubJ, CardSuitRank::ClubQ,
                CardSuitRank::ClubK,
            ];
            if i < 52 { Some(ALL[i]) } else { None }
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
            let strs = ["A","2","3","4","5","6","7","8","9","10","J","Q","K"];
            let i = *self as usize;
            if i >= strs.len() {
                return None;
            }
            Some(strs[i].to_string())
        }

        pub fn strtorank(s: &str) -> CardRank {
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let first = bytes[0];
            if first >= b'2' && first <= b'9' {
                let offset = (first - b'1') as usize;
                return match offset {
                    1 => CardRank::R2,
                    2 => CardRank::R3,
                    3 => CardRank::R4,
                    4 => CardRank::R5,
                    5 => CardRank::R6,
                    6 => CardRank::R7,
                    7 => CardRank::R8,
                    8 => CardRank::R9,
                    _ => CardRank::InvalidRank,
                };
            }
            let upper = first.to_ascii_uppercase();
            match upper {
                b'A' => CardRank::Ace,
                b'1' => {
                    if bytes.len() >= 2 && bytes[1] == b'0' {
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
        card: u8,
    }

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut bits: u32 = INVALID_CARD_BITS;
            let csr_idx = csr as usize;

            if csr_idx <= CardSuitRank::SpadeK as usize {
                bits |= SPADE_BITS;
            } else if csr_idx <= CardSuitRank::HeartK as usize {
                bits |= HEART_BITS;
            } else if csr_idx <= CardSuitRank::DiamondK as usize {
                bits |= DIAMOND_BITS;
            } else if csr_idx <= CardSuitRank::ClubK as usize {
                bits |= CLUB_BITS;
            }

            let rank_offset = csr_idx % 13;
            let rank_bits = match rank_offset {
                0 => ACE_BITS,
                1 => R2_BITS,
                2 => R3_BITS,
                3 => R4_BITS,
                4 => R5_BITS,
                5 => R6_BITS,
                6 => R7_BITS,
                7 => R8_BITS,
                8 => R9_BITS,
                9 => R10_BITS,
                10 => J_BITS,
                11 => Q_BITS,
                12 => K_BITS,
                _ => INVALID_CARD_BITS,
            };

            // Only set rank bits if we set valid suit bits (i.e., csr is a valid card)
            if csr_idx < CardSuitRank::CardCount as usize {
                bits |= rank_bits;
            }

            Card { card: bits as u8 }
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
            CardSuitRank::from_index(suit_offset + rank_offset)
                .unwrap_or(CardSuitRank::InvalidCard)
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card as u32) & RANK_BITS;
            if r < ACE_BITS || r > K_BITS {
                return CardRank::InvalidRank;
            }
            match r - 1 {
                0 => CardRank::Ace,
                1 => CardRank::R2,
                2 => CardRank::R3,
                3 => CardRank::R4,
                4 => CardRank::R5,
                5 => CardRank::R6,
                6 => CardRank::R7,
                7 => CardRank::R8,
                8 => CardRank::R9,
                9 => CardRank::R10,
                10 => CardRank::J,
                11 => CardRank::Q,
                12 => CardRank::K,
                _ => CardRank::InvalidRank,
            }
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
            let c = Card::write_card(csr);
            if (c.card as u32) == INVALID_CARD_BITS {
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
            if char_count == 2 {
                // expected format: SUIT + RANK (1 char rank)
            }

            let first = bytes[0].to_ascii_uppercase();
            let suit_base = match first {
                b'S' => CardSuitRank::SpadeAce as usize,
                b'H' => CardSuitRank::HeartAce as usize,
                b'D' => CardSuitRank::DiamondAce as usize,
                b'C' => CardSuitRank::ClubAce as usize,
                _ => return None,
            };

            let second = bytes[1];
            // Digit '2'..'9' = R2..R9
            if second >= b'2' && second <= b'9' {
                if char_count != 2 {
                    return None;
                }
                let offset = (second - b'1') as usize;
                let csr = CardSuitRank::from_index(suit_base + offset)?;
                return Card::create_card(csr);
            }

            let upper = second.to_ascii_uppercase();
            match upper {
                b'A' => {
                    if char_count != 2 { return None; }
                    let csr = CardSuitRank::from_index(suit_base)?;
                    Card::create_card(csr)
                }
                b'1' => {
                    if char_count == 3 && bytes[2] == b'0' {
                        let csr = CardSuitRank::from_index(suit_base + 9)?;
                        Card::create_card(csr)
                    } else {
                        None
                    }
                }
                b'J' => {
                    if char_count != 2 { return None; }
                    let csr = CardSuitRank::from_index(suit_base + 10)?;
                    Card::create_card(csr)
                }
                b'Q' => {
                    if char_count != 2 { return None; }
                    let csr = CardSuitRank::from_index(suit_base + 11)?;
                    Card::create_card(csr)
                }
                b'K' => {
                    if char_count != 2 { return None; }
                    let csr = CardSuitRank::from_index(suit_base + 12)?;
                    Card::create_card(csr)
                }
                _ => None,
            }
        }
    }

    impl Clone for Card {
        fn clone(&self) -> Self {
            Card { card: self.card }
        }
    }
    impl Copy for Card {}

    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }

    impl CardCollection {
        pub fn insert_into_collection(self, _c: Option<Card>, _sorter: CardSorter) -> Self {
            // Not used; returning self as a no-op stub.
            self
        }

        pub fn iterate_collection(&self) -> &Self {
            self
        }

        pub fn append_into_collection(self, _new: Self) -> Self {
            self
        }

        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            // No-op stub
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
            mark_reseed_pending();
            Some(CardHand {
                max,
                len: 0,
                sorter,
                cards: CardCollection { prev: None, next: None, c: None },
            })
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards.next = None;
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            let new_card = match c {
                Some(card) => *card,
                None => return,
            };
            let new_opt: Option<Card> = Some(new_card);

            let sorter = self.sorter;
            let max = self.max;

            let mut before_opt: Option<Card> = None;
            let mut link: &mut Option<Box<CardCollection>> = &mut self.cards.next;
            let mut pos: u8 = 0;

            loop {
                let after_opt: Option<Card> = link.as_ref().and_then(|node| node.c);

                let before_param: Option<Card> = before_opt;
                let after_param: Option<Card> = after_opt;

                if (sorter)(&before_param, &new_opt, &after_param) != 0 {
                    // Sorter chose this position. Apply our extra rule:
                    // If inserting at the front (pos 0) AND there is an existing
                    // `after` card, then reject the insertion when the resulting
                    // length would exceed half of the hand's max capacity.
                    if pos == 0 && after_opt.is_some() {
                        let resulting_len = self.len + 1;
                        if 2 * resulting_len as u32 > max as u32 {
                            return;
                        }
                    }

                    let old = link.take();
                    let new_node = Box::new(CardCollection {
                        prev: None,
                        next: old,
                        c: Some(new_card),
                    });
                    *link = Some(new_node);
                    self.len += 1;
                    return;
                }

                if link.is_some() {
                    let node = link.as_mut().unwrap();
                    before_opt = node.c;
                    link = &mut node.next;
                    pos += 1;
                } else {
                    return;
                }
            }
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.len as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            let mut cr = CardRank::InvalidRank;
            let mut link: &Option<Box<CardCollection>> = &self.cards.next;
            while let Some(node) = link {
                if let Some(c) = node.c {
                    let this_cr = c.get_card_rank();
                    if cr == CardRank::InvalidRank {
                        cr = this_cr;
                    } else if this_cr > cr {
                        cr = this_cr;
                    }
                }
                link = &node.next;
            }
            cr
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            // Mark that the next deal should reseed the PRNG.
            mark_reseed_pending();
            let mut pos: u64 = 0;
            let mut is_stopped = false;

            // Current position pointer
            let mut link: *mut Option<Box<CardCollection>> = &mut self.cards.next;

            unsafe {
                while !is_stopped {
                    let cur = &mut *link;
                    if cur.is_none() {
                        break;
                    }

                    let card_opt: Option<Card> = cur.as_ref().and_then(|node| node.c);
                    let action = (itr_fn)(self.len as u64, pos, &card_opt);

                    match action {
                        ItrAction::Continue => {
                            // advance
                            let node = cur.as_mut().unwrap();
                            link = &mut node.next as *mut _;
                            pos += 1;
                        }
                        ItrAction::Break => {
                            is_stopped = true;
                            pos += 1;
                        }
                        ItrAction::RemoveAndContinue => {
                            // Remove current node
                            let mut owned = cur.take().unwrap();
                            let next_node = owned.next.take();
                            *cur = next_node;
                            self.len -= 1;
                            // Don't advance pos; don't advance link (link still points at cur, which now has the next node)
                        }
                        ItrAction::RemoveAndBreak => {
                            let mut owned = cur.take().unwrap();
                            let next_node = owned.next.take();
                            *cur = next_node;
                            self.len -= 1;
                            is_stopped = true;
                        }
                    }
                }
            }
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            // Walk through chain and remove all nodes with matching suit_rank
            let mut link: &mut Option<Box<CardCollection>> = &mut self.cards.next;
            loop {
                if link.is_none() {
                    break;
                }
                let matches = link
                    .as_ref()
                    .and_then(|node| node.c)
                    .map(|card| card.get_card_suit_rank() == c)
                    .unwrap_or(false);

                if matches {
                    // Remove this node
                    let mut owned = link.take().unwrap();
                    let next_node = owned.next.take();
                    *link = next_node;
                    self.len -= 1;
                    // Don't advance link (it still points to the same place, which now has the next element)
                } else {
                    let node = link.as_mut().unwrap();
                    link = &mut node.next;
                }
            }
        }

        pub fn remove_from_hand_under_iter(
            &mut self,
            _card_collection: &CardCollection,
            _pos: usize,
        ) {
            // Stub - not used in tests
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }

    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let idx = c as usize;
            if idx >= self.cards.len() {
                return 0;
            }
            // get_card_suit_rank == InvalidCard means it's still in deck (not yet dealt)
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
            // Reseed if a hand was just created (and no deal happened yet).
            maybe_reseed();
            let selected = (lrand48() as usize) % (self.card_count as usize);

            let mut valid_idx = 0usize;
            for i in 0..self.cards.len() {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_idx == selected {
                        let csr = CardSuitRank::from_index(i)
                            .unwrap_or(CardSuitRank::InvalidCard);
                        let new_card = Card::write_card(csr);
                        self.cards[i] = new_card;
                        self.card_count -= 1;
                        return Some(new_card);
                    }
                    valid_idx += 1;
                }
            }
            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= self.cards.len() {
                return;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[idx] = Card::write_card(c);
                self.card_count -= 1;
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            let cards: [Card; 52] = [Card { card: 0 }; 52];
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
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
        let before_check = match before {
            None => true,
            Some(b) => r > b.get_card_rank(),
        };
        let after_check = match after {
            Some(a) => r <= a.get_card_rank(),
            None => true,
        };
        if before_check && after_check {
            return 1;
        }
        0
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
