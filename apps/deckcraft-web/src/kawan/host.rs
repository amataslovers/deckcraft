//! DeckCraft as a KawanEditor bridge host. Documents always save as pptx: DeckCraft's own Ctrl+S
//! on the web writes `.deckcraft`, which `classify` turns into a pptx save.

use deckcraft_engine::cmd::base64_encode;
use deckcraft_ui_egui::SlideApp;
use serde_json::{Value, json};

use super::policy::{self, Policy, Theme};
use super::protocol::{Classify, CommandInfo, Host};

/// Commands that change the document although they are not undoable, or replace it with another
/// one (a save would then hand that one to the page under this document's key),
/// and the user's own saves: view mode saves nothing.
const ALSO_MUTATING: &[&str] = &[
    "edit.undo", "edit.redo", "edit.cut", "file.revert", "text.compose", "text.commit", "file.new", "file.open*", "file.close",
    "file.recovery.open", "file.recovery.discard", "app.closePresentation", "app.openDialog", "file.save*", "app.saveAsDialog",
];

fn undoable(id: &str) -> bool {
    deckcraft_engine::find_command(id).is_some_and(|s| s.undoable)
}

/// State queries and selection moves the UI runs on its own; the host's lists skip them.
const QUERIES: &[&str] = &["format.state", "file.info", "text.get", "edit.select"];

/// The OS colour scheme, read once on `init`: DeckCraft has no mode that follows it.
fn prefers_dark() -> bool {
    web_sys::window().and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten()).is_some_and(|m| m.matches())
}

pub struct DeckHost<'a>(pub &'a mut SlideApp);

impl DeckHost<'_> {
    fn serialize(&self) -> Result<(String, Vec<u8>), String> {
        let doc = self.0.session.active().ok_or("no document")?;
        let bytes = deckcraft_engine::cmd::file::save_bytes(&doc.doc, "pptx").map_err(|e| e.to_string())?;
        Ok((format!("{}.pptx", doc.title()), bytes))
    }
}

impl Host for DeckHost<'_> {
    fn open(&mut self, name: &str, bytes: Vec<u8>) -> Result<(), String> {
        let session = &mut self.0.session;
        // The bridge's own open: the host's command deny list does not apply to it.
        policy::as_host(|| session.execute("file.openBytes", &json!({"name": name, "data": base64_encode(&bytes)}))).map_err(|e| e.to_string())?;
        // The opened deck is the last document; one editor holds one document, so close the rest.
        while session.documents().len() > 1 {
            session.close_document(0);
        }
        Ok(())
    }

    fn save(&mut self) -> Result<(String, Vec<u8>), String> {
        let saved = self.serialize()?;
        self.mark_saved();
        Ok(saved)
    }

    fn export_copy(&mut self) -> Result<(String, Vec<u8>), String> {
        self.serialize()
    }

    fn exec(&mut self, id: &str, params: Value) -> Result<Value, String> {
        self.0.run(id, params)
    }

    fn commands(&self) -> Vec<CommandInfo> {
        self.0
            .session
            .commands()
            .into_iter()
            .filter(|c| policy::allows(c.id, undoable(c.id)).is_ok())
            .map(|c| CommandInfo { id: c.id.to_string(), label: c.label.to_string(), menu: c.menu.join(" › "), mutates: undoable(c.id) })
            .collect()
    }

    fn dirty(&self) -> bool {
        self.0.session.documents().iter().any(|d| d.is_dirty())
    }

    fn revision(&self) -> u64 {
        // ponytail: also moves on selection changes, so `edited` fires for them too (it only resets
        // the autosave debounce). The document Arc's address can repeat between ticks, so it can't
        // be used instead without keeping the last Arc alive.
        self.0.session.active().map_or(0, |d| d.revision)
    }

    fn set_author(&mut self, name: &str) {
        self.0.session.prefs.author = name.chars().take(200).collect();
    }

    fn set_policy(&mut self, mut p: Policy) {
        p.mutating = ALSO_MUTATING.iter().map(|s| s.to_string()).collect();
        p.queries = QUERIES.iter().map(|s| s.to_string()).collect();
        let dark = match p.theme {
            Theme::Light => false,
            Theme::Dark => true,
            Theme::System => prefers_dark(),
        };
        policy::set(p);
        // Always, `System` too: `view.dark` restyles, which brings in the host accent.
        let _ = policy::as_host(|| self.0.run("view.dark", json!({"on": dark})));
    }

    fn classify(&self, name: &str) -> Classify {
        match name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).as_deref() {
            Some("pptx") => Classify::Save,
            // DeckCraft's own Ctrl+S on the web writes its native format: save as pptx instead.
            Some("deckcraft") => Classify::Convert,
            _ => Classify::Export,
        }
    }

    fn mark_saved(&mut self) {
        if let Some(doc) = self.0.session.active_mut() {
            doc.saved_doc = doc.doc.clone();
        }
    }
}
