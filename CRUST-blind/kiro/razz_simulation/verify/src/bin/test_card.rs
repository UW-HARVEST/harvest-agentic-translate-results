use razz_simulation::card::card::*;
use std::cell::RefCell;

// --- CardSuitRank enum ordering ---
#[test]
fn test_enum_ordering() {
    assert!(CardSuitRank::SpadeAce < CardSuitRank::SpadeK);
    assert!(CardSuitRank::HeartK < CardSuitRank::ClubK);
    assert!(CardRank::Ace < CardRank::R3);
    assert_eq!(CardSuitRank::SpadeAce as usize, 0);
    assert_eq!(CardSuitRank::CardCount as usize, 52);
}

// --- CardSuitRank::cardtostr ---
#[test]
fn test_cardtostr() {
    assert_eq!(CardSuitRank::SpadeAce.cardtostr(), Some("SA".to_string()));
    assert_eq!(CardSuitRank::Spade8.cardtostr(), Some("S8".to_string()));
    assert_eq!(CardSuitRank::Club10.cardtostr(), Some("C10".to_string()));
    assert_eq!(CardSuitRank::ClubK.cardtostr(), Some("CK".to_string()));
    assert_eq!(CardSuitRank::HeartJ.cardtostr(), Some("HJ".to_string()));
    assert_eq!(CardSuitRank::DiamondQ.cardtostr(), Some("DQ".to_string()));
    assert_eq!(CardSuitRank::CardCount.cardtostr(), None);
    assert_eq!(CardSuitRank::InvalidCard.cardtostr(), None);
}

// --- CardRank::ranktostr ---
#[test]
fn test_ranktostr() {
    assert_eq!(CardRank::Ace.ranktostr(), Some("A".to_string()));
    assert_eq!(CardRank::R8.ranktostr(), Some("8".to_string()));
    assert_eq!(CardRank::R10.ranktostr(), Some("10".to_string()));
    assert_eq!(CardRank::K.ranktostr(), Some("K".to_string()));
    assert_eq!(CardRank::InvalidRank.ranktostr(), None);
    assert_eq!(CardRank::RankCount.ranktostr(), None);
}

// --- CardRank::strtorank ---
#[test]
fn test_strtorank() {
    assert_eq!(CardRank::strtorank("A"), CardRank::Ace);
    assert_eq!(CardRank::strtorank("a"), CardRank::Ace);
    assert_eq!(CardRank::strtorank("2"), CardRank::R2);
    assert_eq!(CardRank::strtorank("8"), CardRank::R8);
    assert_eq!(CardRank::strtorank("9"), CardRank::R9);
    assert_eq!(CardRank::strtorank("10"), CardRank::R10);
    assert_eq!(CardRank::strtorank("J"), CardRank::J);
    assert_eq!(CardRank::strtorank("Q"), CardRank::Q);
    assert_eq!(CardRank::strtorank("K"), CardRank::K);
    assert_eq!(CardRank::strtorank("1"), CardRank::InvalidRank);
    assert_eq!(CardRank::strtorank("X"), CardRank::InvalidRank);
}

// --- Card creation ---
#[test]
fn test_create_card_spade_ace() {
    let c = Card::create_card(CardSuitRank::SpadeAce).unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::SpadeAce);
    assert_eq!(c.get_card_rank(), CardRank::Ace);
    assert_eq!(c.get_card_suit(), CardSuit::Spade);
}

#[test]
fn test_create_card_club8() {
    let c = Card::create_card(CardSuitRank::Club8).unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::Club8);
    assert_eq!(c.get_card_rank(), CardRank::R8);
    assert_eq!(c.get_card_suit(), CardSuit::Club);
}

#[test]
fn test_create_card_invalid() {
    assert!(Card::create_card(CardSuitRank::CardCount).is_none());
    assert!(Card::create_card(CardSuitRank::InvalidCard).is_none());
}

#[test]
fn test_create_card_all_52() {
    for i in 0..52usize {
        let csr = csr_from_i(i);
        let c = Card::create_card(csr);
        assert!(c.is_some(), "Failed to create card index {}", i);
        assert_eq!(c.unwrap().get_card_suit_rank(), csr);
    }
}

