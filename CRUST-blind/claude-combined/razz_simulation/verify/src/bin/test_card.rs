use razz_simulation::card::card::{
    sort_card_after, sort_card_by_rank, Card, CardDeck, CardHand, CardRank, CardSuit,
    CardSuitRank, ItrAction,
};

#[test]
fn test_constants() {
    use razz_simulation::card::card::*;
    assert_eq!(INVALID_CARD_BITS, 0);
    assert_eq!(SPADE_BITS, 1 << 5);
    assert_eq!(HEART_BITS, 2 << 5);
    assert_eq!(DIAMOND_BITS, 3 << 5);
    assert_eq!(CLUB_BITS, 4 << 5);
    assert_eq!(SUIT_BITS, 0x7 << 5);
    assert_eq!(RANK_BITS, 0x1F);
    assert_eq!(ACE_BITS, 1);
    assert_eq!(R2_BITS, 2);
    assert_eq!(R3_BITS, 3);
    assert_eq!(R4_BITS, 4);
    assert_eq!(R5_BITS, 5);
    assert_eq!(R6_BITS, 6);
    assert_eq!(R7_BITS, 7);
    assert_eq!(R8_BITS, 8);
    assert_eq!(R9_BITS, 9);
    assert_eq!(R10_BITS, 10);
    assert_eq!(J_BITS, 11);
    assert_eq!(Q_BITS, 12);
    assert_eq!(K_BITS, 13);
}

#[test]
fn test_enum_ordering() {
    assert!(CardSuitRank::SpadeAce < CardSuitRank::SpadeK);
    assert!(CardSuitRank::HeartK < CardSuitRank::ClubK);
    assert!(CardRank::Ace < CardRank::R3);
}

#[test]
fn test_create_card_spade_ace() {
    let c = Card::create_card(CardSuitRank::SpadeAce).unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::SpadeAce);
    assert_eq!(c.get_card_rank(), CardRank::Ace);
    assert_eq!(c.get_card_suit(), CardSuit::Spade);
}

#[test]
fn test_create_card_club_8() {
    let c = Card::create_card(CardSuitRank::Club8).unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::Club8);
    assert_eq!(c.get_card_rank(), CardRank::R8);
    assert_eq!(c.get_card_suit(), CardSuit::Club);
}

#[test]
fn test_create_card_invalid() {
    let c = Card::create_card(CardSuitRank::CardCount);
    assert!(c.is_none());
}

#[test]
fn test_strtocard_s8() {
    let c = Card::strtocard("S8").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::Spade8);
    assert_eq!(c.get_card_rank(), CardRank::R8);
    assert_eq!(c.get_card_suit(), CardSuit::Spade);
}

#[test]
fn test_strtocard_dk_lowercase() {
    let c = Card::strtocard("dk").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::DiamondK);
    assert_eq!(c.get_card_rank(), CardRank::K);
    assert_eq!(c.get_card_suit(), CardSuit::Diamond);
}

#[test]
fn test_strtocard_ca() {
    let c = Card::strtocard("Ca").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::ClubAce);
    assert_eq!(c.get_card_rank(), CardRank::Ace);
    assert_eq!(c.get_card_suit(), CardSuit::Club);
}

#[test]
fn test_strtocard_hj() {
    let c = Card::strtocard("hJ").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::HeartJ);
    assert_eq!(c.get_card_rank(), CardRank::J);
    assert_eq!(c.get_card_suit(), CardSuit::Heart);
}

#[test]
fn test_strtocard_sq() {
    let c = Card::strtocard("SQ").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::SpadeQ);
    assert_eq!(c.get_card_rank(), CardRank::Q);
    assert_eq!(c.get_card_suit(), CardSuit::Spade);
}

#[test]
fn test_strtocard_invalid() {
    assert!(Card::strtocard("SS").is_none());
    assert!(Card::strtocard("S0").is_none());
    assert!(Card::strtocard("S1").is_none());
    assert!(Card::strtocard("a2").is_none());
}

