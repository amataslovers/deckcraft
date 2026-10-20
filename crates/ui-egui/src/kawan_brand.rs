//! KawanEditor branding (logo, About line) and the host's accent colour for the DeckCraft UI.
//! DeckCraft's own UI calls these at a few hook points; everything here reads the host policy
//! (`deckcraft_engine::kawan`) and falls back to KawanEditor's default logo and DeckCraft's own
//! colours.

use deckcraft_engine::kawan as policy;
use egui::{Color32, Rect, Sense, pos2, vec2};

use crate::theme::Tokens;

/// The default logo: KawanEditor's mark, 256 x 256.
const DEFAULT_LOGO: &[u8] = include_bytes!("../assets/kawan-logo.png");
/// Larger host logos are refused (a small PNG can inflate to a huge bitmap).
const MAX_SIDE: u32 = 2048;

/// The width and height a PNG's header declares (the IHDR chunk right after the signature).
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if !policy::is_png(bytes) || bytes.get(12..16) != Some(&b"IHDR"[..]) {
        return None;
    }
    let be32 = |i: usize| bytes.get(i..i + 4).and_then(|s| <[u8; 4]>::try_from(s).ok()).map(u32::from_be_bytes);
    Some((be32(16)?, be32(20)?))
}

/// Decode a PNG into an egui image; `None` for anything that is not a PNG of a sane size. The
/// header is checked before decoding, so a decompression bomb is never inflated.
pub fn decode_png(bytes: &[u8]) -> Option<egui::ColorImage> {
    let sane = |w: u32, h: u32| w > 0 && h > 0 && w <= MAX_SIDE && h <= MAX_SIDE;
    let (w, h) = png_size(bytes)?;
    if !sane(w, h) {
        return None;
    }
    let rgba = deckcraft_render::decode_image(bytes)?;
    let (w, h) = rgba.dimensions();
    if !sane(w, h) {
        return None;
    }
    let size = [usize::try_from(w).ok()?, usize::try_from(h).ok()?];
    let raw = rgba.as_raw();
    (raw.len() == size[0].checked_mul(size[1])?.checked_mul(4)?).then(|| egui::ColorImage::from_rgba_unmultiplied(size, raw))
}

/// The logo texture: the host's PNG when it sent one that decodes, otherwise KawanEditor's.
/// Cached per policy generation, so a new policy brings its new logo.
pub fn logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("kawan-logo");
    let generation = policy::generation();
    if let Some((built, texture)) = ctx.data(|d| d.get_temp::<(u64, egui::TextureHandle)>(id))
        && built == generation
    {
        return Some(texture);
    }
    let image = policy::logo_png().and_then(|png| decode_png(&png)).or_else(|| decode_png(DEFAULT_LOGO))?;
    let texture = ctx.load_texture("kawan-logo", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, (generation, texture.clone())));
    Some(texture)
}

/// The logo drawn into `r` (Home, Account and About, where DeckCraft drew its own mark).
pub fn paint_logo(p: &egui::Painter, r: Rect) {
    if let Some(texture) = logo_texture(p.ctx()) {
        p.image(texture.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
}

/// The logo, `size` points square, labelled "KawanEditor". With a host `href` it is a link that
/// opens in a new tab (DeckCraft's own links end in the same `ctx.open_url`).
pub fn logo(ui: &mut egui::Ui, size: f32) -> egui::Response {
    let href = policy::logo_href();
    let sense = if href.is_some() { Sense::click() } else { Sense::hover() };
    let response = match logo_texture(ui.ctx()) {
        Some(texture) => ui.add(egui::Image::from_texture(&texture).fit_to_exact_size(vec2(size, size)).alt_text(policy::PRODUCT).sense(sense)),
        None => {
            let response = ui.allocate_response(vec2(size, size), sense);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, policy::PRODUCT));
            response
        }
    };
    match href {
        Some(href) => {
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(href));
            }
            response
        }
        None => response,
    }
}

/// The small logo at the left end of the title bar, before the quick access buttons.
pub fn header_logo(ui: &mut egui::Ui) -> egui::Response {
    ui.add_space(4.0);
    let response = logo(ui, 18.0);
    ui.add_space(6.0);
    response
}

