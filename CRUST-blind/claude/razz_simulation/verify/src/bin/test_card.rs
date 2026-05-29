use razz_simulation::card::card::*;
use std::cell::RefCell;

thread_local! {
    static CAPTURED: RefCell<Vec<CardSuitRank>> = RefCell::new(Vec::new());
}

fn capture_iterator(_len: u64, _pos: u64, c: &Option<Card>) -> ItrAction {
    if let Some(card) = c {
        CAPTURED.with(|v| v.borrow_mut().push(card.get_card_suit_rank()));
    }
    ItrAction::Continue
}

fn empty_hand_iterator(_len: u64, _pos: u64, _c: &Option<Card>) -> ItrAction {
    panic!("iterator should not be called on empty hand");
}

fn remove_r2_iterator(_len: u64, _pos: u64, c: &Option<Card>) -> ItrAction {
    if let Some(card) = c {
        if card.get_card_rank() == CardRank::R2 {
            return ItrAction::RemoveAndContinue;
        }
    }
    ItrAction::Continue
}

fn break_after_two(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
    if pos == 1 {
        ItrAction::Break
    } else {
        ItrAction::Continue
    }
}

fn remove_first_and_break(_len: u64, _pos: u64, _c: &Option<Card>) -> ItrAction {
    ItrAction::RemoveAndBreak
}

#[test]
fn test_card_create_spade_ace() {
    let c = Card::create_card(CardSuitRank::SpadeAce).unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::SpadeAce);
    assert_eq!(c.get_card_rank(), CardRank::Ace);
    assert_eq!(c.get_card_suit(), CardSuit::Spade);
}

#[test]
fn test_card_create_club_8() {
    let c = Card::create_card(CardSuitRank::Club8).unwrap();
    assert_eq!(c.get_card_suit_rank(), CardSuitRank::Club8);
    assert_eq!(c.get_card_rank(), CardRank::R8);
    assert_eq!(c.get_card_suit(), CardSuit::Club);
}

#[test]
fn test_card_create_invalid() {
    assert!(Card::create_card(CardSuitRank::CardCount).is_none());
    assert!(Card::create_card(CardSuitRank::InvalidCard).is_none());
}

#[test]
fn test_card_create_all_valid() {
    let all = [
        CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::Spade3, CardSuitRank::Spade4,
        CardSuitRank::Spade5, CardSuitRank::Spade6, CardSuitRank::Spade7, CardSuitRank::Spade8,
        CardSuitRank::Spade9, CardSuitRank::Spade10, CardSuitRank::SpadeJ, CardSuitRank::SpadeQ,
        CardSuitRank::SpadeK,
        CardSuitRank::HeartAce, CardSuitRank::Heart2, CardSuitRank::Heart3, CardSuitRank::Heart4,
        CardSuitRank::Heart5, CardSuitRank::Heart6, CardSuitRank::Heart7, CardSuitRank::Heart8,
        CardSuitRank::Heart9, CardSuitRank::Heart10, CardSuitRank::HeartJ, CardSuitRank::HeartQ,
        CardSuitRank::HeartK,
        CardSuitRank::DiamondAce, CardSuitRank::Diamond2, CardSuitRank::Diamond3, CardSuitRank::Diamond4,
        CardSuitRank::Diamond5, CardSuitRank::Diamond6, CardSuitRank::Diamond7, CardSuitRank::Diamond8,
        CardSuitRank::Diamond9, CardSuitRank::Diamond10, CardSuitRank::DiamondJ, CardSuitRank::DiamondQ,
        CardSuitRank::DiamondK,
        CardSuitRank::ClubAce, CardSuitRank::Club2, CardSuitRank::Club3, CardSuitRank::Club4,
        CardSuitRank::Club5, CardSuitRank::Club6, CardSuitRank::Club7, CardSuitRank::Club8,
        CardSuitRank::Club9, CardSuitRank::Club10, CardSuitRank::ClubJ, CardSuitRank::ClubQ,
        CardSuitRank::ClubK,
    ];
    for csr in all {
        let c = Card::create_card(csr).unwrap();
        assert_eq!(c.get_card_suit_rank(), csr);
    }
}