#[test]
fn test_strtorank() {
    assert_eq!(CardRank::strtorank("ace"), CardRank::Ace);
    assert_eq!(CardRank::strtorank("8"), CardRank::R8);
    assert_eq!(CardRank::strtorank("K"), CardRank::K);
    assert_eq!(CardRank::strtorank("10"), CardRank::R10);
    assert_eq!(CardRank::strtorank("1"), CardRank::InvalidRank);
}

#[test]
fn test_cardtostr() {
    assert_eq!(
        CardSuitRank::Spade8.cardtostr(),
        Some("S8".to_string())
    );
    assert_eq!(
        CardSuitRank::Club10.cardtostr(),
        Some("C10".to_string())
    );
    assert_eq!(
        CardSuitRank::SpadeAce.cardtostr(),
        Some("SA".to_string())
    );
    assert_eq!(
        CardSuitRank::ClubK.cardtostr(),
        Some("CK".to_string())
    );
    assert_eq!(CardSuitRank::CardCount.cardtostr(), None);
}

#[test]
fn test_ranktostr() {
    assert_eq!(CardRank::R8.ranktostr(), Some("8".to_string()));
    assert_eq!(CardRank::R10.ranktostr(), Some("10".to_string()));
    assert_eq!(CardRank::Ace.ranktostr(), Some("A".to_string()));
    assert_eq!(CardRank::K.ranktostr(), Some("K".to_string()));
    assert_eq!(CardRank::InvalidRank.ranktostr(), None);
}

#[test]
fn test_deck_creation_and_strip() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    // All 52 cards should be in deck initially
    assert_ne!(d.is_card_in_deck(CardSuitRank::HeartK), 0);
    d.strip_card_from_deck(CardSuitRank::HeartK);
    assert_eq!(d.is_card_in_deck(CardSuitRank::HeartK), 0);
}

#[test]
fn test_deal_from_deck_count() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    // Deal 52 cards; then next is None
    let mut dealt: Vec<CardSuitRank> = Vec::new();
    for _ in 0..52 {
        let c = d.deal_from_deck();
        assert!(c.is_some(), "Should deal a card");
        let csr = c.unwrap().get_card_suit_rank();
        // It should not have been dealt before
        assert!(!dealt.iter().any(|x| *x == csr));
        dealt.push(csr);
        assert_eq!(d.is_card_in_deck(csr), 0);
    }
    let last = d.deal_from_deck();
    assert!(last.is_none(), "Empty deck deals None");
}

#[test]
fn test_sort_card_after() {
    let c1 = Some(Card::create_card(CardSuitRank::SpadeAce).unwrap());
    let c2 = Some(Card::create_card(CardSuitRank::Heart2).unwrap());
    // after is None means insert at end -> 1
    assert_eq!(sort_card_after(&c1, &c2, &None), 1);
    // after is some means don't insert here -> 0
    assert_eq!(sort_card_after(&c1, &c2, &c2), 0);
    assert_eq!(sort_card_after(&None, &c2, &c1), 0);
}

#[test]
fn test_sort_card_by_rank() {
    let ace = Some(Card::create_card(CardSuitRank::SpadeAce).unwrap());
    let two = Some(Card::create_card(CardSuitRank::Heart2).unwrap());
    let king = Some(Card::create_card(CardSuitRank::ClubK).unwrap());

    // after == NULL -> 1 (insert at end)
    assert_eq!(sort_card_by_rank(&ace, &two, &None), 1);

    // before == NULL && new<= after.rank -> 1
    assert_eq!(sort_card_by_rank(&None, &ace, &two), 1);
    // before == NULL but new.rank > after.rank -> 0
    assert_eq!(sort_card_by_rank(&None, &king, &two), 0);

    // before.rank < new.rank <= after.rank -> 1
    assert_eq!(sort_card_by_rank(&ace, &two, &king), 1);
    // before.rank == new.rank -> r > before.rank is false -> 0
    assert_eq!(sort_card_by_rank(&two, &two, &king), 0);
    // before.rank > new.rank -> r > before.rank is false -> 0
    assert_eq!(sort_card_by_rank(&king, &ace, &two), 0);
}