// --- Card::strtocard ---
#[test]
fn test_strtocard() {
    let c = Card::strtocard("S8").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::Spade8);
    assert_eq!(c.get_card_rank(), CardRank::R8);
    assert_eq!(c.get_card_suit(), CardSuit::Spade);

    let c = Card::strtocard("dk").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::DiamondK);

    let c = Card::strtocard("Ca").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::ClubAce);

    let c = Card::strtocard("hJ").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::HeartJ);

    let c = Card::strtocard("SQ").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::SpadeQ);
}

#[test]
fn test_strtocard_invalid() {
    assert!(Card::strtocard("SS").is_none());
    assert!(Card::strtocard("S0").is_none());
    assert!(Card::strtocard("S1").is_none());
    assert!(Card::strtocard("a2").is_none());
    assert!(Card::strtocard("").is_none());
    assert!(Card::strtocard("S").is_none());
    assert!(Card::strtocard("S10").is_none()); // length 3
}

// --- Card suit/rank extraction ---
#[test]
fn test_card_suit_rank_all_suits() {
    let cases = [
        (CardSuitRank::SpadeAce, CardSuit::Spade, CardRank::Ace),
        (CardSuitRank::Heart5, CardSuit::Heart, CardRank::R5),
        (CardSuitRank::Diamond10, CardSuit::Diamond, CardRank::R10),
        (CardSuitRank::ClubK, CardSuit::Club, CardRank::K),
    ];
    for (csr, expected_suit, expected_rank) in &cases {
        let c = Card::create_card(*csr).unwrap();
        assert_eq!(c.get_card_suit(), *expected_suit);
        assert_eq!(c.get_card_rank(), *expected_rank);
    }
}

// --- Invalid card bits ---
#[test]
fn test_invalid_card_bits() {
    let c = Card::write_card(CardSuitRank::InvalidCard);
    assert_eq!(c.get_card_suit(), CardSuit::InvalidSuit);
    assert_eq!(c.get_card_rank(), CardRank::InvalidRank);
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::InvalidCard);
}

// --- sort_card_after ---
#[test]
fn test_sort_card_after() {
    let c = Some(Card::create_card(CardSuitRank::SpadeAce).unwrap());
    assert_eq!(sort_card_after(&None, &c, &None), 1);
    assert_eq!(sort_card_after(&None, &c, &c), 0);
    assert_eq!(sort_card_after(&c, &c, &None), 1);
}

// --- sort_card_by_rank ---
#[test]
fn test_sort_card_by_rank() {
    let ace = Some(Card::create_card(CardSuitRank::SpadeAce).unwrap());
    let five = Some(Card::create_card(CardSuitRank::Spade5).unwrap());
    let king = Some(Card::create_card(CardSuitRank::SpadeK).unwrap());

    assert_eq!(sort_card_by_rank(&None, &five, &None), 1);
    assert_eq!(sort_card_by_rank(&None, &ace, &five), 1);
    assert_eq!(sort_card_by_rank(&None, &king, &five), 0);
    assert_eq!(sort_card_by_rank(&ace, &five, &king), 1);
    assert_eq!(sort_card_by_rank(&five, &ace, &king), 0);
}

// --- CardHand basic operations ---
#[test]
fn test_hand_create_empty() {
    let h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    assert_eq!(h.count_cards_in_hand(), 0);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);
}

#[test]
fn test_hand_insert_and_count() {
    let mut h = CardHand::create_hand(3, sort_card_after).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    assert_eq!(h.count_cards_in_hand(), 1);
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart5));
    assert_eq!(h.count_cards_in_hand(), 2);
    h.insert_into_hand(&Card::create_card(CardSuitRank::ClubK));
    assert_eq!(h.count_cards_in_hand(), 3);
    // Hand is full
    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond2));
    assert_eq!(h.count_cards_in_hand(), 3);
}

// Use thread_local to collect cards during iteration (since iterate_hand takes fn pointer)
thread_local! {
    static COLLECTED: RefCell<Vec<CardSuitRank>> = RefCell::new(Vec::new());
    static ITR_COUNT: RefCell<u64> = RefCell::new(0);
}

fn collect_cards(_len: u64, _pos: u64, c: &Option<Card>) -> ItrAction {
    if let Some(card) = c {
        COLLECTED.with(|v| v.borrow_mut().push(card.get_card_suit_rank()));
    }
    ItrAction::Continue
}

fn get_collected() -> Vec<CardSuitRank> {
    COLLECTED.with(|v| {
        let result = v.borrow().clone();
        v.borrow_mut().clear();
        result
    })
}

