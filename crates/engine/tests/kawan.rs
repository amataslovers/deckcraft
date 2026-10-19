//! KawanEditor policy (`kawan.rs`): deny and read-only reach command execution and the edit
//! chokepoint (pointer drags write through `Session::edit` directly), and the default policy
//! leaves DeckCraft as upstream.

use deckcraft_engine::kawan::{self, Policy};
use deckcraft_engine::{EngineError, Session};
use serde_json::json;

fn slides(s: &Session) -> usize {
    s.doc().map_or(0, |d| d.doc.slides.len())
}

#[test]
fn deny_refuses_execute() {
    let mut s = Session::with_new();
    kawan::set(Policy { deny: vec!["slide.new".into()], ..Policy::default() });
    assert!(matches!(s.execute("slide.new", &json!({})), Err(EngineError::Disabled(..))));
    assert_eq!(slides(&s), 1);
    kawan::set(Policy::default());
}

#[test]
fn read_only_refuses_commands_and_pointer_edits() {
    let mut s = Session::with_new();
    kawan::set(Policy { read_only: true, ..Policy::default() });
    assert!(matches!(s.execute("slide.new", &json!({})), Err(EngineError::Disabled(..))));
    assert!(s.edit(|_, _| Ok(())).is_err());
    assert_eq!(slides(&s), 1);
    // Navigation and copy are not refused for being read-only (they may still fail for their
    // own reasons, such as nothing being selected).
    for id in ["slide.next", "slide.first", "edit.copy"] {
        let r = s.execute(id, &json!({}));
        assert!(!matches!(r, Err(EngineError::Disabled(_, ref why)) if why == "read-only"), "{id}: {r:?}");
    }
    kawan::set(Policy::default());
}

#[test]
fn read_only_refuses_undo() {
    let mut s = Session::with_new();
    s.execute("slide.new", &json!({})).unwrap();
    kawan::set(Policy { read_only: true, mutating: vec!["edit.undo".into()], ..Policy::default() });
    assert!(matches!(s.execute("edit.undo", &json!({})), Err(EngineError::Disabled(..))));
    assert_eq!(slides(&s), 2);
    kawan::set(Policy::default());
}

#[test]
fn upstream_policy_changes_nothing() {
    let mut s = Session::with_new();
    kawan::set(Policy::default());
    s.execute("slide.new", &json!({})).unwrap();
    assert_eq!(slides(&s), 2);
}

#[test]
fn allow_and_deny_spare_the_queries_the_ui_reads() {
    let mut s = Session::with_new();
    // The host knows nothing of `format.state`; the ribbon reads it every frame.
    let queries = vec!["format.state".to_string()];
    kawan::set(Policy { allow: Some(vec!["slide.new".into()]), deny: vec!["format.*".into()], queries, ..Policy::default() });
    assert!(s.execute("format.state", &json!({})).is_ok());
    assert!(matches!(s.execute("format.bold", &json!({})), Err(EngineError::Disabled(..))));
    kawan::set(Policy::default());
}