/// The About dialog's credit line (it replaces DeckCraft's copyright line).
pub fn about() -> String {
    policy::about("DeckCraft")
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// DeckCraft's tokens with the host accent: the accent and the guides, the contextual tab names
/// and the text on top of the accent. Unchanged without an accent.
pub fn tint(t: Tokens) -> Tokens {
    match policy::accent() {
        Some(a) => tint_with(t, a),
        None => t,
    }
}

fn tint_with(mut t: Tokens, a: [u8; 3]) -> Tokens {
    let contextual = if t.dark { policy::mix(a, [255; 3], 0.25) } else { policy::mix(a, [0; 3], 0.10) };
    t.accent = rgb(a);
    t.guide = rgb(a);
    t.contextual = rgb(contextual);
    // `accent_text` is the text on accent fills (Present, Save, OK).
    t.accent_text = rgb(policy::on_color(a));
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_accent(accent: Option<[u8; 3]>) {
        policy::set(policy::Policy { accent, ..policy::Policy::default() });
    }

    #[test]
    fn tint_is_identity_without_accent() {
        with_accent(None);
        for t in [Tokens::light(), Tokens::dark()] {
            assert_eq!(format!("{:?}", tint(t)), format!("{t:?}"));
        }
        assert_eq!(format!("{:?}", Tokens::for_brightness(crate::theme::Brightness::Dark)), format!("{:?}", Tokens::dark()));
    }

    #[test]
    fn tint_replaces_the_accent_family() {
        let a = [0xE1, 0x1D, 0x48];
        with_accent(Some(a));
        let light = tint(Tokens::light());
        assert_eq!(light.accent, rgb(a));
        assert_eq!(light.guide, rgb(a));
        assert_eq!(light.contextual, rgb([203, 26, 65]));
        assert_eq!(light.accent_text, rgb([255, 255, 255]));
        let dark = tint(Tokens::dark());
        assert_eq!(dark.accent, rgb(a));
        assert_eq!(dark.guide, rgb(a));
        assert_eq!(dark.contextual, rgb([233, 86, 118]));
        assert_eq!(dark.accent_text, rgb([255, 255, 255]));
        // Everything else stays DeckCraft's.
        assert_eq!(dark.chrome, Tokens::dark().chrome);
        assert_eq!(light.selection, Tokens::light().selection);
        assert_eq!(light.text_selection, Tokens::light().text_selection);
        // The gallery previews keep calling the pure tokens; the UI's own builders tint.
        assert_eq!(Tokens::light().accent, Color32::from_rgb(0xD9, 0x5A, 0x1A));
        assert_eq!(Tokens::for_brightness(crate::theme::Brightness::Light).accent, rgb(a));
        with_accent(None);
    }

    #[test]
    fn on_accent_is_readable() {
        with_accent(Some([0xFF, 0xFF, 0x00]));
        for t in [tint(Tokens::light()), tint(Tokens::dark())] {
            assert_eq!(t.accent_text, rgb([17, 17, 17]));
        }
        with_accent(None);
    }

    /// A 4 x 4 opaque red PNG.
    const RED_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x04, 0x08, 0x06, 0x00, 0x00, 0x00, 0xA9, 0xF1, 0x9E, 0x7E, 0x00, 0x00, 0x00, 0x12, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF,
        0xC0, 0xF0, 0x1F, 0x19, 0x33, 0x90, 0x2E, 0x00, 0x00, 0x3C, 0x40, 0x1F, 0xE1, 0xE0, 0x81, 0x77, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn bad_host_png_falls_back_to_default() {
        assert!(decode_png(b"nope").is_none());
        assert!(decode_png(&[]).is_none());
        assert!(decode_png(&RED_PNG[..40]).is_none(), "a PNG header alone is not enough");
        assert_eq!(decode_png(DEFAULT_LOGO).map(|i| i.size), Some([256, 256]));
        assert_eq!(decode_png(RED_PNG).map(|i| i.size), Some([4, 4]));
    }

    #[test]
    fn oversized_or_non_png_is_refused() {
        let mut huge = RED_PNG.to_vec();
        huge[16..20].copy_from_slice(&(MAX_SIDE + 1).to_be_bytes());
        assert_eq!(png_size(&huge), Some((MAX_SIDE + 1, 4)));
        assert!(decode_png(&huge).is_none(), "refused from the header, before decoding");
        // A GIF decodes in DeckCraft's own image decoder, but the host logo is PNG only.
        assert!(decode_png(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;").is_none());
    }

    #[test]
    fn logo_texture_follows_the_policy() {
        let ctx = egui::Context::default();
        policy::set(policy::Policy { logo_png: Some(RED_PNG.into()), ..policy::Policy::default() });
        assert_eq!(logo_texture(&ctx).map(|t| t.size()), Some([4, 4]));
        let first = logo_texture(&ctx).map(|t| t.id());
        assert_eq!(logo_texture(&ctx).map(|t| t.id()), first, "cached while the policy stays");
        let textures = ctx.tex_manager().read().num_allocated();
        policy::set(policy::Policy { logo_png: Some(b"nope".to_vec().into()), ..policy::Policy::default() });
        assert_eq!(logo_texture(&ctx).map(|t| t.size()), Some([256, 256]), "a bad host PNG falls back");
        assert_ne!(logo_texture(&ctx).map(|t| t.id()), first, "a new policy, a new texture");
        for _ in 0..3 {
            policy::set(policy::Policy { logo_png: Some(RED_PNG.into()), ..policy::Policy::default() });
            assert_eq!(logo_texture(&ctx).map(|t| t.size()), Some([4, 4]));
        }
        assert_eq!(ctx.tex_manager().read().num_allocated(), textures, "the old logo is freed, not kept per generation");
        policy::set(policy::Policy::default());
    }

    #[test]
    fn about_names_deck_craft() {
        assert_eq!(about(), policy::about("DeckCraft"));
        assert!(about().starts_with("KawanEditor "));
    }
}
