#![allow(dead_code, unused_imports)]

use cfsm::cfsm::{CfsmCtx, CFSM_VER_MAJOR, CFSM_VER_MINOR, CFSM_VER_PATCH};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

// Serialize tests because they share global counter state.
static TEST_LOCK: Mutex<()> = Mutex::new(());

// Using static atomics so that we can use plain `fn` pointers (no closures).
static A_ENTER: AtomicI32 = AtomicI32::new(0);
static A_LEAVE: AtomicI32 = AtomicI32::new(0);
static A_EVENT: AtomicI32 = AtomicI32::new(0);
static A_PROCESS: AtomicI32 = AtomicI32::new(0);
static A_LAST_EVENT_ID: AtomicI32 = AtomicI32::new(0);

static B_ENTER: AtomicI32 = AtomicI32::new(0);
static B_LEAVE: AtomicI32 = AtomicI32::new(0);
static B_EVENT: AtomicI32 = AtomicI32::new(0);
static B_PROCESS: AtomicI32 = AtomicI32::new(0);

fn reset_counters() {
    A_ENTER.store(0, Ordering::SeqCst);
    A_LEAVE.store(0, Ordering::SeqCst);
    A_EVENT.store(0, Ordering::SeqCst);
    A_PROCESS.store(0, Ordering::SeqCst);
    A_LAST_EVENT_ID.store(0, Ordering::SeqCst);
    B_ENTER.store(0, Ordering::SeqCst);
    B_LEAVE.store(0, Ordering::SeqCst);
    B_EVENT.store(0, Ordering::SeqCst);
    B_PROCESS.store(0, Ordering::SeqCst);
}

fn state_only_on_enter(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    // no-op
}

fn state_a_on_enter(fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    if let Some(b) = fsm {
        b.on_event = Some(state_a_on_event);
        b.on_leave = Some(state_a_on_leave);
        b.on_process = Some(state_a_on_process);
    }
    A_ENTER.fetch_add(1, Ordering::SeqCst);
}

fn state_a_on_event(_fsm: Option<Box<&mut CfsmCtx<u8>>>, event_id: i32) {
    A_EVENT.fetch_add(1, Ordering::SeqCst);
    A_LAST_EVENT_ID.store(event_id, Ordering::SeqCst);
}

fn state_a_on_process(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    A_PROCESS.fetch_add(1, Ordering::SeqCst);
}

fn state_a_on_leave(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    A_LEAVE.fetch_add(1, Ordering::SeqCst);
}

fn state_b_on_enter(fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    if let Some(b) = fsm {
        b.on_event = Some(state_b_on_event);
        b.on_leave = Some(state_b_on_leave);
        b.on_process = Some(state_b_on_process);
    }
    B_ENTER.fetch_add(1, Ordering::SeqCst);
}

fn state_b_on_event(_fsm: Option<Box<&mut CfsmCtx<u8>>>, _event_id: i32) {
    B_EVENT.fetch_add(1, Ordering::SeqCst);
}

fn state_b_on_process(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    B_PROCESS.fetch_add(1, Ordering::SeqCst);
}

fn state_b_on_leave(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    B_LEAVE.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn test_cfsm_version() {
    // Versions per c_fsm.h: 0.3.0
    assert_eq!(CFSM_VER_MAJOR, 0);
    assert_eq!(CFSM_VER_MINOR, 3);
    assert_eq!(CFSM_VER_PATCH, 0);
}

#[test]
fn test_cfsm_new_initializes_to_none() {
    let fsm: CfsmCtx<u8> = CfsmCtx::new();
    assert!(fsm.ctx_ptr.is_none());
    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
}

#[test]
fn test_cfsm_init_should_clear_handler() {
    let mut fsm: CfsmCtx<u8> = CfsmCtx::new();
    // Pre-populate handlers to simulate "corrupt" content.
    fsm.on_event = Some(state_a_on_event);
    fsm.on_process = Some(state_a_on_process);
    fsm.on_leave = Some(state_a_on_leave);

    let dummy: Box<u8> = Box::new(42u8);
    fsm.cfsm_init(Some(dummy));

    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());

    assert!(fsm.ctx_ptr.is_some());
    assert_eq!(*fsm.ctx_ptr.as_ref().unwrap().as_ref(), 42u8);
}