#[test]
fn test_get_card_rank_all() {
    let pairs = [
        (CardSuitRank::SpadeAce, CardRank::Ace, CardSuit::Spade),
        (CardSuitRank::Spade2, CardRank::R2, CardSuit::Spade),
        (CardSuitRank::Spade3, CardRank::R3, CardSuit::Spade),
        (CardSuitRank::Spade4, CardRank::R4, CardSuit::Spade),
        (CardSuitRank::Spade5, CardRank::R5, CardSuit::Spade),
        (CardSuitRank::Spade6, CardRank::R6, CardSuit::Spade),
        (CardSuitRank::Spade7, CardRank::R7, CardSuit::Spade),
        (CardSuitRank::Spade8, CardRank::R8, CardSuit::Spade),
        (CardSuitRank::Spade9, CardRank::R9, CardSuit::Spade),
        (CardSuitRank::Spade10, CardRank::R10, CardSuit::Spade),
        (CardSuitRank::SpadeJ, CardRank::J, CardSuit::Spade),
        (CardSuitRank::SpadeQ, CardRank::Q, CardSuit::Spade),
        (CardSuitRank::SpadeK, CardRank::K, CardSuit::Spade),
        (CardSuitRank::HeartAce, CardRank::Ace, CardSuit::Heart),
        (CardSuitRank::HeartK, CardRank::K, CardSuit::Heart),
        (CardSuitRank::DiamondAce, CardRank::Ace, CardSuit::Diamond),
        (CardSuitRank::DiamondK, CardRank::K, CardSuit::Diamond),
        (CardSuitRank::ClubAce, CardRank::Ace, CardSuit::Club),
        (CardSuitRank::ClubK, CardRank::K, CardSuit::Club),
    ];
    for (csr, rank, suit) in pairs {
        let c = Card::create_card(csr).unwrap();
        assert_eq!(c.get_card_rank(), rank);
        assert_eq!(c.get_card_suit(), suit);
        assert_eq!(c.get_card_suit_rank(), csr);
    }
}

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
    assert!(Card::strtocard("").is_none());
    assert!(Card::strtocard("S").is_none());
    assert!(Card::strtocard("S10").is_none()); // length != 2 returns None
    assert!(Card::strtocard("XK").is_none());
}

#[test]
fn test_strtorank_valid() {
    assert_eq!(CardRank::strtorank("ace"), CardRank::Ace);
    assert_eq!(CardRank::strtorank("8"), CardRank::R8);
    assert_eq!(CardRank::strtorank("K"), CardRank::K);
    assert_eq!(CardRank::strtorank("10"), CardRank::R10);
    assert_eq!(CardRank::strtorank("A"), CardRank::Ace);
    assert_eq!(CardRank::strtorank("2"), CardRank::R2);
    assert_eq!(CardRank::strtorank("9"), CardRank::R9);
    assert_eq!(CardRank::strtorank("J"), CardRank::J);
    assert_eq!(CardRank::strtorank("Q"), CardRank::Q);
    assert_eq!(CardRank::strtorank("k"), CardRank::K);
    assert_eq!(CardRank::strtorank("j"), CardRank::J);
    assert_eq!(CardRank::strtorank("q"), CardRank::Q);
}

#[test]
fn test_strtorank_invalid() {
    assert_eq!(CardRank::strtorank("1"), CardRank::InvalidRank);
    assert_eq!(CardRank::strtorank("0"), CardRank::InvalidRank);
    assert_eq!(CardRank::strtorank("Z"), CardRank::InvalidRank);
    assert_eq!(CardRank::strtorank(""), CardRank::InvalidRank);
}

#[test]
fn test_cardtostr_valid() {
    assert_eq!(CardSuitRank::Spade8.cardtostr().unwrap(), "S8");
    assert_eq!(CardSuitRank::Club10.cardtostr().unwrap(), "C10");
    assert_eq!(CardSuitRank::SpadeAce.cardtostr().unwrap(), "SA");
    assert_eq!(CardSuitRank::ClubK.cardtostr().unwrap(), "CK");
    assert_eq!(CardSuitRank::HeartJ.cardtostr().unwrap(), "HJ");
    assert_eq!(CardSuitRank::Diamond5.cardtostr().unwrap(), "D5");
}

