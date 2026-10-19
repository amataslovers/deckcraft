//! KawanEditor policy in the DeckCraft UI: hidden tabs are not drawn, a hidden active tab falls
//! back to a visible one, and denied buttons disappear from the ribbon.

use deckcraft_engine::Session;
use deckcraft_engine::kawan::{self, Policy};
use deckcraft_ui_egui::{Services, SlideApp};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

fn harness(tab: &str) -> Harness<'static, SlideApp> {
    let mut app = SlideApp::new(Session::with_new(), Services::default());
    app.ui.tab = tab.into();
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(3);
    h
}

#[test]
fn hidden_tab_is_not_drawn() {
    kawan::set(Policy::default());
    assert!(harness("Home").query_by_label("Review").is_some(), "the tab strip shows Review");
    kawan::set(Policy { hidden_tabs: vec!["Review".into()], ..Policy::default() });
    assert!(harness("Home").query_by_label("Review").is_none());
    kawan::set(Policy::default());
}

#[test]
fn hidden_active_tab_falls_back() {
    kawan::set(Policy { hidden_tabs: vec!["Home".into()], ..Policy::default() });
    let h = harness("Home");
    assert_ne!(h.state().ui.tab, "Home");
    kawan::set(Policy::default());
}

#[test]
fn denied_button_is_not_drawn() {
    kawan::set(Policy::default());
    assert!(harness("Insert").query_by_label("Text Box").is_some(), "Insert shows Text Box");
    kawan::set(Policy { deny: vec!["insert.textBox".into()], ..Policy::default() });
    assert!(harness("Insert").query_by_label("Text Box").is_none());
    kawan::set(Policy::default());
}