#[test]
fn test_cfsm_init_is_safe_to_use() {
    let mut fsm: CfsmCtx<u8> = CfsmCtx::new();
    fsm.cfsm_init(None);

    // should not crash
    fsm.cfsm_process();
    fsm.cfsm_event(0x12345678);

    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
    assert!(fsm.ctx_ptr.is_none());
}

#[test]
fn test_cfsm_transition_should_set_enter_handler_only() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_counters();
    let mut fsm: CfsmCtx<u8> = CfsmCtx::new();
    fsm.cfsm_init(None);

    fsm.cfsm_transition(Some(state_only_on_enter));

    // state_only_on_enter does not set any handler.
    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
}

#[test]
fn test_cfs_process() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_counters();
    let mut fsm: CfsmCtx<u8> = CfsmCtx::new();
    fsm.cfsm_init(None);

    fsm.cfsm_transition(Some(state_a_on_enter));

    assert_eq!(A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(A_EVENT.load(Ordering::SeqCst), 0);
    assert_eq!(A_LAST_EVENT_ID.load(Ordering::SeqCst), 0);

    for _ in 0..10 {
        fsm.cfsm_process();
        // process only increments processCalls; lastEventId remains untouched.
        assert_eq!(A_LAST_EVENT_ID.load(Ordering::SeqCst), 0);
    }
    assert_eq!(A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(A_PROCESS.load(Ordering::SeqCst), 10);
    assert_eq!(A_EVENT.load(Ordering::SeqCst), 0);
}

#[test]
fn test_cfs_signal_event() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_counters();
    let mut fsm: CfsmCtx<u8> = CfsmCtx::new();
    fsm.cfsm_init(None);

    fsm.cfsm_transition(Some(state_a_on_enter));

    assert_eq!(A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(A_EVENT.load(Ordering::SeqCst), 0);

    for i in 0..10 {
        fsm.cfsm_event(i);
    }
    assert_eq!(A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(A_EVENT.load(Ordering::SeqCst), 10);
    // last event id should be 9
    assert_eq!(A_LAST_EVENT_ID.load(Ordering::SeqCst), 9);
}

#[test]
fn test_cfs_transition_a_b_a() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_counters();
    let mut fsm: CfsmCtx<u8> = CfsmCtx::new();
    fsm.cfsm_init(None);

    fsm.cfsm_transition(Some(state_a_on_enter));

    assert!(fsm.on_event.is_some());
    assert!(fsm.on_process.is_some());
    assert!(fsm.on_leave.is_some());

    assert_eq!(A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(A_EVENT.load(Ordering::SeqCst), 0);

    assert_eq!(B_ENTER.load(Ordering::SeqCst), 0);
    assert_eq!(B_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(B_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(B_EVENT.load(Ordering::SeqCst), 0);

    fsm.cfsm_transition(Some(state_b_on_enter));

    assert!(fsm.on_event.is_some());
    assert!(fsm.on_process.is_some());
    assert!(fsm.on_leave.is_some());

    assert_eq!(A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 1);
    assert_eq!(A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(A_EVENT.load(Ordering::SeqCst), 0);

    assert_eq!(B_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(B_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(B_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(B_EVENT.load(Ordering::SeqCst), 0);

    fsm.cfsm_transition(Some(state_a_on_enter));

    assert!(fsm.on_event.is_some());
    assert!(fsm.on_process.is_some());
    assert!(fsm.on_leave.is_some());

    assert_eq!(A_ENTER.load(Ordering::SeqCst), 2);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 1);
    assert_eq!(A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(A_EVENT.load(Ordering::SeqCst), 0);

    assert_eq!(B_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(B_LEAVE.load(Ordering::SeqCst), 1);
    assert_eq!(B_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(B_EVENT.load(Ordering::SeqCst), 0);
}

#[test]
fn test_cfsm_transition_to_none_invokes_leave_and_clears() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_counters();
    let mut fsm: CfsmCtx<u8> = CfsmCtx::new();
    fsm.cfsm_init(None);

    fsm.cfsm_transition(Some(state_a_on_enter));
    assert_eq!(A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 0);

    // Transition to None should invoke leave and clear all handlers.
    fsm.cfsm_transition(None);
    assert_eq!(A_LEAVE.load(Ordering::SeqCst), 1);
    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());

    // After clearing, process and event should be no-ops (no panic).
    fsm.cfsm_process();
    fsm.cfsm_event(99);
    assert_eq!(A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(A_EVENT.load(Ordering::SeqCst), 0);
}

fn main() {}