#[test]
fn test_cardtostr_invalid() {
    assert!(CardSuitRank::CardCount.cardtostr().is_none());
    assert!(CardSuitRank::InvalidCard.cardtostr().is_none());
}

#[test]
fn test_ranktostr_valid() {
    assert_eq!(CardRank::R8.ranktostr().unwrap(), "8");
    assert_eq!(CardRank::R10.ranktostr().unwrap(), "10");
    assert_eq!(CardRank::Ace.ranktostr().unwrap(), "A");
    assert_eq!(CardRank::K.ranktostr().unwrap(), "K");
    assert_eq!(CardRank::J.ranktostr().unwrap(), "J");
    assert_eq!(CardRank::Q.ranktostr().unwrap(), "Q");
    assert_eq!(CardRank::R2.ranktostr().unwrap(), "2");
    assert_eq!(CardRank::R9.ranktostr().unwrap(), "9");
}

#[test]
fn test_ranktostr_invalid() {
    assert!(CardRank::InvalidRank.ranktostr().is_none());
    assert!(CardRank::RankCount.ranktostr().is_none());
}

#[test]
fn test_deck_create() {
    let d = CardDeck::create_shuffled_deck().unwrap();
    // All cards should be in the deck initially.
    let all = [
        CardSuitRank::SpadeAce, CardSuitRank::SpadeK,
        CardSuitRank::HeartAce, CardSuitRank::HeartK,
        CardSuitRank::DiamondAce, CardSuitRank::DiamondK,
        CardSuitRank::ClubAce, CardSuitRank::ClubK,
        CardSuitRank::Heart9,
    ];
    for csr in all {
        assert_eq!(d.is_card_in_deck(csr), 1);
    }
}

#[test]
fn test_deck_strip() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    assert_eq!(d.is_card_in_deck(CardSuitRank::HeartK), 1);
    d.strip_card_from_deck(CardSuitRank::HeartK);
    assert_eq!(d.is_card_in_deck(CardSuitRank::HeartK), 0);
    d.strip_card_from_deck(CardSuitRank::Heart9);
    assert_eq!(d.is_card_in_deck(CardSuitRank::Heart9), 0);
    // Stripping again is a no-op (card already removed)
    d.strip_card_from_deck(CardSuitRank::HeartK);
    assert_eq!(d.is_card_in_deck(CardSuitRank::HeartK), 0);
}

#[test]
fn test_deck_deal_full() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..52 {
        let c = d.deal_from_deck();
        assert!(c.is_some());
        let csr = c.unwrap().get_card_suit_rank();
        assert!(!seen.contains(&format!("{:?}", csr)));
        seen.insert(format!("{:?}", csr));
        // Card no longer in deck.
        assert_eq!(d.is_card_in_deck(csr), 0);
    }
    // Deck is empty now.
    assert!(d.deal_from_deck().is_none());
}

#[test]
fn test_deck_deal_after_strip() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    d.strip_card_from_deck(CardSuitRank::HeartK);
    d.strip_card_from_deck(CardSuitRank::Heart9);
    // Should be able to deal 50 cards now.
    for _ in 0..50 {
        let c = d.deal_from_deck();
        assert!(c.is_some());
        let csr = c.unwrap().get_card_suit_rank();
        assert_ne!(csr, CardSuitRank::HeartK);
        assert_ne!(csr, CardSuitRank::Heart9);
    }
    assert!(d.deal_from_deck().is_none());
}

#[test]
fn test_hand_empty() {
    let h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    assert_eq!(h.count_cards_in_hand(), 0);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);
}