#[test]
fn test_create_hand_basic() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    assert_eq!(h.count_cards_in_hand(), 0);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);
}

#[test]
fn test_hand_insert_and_max() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();

    let c_ace = Card::create_card(CardSuitRank::SpadeAce);
    h.insert_into_hand(&c_ace);
    assert_eq!(h.count_cards_in_hand(), 1);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Ace);

    let c_q = Card::create_card(CardSuitRank::HeartQ);
    h.insert_into_hand(&c_q);
    assert_eq!(h.count_cards_in_hand(), 2);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);

    let c_2 = Card::create_card(CardSuitRank::Diamond2);
    h.insert_into_hand(&c_2);
    assert_eq!(h.count_cards_in_hand(), 3);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);
}

#[test]
fn test_hand_full() {
    let mut h = CardHand::create_hand(2, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond3));
    assert_eq!(h.count_cards_in_hand(), 2);
    assert_eq!(h.get_max_of_hand(), 2);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R2);
}

#[test]
fn test_hand_reset() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    assert_eq!(h.count_cards_in_hand(), 2);
    h.reset_hand();
    assert_eq!(h.count_cards_in_hand(), 0);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);
}

#[test]
fn test_hand_remove() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond6));
    h.insert_into_hand(&Card::create_card(CardSuitRank::HeartQ));
    assert_eq!(h.count_cards_in_hand(), 4);

    h.remove_from_hand(CardSuitRank::Diamond6);
    assert_eq!(h.count_cards_in_hand(), 3);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);

    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 2);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R2);
}

#[test]
fn test_iterate_hand_basic() {
    fn iter_fn(_len: u64, _pos: u64, _c: &Option<Card>) -> ItrAction {
        ItrAction::Continue
    }
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    h.iterate_hand(iter_fn);
    assert_eq!(h.count_cards_in_hand(), 2);
}

#[test]
fn test_iterate_hand_remove_continue() {
    // Test removing non-head positions (mirrors length_trimmer in razz)
    fn iter_fn(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
        if pos >= 2 {
            ItrAction::RemoveAndContinue
        } else {
            ItrAction::Continue
        }
    }
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    // Insert in rank order: A, 2, 3, 4
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond3));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Club4));
    assert_eq!(h.count_cards_in_hand(), 4);
    h.iterate_hand(iter_fn);
    // pos 0 (A) and 1 (2) kept; pos 2 (3) and 3 (4) removed
    assert_eq!(h.count_cards_in_hand(), 2);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R2);
}

#[test]
fn test_iterate_hand_break() {
    use std::sync::Mutex;
    static COUNTER: Mutex<u64> = Mutex::new(0);
    fn iter_fn(_len: u64, _pos: u64, _c: &Option<Card>) -> ItrAction {
        let mut c = COUNTER.lock().unwrap();
        *c += 1;
        if *c >= 2 {
            ItrAction::Break
        } else {
            ItrAction::Continue
        }
    }
    *COUNTER.lock().unwrap() = 0;
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond3));
    h.iterate_hand(iter_fn);
    assert_eq!(*COUNTER.lock().unwrap(), 2);
    assert_eq!(h.count_cards_in_hand(), 3);
}

#[test]
fn test_strip_card_from_deck() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    assert_ne!(d.is_card_in_deck(CardSuitRank::SpadeAce), 0);
    d.strip_card_from_deck(CardSuitRank::SpadeAce);
    assert_eq!(d.is_card_in_deck(CardSuitRank::SpadeAce), 0);
    // Stripping again should be no-op (already stripped)
    d.strip_card_from_deck(CardSuitRank::SpadeAce);
    assert_eq!(d.is_card_in_deck(CardSuitRank::SpadeAce), 0);
}

fn main() {}
