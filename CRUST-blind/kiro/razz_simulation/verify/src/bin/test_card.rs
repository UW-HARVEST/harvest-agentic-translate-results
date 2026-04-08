use razz_simulation::card::card::*;
use std::cell::RefCell;

thread_local! {
    static COLLECTED: RefCell<Vec<CardSuitRank>> = RefCell::new(Vec::new());
    static VISITED_COUNT: RefCell<u64> = RefCell::new(0);
}

fn collector(_len: u64, _pos: u64, c: &Option<Card>) -> ItrAction {
    if let Some(card) = c {
        COLLECTED.with(|v| v.borrow_mut().push(card.get_card_suit_rank()));
    }
    ItrAction::Continue
}

fn collect_and_clear(h: &mut CardHand) -> Vec<CardSuitRank> {
    COLLECTED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(collector);
    COLLECTED.with(|v| v.borrow().clone())
}

// --- Enum ordering tests ---

#[test]
fn test_enum_ordering() {
    assert!(CardSuitRank::SpadeAce < CardSuitRank::SpadeK);
    assert!(CardSuitRank::HeartK < CardSuitRank::ClubK);
    assert!(CardRank::Ace < CardRank::R3);
}

#[test]
fn test_enum_values() {
    assert_eq!(CardSuitRank::SpadeAce as usize, 0);
    assert_eq!(CardSuitRank::HeartAce as usize, 13);
    assert_eq!(CardSuitRank::DiamondAce as usize, 26);
    assert_eq!(CardSuitRank::ClubAce as usize, 39);
    assert_eq!(CardSuitRank::CardCount as usize, 52);
    assert_eq!(CardSuitRank::InvalidCard as usize, 53);
    assert_eq!(CardRank::Ace as usize, 0);
    assert_eq!(CardRank::R2 as usize, 1);
    assert_eq!(CardRank::R10 as usize, 9);
    assert_eq!(CardRank::J as usize, 10);
    assert_eq!(CardRank::Q as usize, 11);
    assert_eq!(CardRank::K as usize, 12);
    assert_eq!(CardRank::InvalidRank as usize, 14);
    assert_eq!(CardSuit::Spade as usize, 0);
    assert_eq!(CardSuit::Heart as usize, 1);
    assert_eq!(CardSuit::Diamond as usize, 2);
    assert_eq!(CardSuit::Club as usize, 3);
    assert_eq!(CardSuit::InvalidSuit as usize, 5);
}

// --- Card creation and field extraction ---

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
fn test_create_card_heart_5() {
    let c = Card::create_card(CardSuitRank::Heart5).unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::Heart5);
    assert_eq!(c.get_card_rank(), CardRank::R5);
    assert_eq!(c.get_card_suit(), CardSuit::Heart);
}

#[test]
fn test_create_card_diamond_k() {
    let c = Card::create_card(CardSuitRank::DiamondK).unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::DiamondK);
    assert_eq!(c.get_card_rank(), CardRank::K);
    assert_eq!(c.get_card_suit(), CardSuit::Diamond);
}

#[test]
fn test_create_card_invalid() {
    assert!(Card::create_card(CardSuitRank::CardCount).is_none());
    assert!(Card::create_card(CardSuitRank::InvalidCard).is_none());
}

// --- All 52 cards round-trip ---

#[test]
fn test_all_52_cards_round_trip() {
    let all_csrs = [
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
    for &csr in &all_csrs {
        let c = Card::create_card(csr).unwrap();
        assert_eq!(c.get_card_suit_rank(), csr);
        let idx = csr as usize;
        let expected_suit = match idx / 13 {
            0 => CardSuit::Spade, 1 => CardSuit::Heart,
            2 => CardSuit::Diamond, 3 => CardSuit::Club, _ => unreachable!(),
        };
        assert_eq!(c.get_card_suit(), expected_suit);
        assert_eq!(c.get_card_rank() as usize, idx % 13);
    }
}

// --- strtocard ---

#[test]
fn test_strtocard_valid() {
    let c = Card::strtocard("S8").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::Spade8);
    assert_eq!(c.get_card_rank(), CardRank::R8);
    assert_eq!(c.get_card_suit(), CardSuit::Spade);

    let c = Card::strtocard("dk").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::DiamondK);
    assert_eq!(c.get_card_rank(), CardRank::K);
    assert_eq!(c.get_card_suit(), CardSuit::Diamond);

    let c = Card::strtocard("Ca").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::ClubAce);
    assert_eq!(c.get_card_rank(), CardRank::Ace);
    assert_eq!(c.get_card_suit(), CardSuit::Club);

    let c = Card::strtocard("hJ").unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::HeartJ);
    assert_eq!(c.get_card_rank(), CardRank::J);
    assert_eq!(c.get_card_suit(), CardSuit::Heart);

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

// --- strtorank ---