#[test]
fn test_hand_insert_sorted_by_rank() {
    let order = [
        CardSuitRank::Heart9, CardSuitRank::SpadeAce, CardSuitRank::Heart10,
        CardSuitRank::Club2, CardSuitRank::Diamond6, CardSuitRank::HeartQ,
        CardSuitRank::Diamond2,
    ];
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();

    h.insert_into_hand(&Card::create_card(order[0]));
    assert_eq!(h.count_cards_in_hand(), 1);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);

    h.insert_into_hand(&Card::create_card(order[1]));
    assert_eq!(h.count_cards_in_hand(), 2);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);

    h.insert_into_hand(&Card::create_card(order[2]));
    assert_eq!(h.count_cards_in_hand(), 3);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    h.insert_into_hand(&Card::create_card(order[3]));
    assert_eq!(h.count_cards_in_hand(), 4);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    h.insert_into_hand(&Card::create_card(order[4]));
    assert_eq!(h.count_cards_in_hand(), 5);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);

    h.insert_into_hand(&Card::create_card(order[5]));
    assert_eq!(h.count_cards_in_hand(), 6);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);

    h.insert_into_hand(&Card::create_card(order[6]));
    assert_eq!(h.count_cards_in_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);

    // Hand is full, next insert should be ignored.
    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond9));
    assert_eq!(h.count_cards_in_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);

    let expected = [
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Diamond6, CardSuitRank::Heart9, CardSuitRank::Heart10,
        CardSuitRank::HeartQ,
    ];
    CAPTURED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(capture_iterator);
    CAPTURED.with(|v| {
        let captured = v.borrow().clone();
        assert_eq!(captured, expected);
    });

    // remove_from_hand DIAMOND_6
    h.remove_from_hand(CardSuitRank::Diamond6);
    assert_eq!(h.count_cards_in_hand(), 6);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::Q);
    let expected2 = [
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Heart9, CardSuitRank::Heart10, CardSuitRank::HeartQ,
    ];
    CAPTURED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(capture_iterator);
    CAPTURED.with(|v| {
        let captured = v.borrow().clone();
        assert_eq!(captured, expected2);
    });

    // remove_from_hand HEART_Q
    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 5);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);
    let expected3 = [
        CardSuitRank::SpadeAce, CardSuitRank::Diamond2, CardSuitRank::Club2,
        CardSuitRank::Heart9, CardSuitRank::Heart10,
    ];
    CAPTURED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(capture_iterator);
    CAPTURED.with(|v| {
        let captured = v.borrow().clone();
        assert_eq!(captured, expected3);
    });

    // reset_hand
    h.reset_hand();
    assert_eq!(h.count_cards_in_hand(), 0);
    assert_eq!(h.get_max_of_hand(), 7);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::InvalidRank);
    h.remove_from_hand(CardSuitRank::HeartQ);
    h.iterate_hand(empty_hand_iterator); // should not call iterator
}

#[test]
fn test_hand_insert_after() {
    // sort_card_after just appends in insertion order.
    let order = [
        CardSuitRank::Heart9, CardSuitRank::SpadeAce, CardSuitRank::Heart10,
        CardSuitRank::Club2,
    ];
    let mut h = CardHand::create_hand(4, sort_card_after).unwrap();
    for csr in order {
        h.insert_into_hand(&Card::create_card(csr));
    }
    CAPTURED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(capture_iterator);
    CAPTURED.with(|v| {
        let captured = v.borrow().clone();
        assert_eq!(captured, order);
    });
}

#[test]
fn test_hand_max_size_1() {
    let mut h = CardHand::create_hand(1, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart9));
    assert_eq!(h.count_cards_in_hand(), 1);
    assert_eq!(h.get_max_of_hand(), 1);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R9);
    // Second insert ignored.
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    assert_eq!(h.count_cards_in_hand(), 1);
    // remove non-existent
    h.remove_from_hand(CardSuitRank::HeartQ);
    assert_eq!(h.count_cards_in_hand(), 1);
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
    // Remove the head (SPADE_ACE)
    h.remove_from_hand(CardSuitRank::SpadeAce);
    assert_eq!(h.get_max_rank_of_hand(), CardRank::R10);
    assert_eq!(h.count_cards_in_hand(), 2);
    let expected = [CardSuitRank::Heart9, CardSuitRank::Heart10];
    CAPTURED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(capture_iterator);
    CAPTURED.with(|v| {
        let captured = v.borrow().clone();
        assert_eq!(captured, expected);
    });
}