#[test]
fn test_hand_sort_by_rank() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    // Insert in same order as C test (seed 3 dealing): H9, SA, H10, C2, D6, HQ, D2
    let cards = [
        CardSuitRank::Heart9,
        CardSuitRank::SpadeAce,
        CardSuitRank::Heart10,
        CardSuitRank::Club2,
        CardSuitRank::Diamond6,
        CardSuitRank::HeartQ,
        CardSuitRank::Diamond2,
    ];
    for csr in &cards {
        h.insert_into_hand(&Card::create_card(*csr));
    }
    assert_eq!(h.count_cards_in_hand(), 7);

    // Expected order from C test: SA, D2, C2, D6, H9, H10, HQ
    let expected = [
        CardSuitRank::SpadeAce,
        CardSuitRank::Diamond2,
        CardSuitRank::Club2,
        CardSuitRank::Diamond6,
        CardSuitRank::Heart9,
        CardSuitRank::Heart10,
        CardSuitRank::HeartQ,
    ];
    get_collected();
    h.iterate_hand(collect_cards);
    assert_eq!(get_collected(), expected);
}

#[test]
fn test_hand_max_rank() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);

    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);

    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);

    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart10));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);
}

// --- remove_from_hand ---
#[test]
fn test_hand_remove() {
    // Insert in same order as C test (seed 3 dealing): H9, SA, H10, C2, D6, HQ, D2
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    for csr in [CardSuitRank::Heart9, CardSuitRank::SpadeAce, CardSuitRank::Heart10,
                CardSuitRank::Club2, CardSuitRank::Diamond6, CardSuitRank::HeartQ, CardSuitRank::Diamond2] {
        h.insert_into_hand(&Card::create_card(csr));
    }
    assert_eq!(h.count_cards_in_hand(), 7);

    // Verify initial sorted order: SA, D2, C2, D6, H9, H10, HQ
    get_collected();
    h.iterate_hand(collect_cards);
    assert_eq!(get_collected(), vec![
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Diamond6, CardSuitRank::Heart9, CardSuitRank::Heart10, CardSuitRank::HeartQ,
    ]);

    h.remove_from_hand(CardSuitRank::Diamond6);
    assert_eq!(h.count_cards_in_hand(), 6);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);

    get_collected();
    h.iterate_hand(collect_cards);
    assert_eq!(get_collected(), vec![
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Heart9, CardSuitRank::Heart10, CardSuitRank::HeartQ,
    ]);

    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 5);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    get_collected();
    h.iterate_hand(collect_cards);
    assert_eq!(get_collected(), vec![
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Heart9, CardSuitRank::Heart10,
    ]);
}

#[test]
fn test_hand_remove_nonexistent() {
    let mut h = CardHand::create_hand(3, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 1);
    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 1);
}

#[test]
fn test_hand_remove_head() {
    let mut h = CardHand::create_hand(3, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart10));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    h.remove_from_hand(CardSuitRank::SpadeAce);
    assert_eq!(h.count_cards_in_hand(), 2);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    let expected = [CardSuitRank::Heart9, CardSuitRank::Heart10];
    get_collected();
    h.iterate_hand(collect_cards);
    assert_eq!(get_collected(), expected);
}

// --- reset_hand ---
#[test]
fn test_hand_reset() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart5));
    assert_eq!(h.count_cards_in_hand(), 2);

    h.reset_hand();
    assert_eq!(h.count_cards_in_hand(), 0);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);

    h.remove_from_hand(CardSuitRank::HeartQ);
}

fn empty_hand_checker(_len: u64, _pos: u64, _c: &Option<Card>) -> ItrAction {
    panic!("Should not be called on empty hand");
}

#[test]
fn test_iterate_empty_hand() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    // This should not panic since the callback should never be called
    h.iterate_hand(empty_hand_checker);
}

// --- iterate_hand with BREAK ---
fn break_at_second(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
    ITR_COUNT.with(|v| *v.borrow_mut() += 1);
    if pos == 1 { ItrAction::Break } else { ItrAction::Continue }
}

#[test]
fn test_iterate_hand_break() {
    let mut h = CardHand::create_hand(5, sort_card_after).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade3));

    ITR_COUNT.with(|v| *v.borrow_mut() = 0);
    h.iterate_hand(break_at_second);
    let count = ITR_COUNT.with(|v| *v.borrow());
    assert_eq!(count, 2);
    assert_eq!(h.count_cards_in_hand(), 3); // no removal
}