#[test]
fn test_strtorank() {
    assert_eq!(CardRank::strtorank("ace"), CardRank::Ace);
    assert_eq!(CardRank::strtorank("8"), CardRank::R8);
    assert_eq!(CardRank::strtorank("K"), CardRank::K);
    assert_eq!(CardRank::strtorank("10"), CardRank::R10);
    assert_eq!(CardRank::strtorank("1"), CardRank::InvalidRank);
    assert_eq!(CardRank::strtorank("2"), CardRank::R2);
    assert_eq!(CardRank::strtorank("9"), CardRank::R9);
    assert_eq!(CardRank::strtorank("J"), CardRank::J);
    assert_eq!(CardRank::strtorank("Q"), CardRank::Q);
}

// --- cardtostr ---

#[test]
fn test_cardtostr() {
    assert_eq!(CardSuitRank::Spade8.cardtostr(), Some("S8".to_string()));
    assert_eq!(CardSuitRank::Club10.cardtostr(), Some("C10".to_string()));
    assert_eq!(CardSuitRank::SpadeAce.cardtostr(), Some("SA".to_string()));
    assert_eq!(CardSuitRank::ClubK.cardtostr(), Some("CK".to_string()));
    assert_eq!(CardSuitRank::CardCount.cardtostr(), None);
    assert_eq!(CardSuitRank::InvalidCard.cardtostr(), None);
}

// --- ranktostr ---

#[test]
fn test_ranktostr() {
    assert_eq!(CardRank::R8.ranktostr(), Some("8".to_string()));
    assert_eq!(CardRank::R10.ranktostr(), Some("10".to_string()));
    assert_eq!(CardRank::Ace.ranktostr(), Some("A".to_string()));
    assert_eq!(CardRank::K.ranktostr(), Some("K".to_string()));
    assert_eq!(CardRank::InvalidRank.ranktostr(), None);
    assert_eq!(CardRank::RankCount.ranktostr(), None);
}

// --- sort_card_by_rank ---

#[test]
fn test_sort_card_by_rank() {
    let c1 = Card::create_card(CardSuitRank::Spade3);
    let c2 = Card::create_card(CardSuitRank::Heart5);
    let c3 = Card::create_card(CardSuitRank::Diamond8);
    assert_eq!(sort_card_by_rank(&None, &c2, &c3), 1);
    assert_eq!(sort_card_by_rank(&c1, &c2, &c3), 1);
    assert_eq!(sort_card_by_rank(&c1, &c2, &None), 1);
    assert_eq!(sort_card_by_rank(&c2, &c1, &c3), 0);
}

// --- sort_card_after ---

#[test]
fn test_sort_card_after() {
    let c1 = Card::create_card(CardSuitRank::Spade3);
    let c2 = Card::create_card(CardSuitRank::Heart5);
    assert_eq!(sort_card_after(&None, &c1, &c2), 0);
    assert_eq!(sort_card_after(&c1, &c2, &None), 1);
}

// --- write_card ---

#[test]
fn test_write_card() {
    let c = Card::write_card(CardSuitRank::HeartQ);
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::HeartQ);
    assert_eq!(c.get_card_rank(), CardRank::Q);
    assert_eq!(c.get_card_suit(), CardSuit::Heart);
}

// --- Deck ---

#[test]
fn test_deck_creation_and_strip() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    assert_eq!(d.is_card_in_deck(CardSuitRank::HeartK), 1);
    d.strip_card_from_deck(CardSuitRank::HeartK);
    assert_eq!(d.is_card_in_deck(CardSuitRank::HeartK), 0);
    d.strip_card_from_deck(CardSuitRank::HeartK);
    assert_eq!(d.is_card_in_deck(CardSuitRank::HeartK), 0);
}

#[test]
fn test_deck_deal_all() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    for _ in 0..52 {
        assert!(d.deal_from_deck().is_some());
    }
    assert!(d.deal_from_deck().is_none());
}

#[test]
fn test_deck_strip_then_deal() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    d.strip_card_from_deck(CardSuitRank::Heart9);
    d.strip_card_from_deck(CardSuitRank::HeartK);
    for _ in 0..50 {
        let c = d.deal_from_deck().unwrap();
        assert_ne!(c.get_card_suit_rank(), CardSuitRank::Heart9);
        assert_ne!(c.get_card_suit_rank(), CardSuitRank::HeartK);
    }
    assert!(d.deal_from_deck().is_none());
}

// --- Hand ---

#[test]
fn test_hand_empty() {
    let h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    assert_eq!(h.count_cards_in_hand(), 0);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);
}

#[test]
fn test_hand_insert_and_sort_by_rank() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    // Insert cards in seed3 dealing order: H9, SA, H10, C2, D6, HQ, D2
    let cards_to_insert = [
        CardSuitRank::Heart9, CardSuitRank::SpadeAce, CardSuitRank::Heart10,
        CardSuitRank::Club2, CardSuitRank::Diamond6, CardSuitRank::HeartQ,
        CardSuitRank::Diamond2,
    ];
    for &csr in &cards_to_insert {
        h.insert_into_hand(&Card::create_card(csr));
    }
    assert_eq!(h.count_cards_in_hand(), 7);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);

    let sorted = collect_and_clear(&mut h);
    assert_eq!(sorted, vec![
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Diamond6, CardSuitRank::Heart9, CardSuitRank::Heart10,
        CardSuitRank::HeartQ,
    ]);
}