#[test]
fn test_hand_remove_duplicate_rank_keeps_other_suits() {
    // Make sure remove_from_hand only removes the specified suit+rank.
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Diamond2));
    // Remove only Heart2
    h.remove_from_hand(CardSuitRank::Heart2);
    assert_eq!(h.count_cards_in_hand(), 2);
    CAPTURED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(capture_iterator);
    CAPTURED.with(|v| {
        let captured = v.borrow().clone();
        // sort_card_by_rank: same rank, tie-broken by insertion order, but Heart2 was first so
        // after Heart2 removed, only [Spade2, Diamond2] (in insertion order).
        assert_eq!(captured.len(), 2);
        for csr in &captured {
            assert_eq!(Card::create_card(*csr).unwrap().get_card_rank(), CardRank::R2);
        }
    });
}

#[test]
fn test_iterate_hand_remove_and_continue() {
    let mut h = CardHand::create_hand(5, sort_card_after).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart3));
    h.iterate_hand(remove_r2_iterator);
    assert_eq!(h.count_cards_in_hand(), 2);
    let expected = [CardSuitRank::SpadeAce, CardSuitRank::Heart3];
    CAPTURED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(capture_iterator);
    CAPTURED.with(|v| {
        let captured = v.borrow().clone();
        assert_eq!(captured, expected);
    });
}

#[test]
fn test_iterate_hand_break() {
    let mut h = CardHand::create_hand(5, sort_card_after).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade3));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade4));
    // Should not modify hand.
    h.iterate_hand(break_after_two);
    assert_eq!(h.count_cards_in_hand(), 4);
}

#[test]
fn test_iterate_hand_remove_and_break() {
    let mut h = CardHand::create_hand(5, sort_card_after).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeAce));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade2));
    h.insert_into_hand(&Card::create_card(CardSuitRank::Spade3));
    h.iterate_hand(remove_first_and_break);
    assert_eq!(h.count_cards_in_hand(), 2);
    let expected = [CardSuitRank::Spade2, CardSuitRank::Spade3];
    CAPTURED.with(|v| v.borrow_mut().clear());
    h.iterate_hand(capture_iterator);
    CAPTURED.with(|v| {
        let captured = v.borrow().clone();
        assert_eq!(captured, expected);
    });
}

#[test]
fn test_sort_card_after_predicate() {
    // sort_card_after returns 1 only when after is None, else 0.
    let c = Card::create_card(CardSuitRank::SpadeAce);
    let other = Card::create_card(CardSuitRank::HeartK);
    assert_eq!(sort_card_after(&None, &c, &None), 1);
    assert_eq!(sort_card_after(&other, &c, &None), 1);
    assert_eq!(sort_card_after(&None, &c, &other), 0);
    assert_eq!(sort_card_after(&other, &c, &other), 0);
}

#[test]
fn test_sort_card_by_rank_predicate() {
    // sort_card_by_rank: insert when after==NULL OR
    //   ((before==NULL || r > before.rank) && r <= after.rank)
    let new_c = Card::create_card(CardSuitRank::Heart6);
    let lower = Card::create_card(CardSuitRank::SpadeAce); // rank Ace (0)
    let higher = Card::create_card(CardSuitRank::Heart9); // rank R9 (8)

    // after is None -> always 1
    assert_eq!(sort_card_by_rank(&None, &new_c, &None), 1);
    assert_eq!(sort_card_by_rank(&lower, &new_c, &None), 1);

    // before None, after rank > new rank -> insert at front
    assert_eq!(sort_card_by_rank(&None, &new_c, &higher), 1);

    // before rank < new rank, after rank > new rank -> insert in middle
    assert_eq!(sort_card_by_rank(&lower, &new_c, &higher), 1);

    // before rank > new rank: don't insert
    assert_eq!(sort_card_by_rank(&higher, &new_c, &higher), 0);

    // after rank < new rank: don't insert (e.g. before=None, after=lower)
    assert_eq!(sort_card_by_rank(&None, &new_c, &lower), 0);
}

#[test]
fn test_get_max_rank_after_inserts() {
    let mut h = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    h.insert_into_hand(&Card::create_card(CardSuitRank::SpadeK));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::K);
    h.insert_into_hand(&Card::create_card(CardSuitRank::Heart2));
    assert_eq!(h.get_max_rank_of_hand(), CardRank::K);
}

fn main() {}