// --- iterate_hand with RemoveAndBreak ---
fn remove_and_break_at_second(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
    if pos == 1 { ItrAction::RemoveAndBreak } else { ItrAction::Continue }
}

#[test]
fn test_iterate_hand_remove_and_break() {
    let mut h = CardHand::create_hand(5, sort_card_after).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade3));

    h.iterate_hand(remove_and_break_at_second);
    assert_eq!(h.count_cards_in_hand(), 2);
}

// --- CardDeck ---
#[test]
fn test_deck_create() {
    assert!(CardDeck::create_shuffled_deck().is_some());
}

#[test]
fn test_deck_is_card_in_deck() {
    let deck = CardDeck::create_shuffled_deck().unwrap();
    assert_eq!(deck.is_card_in_deck(CardSuitRank::HeartK), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::SpadeAce), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::ClubK), 1);
}

#[test]
fn test_deck_strip_card() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    assert_eq!(deck.is_card_in_deck(CardSuitRank::HeartK), 1);
    deck.strip_card_from_deck(CardSuitRank::HeartK);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::HeartK), 0);
    // Stripping again should be safe
    deck.strip_card_from_deck(CardSuitRank::HeartK);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::HeartK), 0);
}

#[test]
fn test_deck_deal_all() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    for _ in 0..52 {
        assert!(deck.deal_from_deck().is_some());
    }
    assert!(deck.deal_from_deck().is_none());
}

#[test]
fn test_deck_deal_strips_card() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let c = deck.deal_from_deck().unwrap();
    let csr = c.get_card_suit_rank();
    assert_ne!(csr, CardSuitRank::InvalidCard);
    assert_eq!(deck.is_card_in_deck(csr), 0);
}

#[test]
fn test_deck_strip_then_deal_all() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    deck.strip_card_from_deck(CardSuitRank::HeartK);
    deck.strip_card_from_deck(CardSuitRank::Heart9);
    for _ in 0..50 {
        assert!(deck.deal_from_deck().is_some());
    }
    assert!(deck.deal_from_deck().is_none());
}

// --- Hand with max 1 ---
#[test]
fn test_hand_max_one() {
    let mut h = CardHand::create_hand(1, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    assert_eq!(h.count_cards_in_hand(), 1);
    assert_eq!(h.get_max_of_hand(), 1);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    assert_eq!(h.count_cards_in_hand(), 1);
}

// --- Insert None into hand ---
#[test]
fn test_hand_insert_none() {
    let mut h = CardHand::create_hand(5, sort_card_after).unwrap();
    h.insert_into_hand(&None);
    assert_eq!(h.count_cards_in_hand(), 0);
}

fn csr_from_i(i: usize) -> CardSuitRank {
    let all = [
        CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::Spade3,
        CardSuitRank::Spade4, CardSuitRank::Spade5, CardSuitRank::Spade6,
        CardSuitRank::Spade7, CardSuitRank::Spade8, CardSuitRank::Spade9,
        CardSuitRank::Spade10, CardSuitRank::SpadeJ, CardSuitRank::SpadeQ,
        CardSuitRank::SpadeK, CardSuitRank::HeartAce, CardSuitRank::Heart2,
        CardSuitRank::Heart3, CardSuitRank::Heart4, CardSuitRank::Heart5,
        CardSuitRank::Heart6, CardSuitRank::Heart7, CardSuitRank::Heart8,
        CardSuitRank::Heart9, CardSuitRank::Heart10, CardSuitRank::HeartJ,
        CardSuitRank::HeartQ, CardSuitRank::HeartK, CardSuitRank::DiamondAce,
        CardSuitRank::Diamond2, CardSuitRank::Diamond3, CardSuitRank::Diamond4,
        CardSuitRank::Diamond5, CardSuitRank::Diamond6, CardSuitRank::Diamond7,
        CardSuitRank::Diamond8, CardSuitRank::Diamond9, CardSuitRank::Diamond10,
        CardSuitRank::DiamondJ, CardSuitRank::DiamondQ, CardSuitRank::DiamondK,
        CardSuitRank::ClubAce, CardSuitRank::Club2, CardSuitRank::Club3,
        CardSuitRank::Club4, CardSuitRank::Club5, CardSuitRank::Club6,
        CardSuitRank::Club7, CardSuitRank::Club8, CardSuitRank::Club9,
        CardSuitRank::Club10, CardSuitRank::ClubJ, CardSuitRank::ClubQ,
        CardSuitRank::ClubK,
    ];
    all[i]
}

fn main() {}
