//! KawanEditor policy in the DeckCraft UI: hidden tabs are not drawn, a hidden active tab falls
//! back to a visible one, and denied buttons disappear from the ribbon. KawanEditor branding:
//! no ArtCraft mark, KawanEditor's (or the host's) logo in the title bar.

use deckcraft_engine::Session;
use deckcraft_engine::kawan::{self, Policy};
use deckcraft_ui_egui::{Services, SlideApp};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use serde_json::json;

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
    assert!(harness("Home").query_all_by_label("Review").next().is_some(), "the tab strip shows Review");
    kawan::set(Policy { hidden_tabs: vec!["Review".into()], ..Policy::default() });
    assert!(harness("Home").query_all_by_label("Review").next().is_none());
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
    assert!(harness("Insert").query_all_by_label("Text Box").next().is_some(), "Insert shows Text Box");
    kawan::set(Policy { deny: vec!["insert.textBox".into()], ..Policy::default() });
    assert!(harness("Insert").query_all_by_label("Text Box").next().is_none());
    kawan::set(Policy::default());
}

/// The topmost node labelled `label` (`get_all`: About, Home and the title bar can all carry one).
fn topmost<'a>(h: &'a Harness<'static, SlideApp>, label: &'a str) -> egui_kittest::Node<'a> {
    h.get_all_by_label(label).min_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).expect("get_all_by_label found one")
}

/// No ArtCraft mark and no community link; the attribution sentence of the About line is allowed.
/// A `Role::Label` node (`ui.label`) keeps its text in `value`, every other node in `label`.
fn no_artcraft(h: &Harness<'static, SlideApp>, place: &str) {
    let mark = h
        .query_all_by(|n| {
            [n.label(), n.value()]
                .into_iter()
                .flatten()
                .any(|l| (l.to_ascii_lowercase().contains("artcraft") && !l.contains("by the ArtCraft team")) || l.contains("Discord"))
        })
        .next();
    assert!(mark.is_none(), "{place}: {mark:?}");
}

/// A 4 x 4 opaque red PNG.
const RED_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x04,
    0x08, 0x06, 0x00, 0x00, 0x00, 0xA9, 0xF1, 0x9E, 0x7E, 0x00, 0x00, 0x00, 0x12, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0,
    0x1F, 0x19, 0x33, 0x90, 0x2E, 0x00, 0x00, 0x3C, 0x40, 0x1F, 0xE1, 0xE0, 0x81, 0x77, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

/// The URL the app handed to egui over the next few frames, if any.
fn opened_url(h: &mut Harness<'static, SlideApp>) -> Option<String> {
    let mut opened = None;
    for _ in 0..4 {
        h.step();
        opened = opened.or_else(|| {
            h.output().platform_output.commands.iter().find_map(|c| match c {
                egui::OutputCommand::OpenUrl(open) => Some(open.url.clone()),
                _ => None,
            })
        });
    }
    opened
}

#[test]
fn no_artcraft_label_in_home_and_about() {
    kawan::set(Policy::default());
    let mut h = harness("Home");
    h.state_mut().run("app.backstage", json!({"page": "home"})).unwrap();
    h.run_steps(4);
    no_artcraft(&h, "File > Home");
    assert!(h.query_all_by_label("KawanEditor").count() >= 1, "the Home heading");
    h.state_mut().run("app.backstage", json!({"page": "account"})).unwrap();
    h.run_steps(4);
    no_artcraft(&h, "File > Account");
    assert!(h.query_all_by_label_contains("About KawanEditor").next().is_some(), "the About task (button and title)");
    assert!(h.query_all_by_label("Source on GitHub").next().is_some(), "the upstream source link stays");
    h.state_mut().run("app.backstage", json!({"on": false})).unwrap();
    h.state_mut().run("app.about", json!({})).unwrap();
    h.run_steps(4);
    no_artcraft(&h, "About");
    assert!(h.query_all_by_label("About KawanEditor").next().is_some(), "the dialog title");
    let about = kawan::about("DeckCraft");
    assert!(h.query_all_by_label(&about).next().is_some(), "the About line {about:?}");
    assert!(h.query_all_by_label("KawanEditor").count() >= 2, "the About heading and the title bar logo");
    assert!(h.query_all_by_label("Source on GitHub").next().is_some(), "the upstream source link stays");
}

#[test]
fn header_logo_is_drawn() {
    kawan::set(Policy::default());
    let h = harness("Home");
    assert!(h.state().session.active().is_some(), "a presentation is open");
    let logo = topmost(&h, "KawanEditor").rect();
    let tab = topmost(&h, "Review").rect();
    assert!(logo.bottom() <= tab.top(), "in the title bar, above the ribbon tabs: {logo:?} {tab:?}");
    // The quick access buttons are small unlabelled nodes in the title bar (wherever the bar
    // starts: macOS leaves room for the traffic lights).
    let buttons: Vec<egui::Rect> = h
        .query_all_by(|n| n.label().is_none() && n.bounding_box().is_some_and(|b| b.y1 <= f64::from(tab.top()) && b.width() < 40.0))
        .map(|n| n.rect())
        .collect();
    assert!(!buttons.is_empty(), "the quick access buttons");
    assert!(buttons.iter().all(|b| logo.right() <= b.left()), "at its left end, before the quick access buttons: {logo:?} {buttons:?}");
}

#[test]
fn host_logo_with_href_opens_it() {
    kawan::set(Policy { logo_png: Some(RED_PNG.into()), logo_href: Some("https://example.com/".into()), ..Policy::default() });
    let mut h = harness("Home");
    topmost(&h, "KawanEditor").click();
    assert_eq!(opened_url(&mut h).as_deref(), Some("https://example.com/"));
    let logo = deckcraft_ui_egui::kawan_brand::logo_texture(&h.ctx).map(|t| t.size());
    assert_eq!(logo, Some([4, 4]), "the host's PNG, not the default");
    kawan::set(Policy::default());
}

#[test]
fn host_logo_without_href_is_not_a_link() {
    kawan::set(Policy { logo_png: Some(b"not a png".to_vec().into()), ..Policy::default() });
    let mut h = harness("Home");
    topmost(&h, "KawanEditor").click();
    assert_eq!(opened_url(&mut h), None);
    let logo = deckcraft_ui_egui::kawan_brand::logo_texture(&h.ctx).map(|t| t.size());
    assert_eq!(logo, Some([256, 256]), "a bad host PNG falls back to the default logo");
    kawan::set(Policy::default());
}