#[test]
fn test_hand_incremental_insert() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    assert_eq!(h.count_cards_in_hand(), 1);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);

    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    assert_eq!(h.count_cards_in_hand(), 2);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);

    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart10));
    assert_eq!(h.count_cards_in_hand(), 3);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    h.insert_into_hand(&Card::create_card(CardSuitRank::Club2));
    assert_eq!(h.count_cards_in_hand(), 4);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond6));
    assert_eq!(h.count_cards_in_hand(), 5);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    h.insert_into_hand(&Card::create_card(CardSuitRank::HeartQ));
    assert_eq!(h.count_cards_in_hand(), 6);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);

    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond2));
    assert_eq!(h.count_cards_in_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);
}

#[test]
fn test_hand_full_reject() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    let cards = [
        CardSuitRank::Heart9, CardSuitRank::SpadeAce, CardSuitRank::Heart10,
        CardSuitRank::Club2, CardSuitRank::Diamond6, CardSuitRank::HeartQ,
        CardSuitRank::Diamond2,
    ];
    for &csr in &cards {
        h.insert_into_hand(&Card::create_card(csr));
    }
    assert_eq!(h.count_cards_in_hand(), 7);
    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond9));
    assert_eq!(h.count_cards_in_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);
}

#[test]
fn test_hand_remove() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    let cards = [
        CardSuitRank::Heart9, CardSuitRank::SpadeAce, CardSuitRank::Heart10,
        CardSuitRank::Club2, CardSuitRank::Diamond6, CardSuitRank::HeartQ,
        CardSuitRank::Diamond2,
    ];
    for &csr in &cards {
        h.insert_into_hand(&Card::create_card(csr));
    }

    h.remove_from_hand(CardSuitRank::Diamond6);
    assert_eq!(h.count_cards_in_hand(), 6);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);
    let sorted = collect_and_clear(&mut h);
    assert_eq!(sorted, vec![
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Heart9, CardSuitRank::Heart10, CardSuitRank::HeartQ,
    ]);

    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 5);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);
    let sorted2 = collect_and_clear(&mut h);
    assert_eq!(sorted2, vec![
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Heart9, CardSuitRank::Heart10,
    ]);
}

#[test]
fn test_hand_reset() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.reset_hand();
    assert_eq!(h.count_cards_in_hand(), 0);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);
    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 0);
}

fn empty_hand_checker(_len: u64, _pos: u64, _c: &Option<Card>) -> ItrAction {
    panic!("should not be called on empty hand");
}

#[test]
fn test_hand_iterate_empty() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.iterate_hand(empty_hand_checker);
    // If we get here, the callback was never called (good)
}

#[test]
fn test_hand_max_1() {
    let mut h = CardHand::create_hand(1, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    assert_eq!(h.count_cards_in_hand(), 1);
    assert_eq!(h.get_max_of_hand(), 1);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);
    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 1);
}

#[test]
fn test_hand_head_removal() {
    let mut h = CardHand::create_hand(3, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart10));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    h.remove_from_hand(CardSuitRank::SpadeAce);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);
    assert_eq!(h.count_cards_in_hand(), 2);

    let sorted = collect_and_clear(&mut h);
    assert_eq!(sorted, vec![CardSuitRank::Heart9, CardSuitRank::Heart10]);
}

#[test]
fn test_hand_sort_card_after() {
    let mut h = CardHand::create_hand(3, sort_card_after).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::DiamondK));
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart5));

    let sorted = collect_and_clear(&mut h);
    assert_eq!(sorted, vec![
        CardSuitRank::DiamondK, CardSuitRank::SpadeAce, CardSuitRank::Heart5,
    ]);
}

fn remove_from_pos3(len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
    if pos >= 3 { ItrAction::RemoveAndContinue } else { ItrAction::Continue }
}

#[test]
fn test_iterate_hand_remove_and_continue() {
    let mut h = CardHand::create_hand(5, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade3));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade4));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade5));
    assert_eq!(h.count_cards_in_hand(), 5);

    h.iterate_hand(remove_from_pos3);
    assert_eq!(h.count_cards_in_hand(), 3);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R3);
}

fn remove_first_and_break(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
    if pos == 0 { ItrAction::RemoveAndBreak } else { ItrAction::Continue }
}

#[test]
fn test_iterate_hand_remove_and_break() {
    let mut h = CardHand::create_hand(5, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade3));

    h.iterate_hand(remove_first_and_break);
    assert_eq!(h.count_cards_in_hand(), 2);

    let sorted = collect_and_clear(&mut h);
    assert_eq!(sorted, vec![CardSuitRank::Spade2, CardSuitRank::Spade3]);
}

fn break_at_pos1(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
    if pos == 1 { ItrAction::Break } else { ItrAction::Continue }
}

#[test]
fn test_iterate_hand_break() {
    let mut h = CardHand::create_hand(5, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade3));

    h.iterate_hand(break_at_pos1);
    // Break doesn't remove, all 3 still there
    assert_eq!(h.count_cards_in_hand(), 3);
}

#[test]
fn test_insert_null_card() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&None);
    assert_eq!(h.count_cards_in_hand(), 0);
}

fn main() {}
