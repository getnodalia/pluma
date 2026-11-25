//! File → Template Gallery…: the built-in templates (`file.templates`) as cards with their
//! thumbnails, in category tabs, wrapped to the window's width and scrolled when they do not all
//! fit. A click chooses a card (its summary and size show under the grid), a double-click, Enter or
//! Create opens it as a new untitled document (`file.newFromGallery`). The Home screen's template
//! row uses the same cards.
//!
//! Fields: `category` (`all` or a category id) and `template` (the chosen template's id; setting
//! it chooses that card).

use pluma_engine::cmd::gallery::{self, CATEGORIES, Lang, TEMPLATES, Template};
use serde_json::{Value, json};

use super::DialogSpec;
use crate::state::Dialog;
use crate::theme::{self, Tokens};
use crate::{PlumaApp, widgets};

pub(super) const KIND: &str = "templateGallery";

pub(super) const SPEC: DialogSpec = DialogSpec { inputs: |_| &["template", "category"], ..DialogSpec::window(show, confirm) };

/// The window's inner width (the cards wrap to it) and the grid's height.
const WIDTH: f32 = 760.0;
const GRID_HEIGHT: f32 = 400.0;

/// A card's width and height, and the thumbnail's height inside it.
const CARD: egui::Vec2 = egui::vec2(172.0, 196.0);
const THUMB_HEIGHT: f32 = 128.0;

/// Open the dialog, on `template` when it names one.
pub fn open(app: &mut PlumaApp, template: Option<&str>) {
    let first = template.and_then(gallery::find).or_else(|| TEMPLATES.first());
    app.ui.dialog = Some(Dialog::new(KIND, json!({ "category": "all", "template": first.map_or("", |t| t.id) })));
}

fn chosen(d: &Dialog) -> Option<&'static Template> {
    gallery::find(&d.str("template"))
}

/// The templates of the tab `category`.
fn in_category(category: &str) -> Vec<&'static Template> {
    TEMPLATES.iter().filter(|t| category == "all" || t.category.id == category).collect()
}

/// A template card: its thumbnail on a page, the name and the size. Returns the card's response.
pub fn card(ui: &mut egui::Ui, template: &'static Template, lang: Lang, selected: bool) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(CARD, egui::Sense::click());
    let p = ui.painter();
    p.rect_filled(r, egui::CornerRadius::same(8), if resp.hovered() || selected { t.hover } else { t.panel_darker });
    if selected {
        p.rect_stroke(r, egui::CornerRadius::same(8), egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
    }
    // The thumbnail fits the box it is given, whatever the template's proportions.
    let slot = egui::Rect::from_min_size(r.min + egui::vec2(10.0, 10.0), egui::vec2(CARD.x - 20.0, THUMB_HEIGHT));
    let thumb = crate::canvas::gallery_thumbs::get(ui.ctx(), template, lang);
    let (w, h) = thumb.map_or((template.width, template.height), |(_, size)| (f64::from(size.x), f64::from(size.y)));
    let k = (f64::from(slot.width()) / w.max(1e-9)).min(f64::from(slot.height()) / h.max(1e-9)) as f32;
    let page = egui::Rect::from_center_size(slot.center(), egui::vec2((w as f32 * k).max(4.0), (h as f32 * k).max(4.0)));
    p.rect_filled(page.translate(egui::vec2(1.5, 2.5)), egui::CornerRadius::same(2), egui::Color32::from_black_alpha(70));
    p.rect_filled(page, egui::CornerRadius::same(2), egui::Color32::WHITE);
    if let Some((id, _)) = thumb {
        p.image(id, page, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    }
    let name = template.name(lang);
    let mut job = egui::text::LayoutJob::single_section(
        name.to_string(),
        egui::TextFormat { font_id: theme::semibold(12.0), color: t.text_strong, ..Default::default() },
    );
    job.halign = egui::Align::Center;
    job.wrap = egui::text::TextWrapping { max_width: r.width() - 16.0, max_rows: 2, break_anywhere: false, overflow_character: Some('…') };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let name_h = galley.size().y;
    p.galley(egui::pos2(r.center().x, r.top() + THUMB_HEIGHT + 22.0), galley, t.text_strong);
    p.text(
        egui::pos2(r.center().x, r.top() + THUMB_HEIGHT + 26.0 + name_h),
        egui::Align2::CENTER_TOP,
        template.size_label(),
        egui::FontId::proportional(11.0),
        t.text_dim,
    );
    resp.on_hover_text(template.summary(lang))
}

fn show(app: &mut PlumaApp, ctx: &egui::Context) {
    let Some(mut d) = app.ui.dialog.clone() else { return };
    let lang = gallery::language(&app.session);
    let t = Tokens::get(ctx);
    let (mut create, mut close) = (false, false);
    super::modal::show(ctx, tl!("Template Gallery"), egui::Id::unique(("dialog", KIND)), -20.0, 22, |ui| {
        ui.set_width(WIDTH);
        super::modal::heading(ui, tl!("Template Gallery"));
        ui.add_space(10.0);
        let mut tabs: Vec<&str> = vec![tl!("All")];
        tabs.extend(CATEGORIES.iter().map(|c| c.name(lang)));
        let current = CATEGORIES.iter().position(|c| c.id == d.str("category")).map_or(0, |i| i + 1);
        if let Some(i) = widgets::tab_bar(ui, &tabs, current) {
            let id = if i == 0 { "all" } else { CATEGORIES.get(i - 1).map_or("all", |c| c.id) };
            d.fields.insert("category".into(), json!(id));
        }
        ui.add_space(12.0);
        let list = in_category(&d.str("category"));
        // A tab keeps the chosen card when it shows it, else chooses its first.
        if !list.iter().any(|t| Some(t.id) == chosen(&d).map(|c| c.id)) {
            d.fields.insert("template".into(), json!(list.first().map_or("", |t| t.id)));
        }
        let picked = chosen(&d);
        egui::ScrollArea::vertical().id_salt(("template-gallery", d.str("category"))).max_height(GRID_HEIGHT).auto_shrink([false, false]).show(
            ui,
            |ui| {
                ui.set_width(WIDTH);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                    for template in &list {
                        let resp = card(ui, template, lang, Some(template.id) == picked.map(|p| p.id));
                        if resp.clicked() {
                            d.fields.insert("template".into(), json!(template.id));
                        }
                        if resp.double_clicked() {
                            create = true;
                        }
                    }
                });
            },
        );
        ui.add_space(12.0);
        let (title, detail) =
            picked.map_or((String::new(), String::new()), |p| (p.name(lang).to_string(), format!("{} · {}", p.size_label(), p.summary(lang))));
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(WIDTH - 220.0);
                ui.label(egui::RichText::new(title).font(theme::semibold(13.0)).color(t.text_strong));
                ui.label(egui::RichText::new(detail).color(t.text_dim));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                create |= widgets::primary_button_enabled(ui, tl!("Create"), picked.is_some()).clicked();
                ui.add_space(8.0);
                close |= widgets::secondary_button(ui, tl!("Close")).clicked();
            });
        });
    });
    let enter = crate::shortcuts::pressed_anew(ctx, egui::Key::Enter) && !ctx.memory(|m| m.focused().is_some());
    app.ui.dialog = Some(d);
    if close {
        app.ui.dialog = None;
    } else if (create || enter)
        && let Err(e) = super::confirm(app)
    {
        app.status(e);
    }
}

/// Create: open the chosen template as a new document and close the dialog.
fn confirm(app: &mut PlumaApp, d: &Dialog) -> Result<Value, String> {
    let template = chosen(d).ok_or_else(|| tl!("Choose a template first.").to_string())?;
    let r = app.run("file.newFromGallery", json!({ "template": template.id }));
    if r.is_ok() {
        app.ui.dialog = None;
    }
    r
}

#[cfg(test)]
mod tests {
    use pluma_engine::Session;

    use super::*;

    fn app() -> PlumaApp {
        PlumaApp::new(Session::new(), Default::default())
    }

    fn frame(app: &mut PlumaApp, width: f32) {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        for pass in 1..=2 {
            let screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 900.0)));
            let raw = egui::RawInput { screen_rect, time: Some(f64::from(pass) * 0.5), ..Default::default() };
            let mut out = ctx.run_ui(raw, |ui| crate::dialogs::show(app, ui.ctx()));
            out.textures_delta.clear();
        }
    }

    fn field(app: &PlumaApp, key: &str) -> Value {
        app.ui.dialog.as_ref().and_then(|d| d.fields.get(key)).cloned().unwrap_or_default()
    }

    #[test]
    fn the_command_opens_the_gallery_on_the_named_template_or_the_first() {
        let mut app = app();
        app.run("file.templateGallery", json!({})).unwrap();
        assert_eq!(app.ui.dialog.as_ref().map(|d| d.kind.as_str()), Some(KIND));
        assert_eq!(field(&app, "template"), json!(TEMPLATES[0].id));
        app.run("file.templateGallery", json!({"template": "social-post"})).unwrap();
        assert_eq!(field(&app, "template"), json!("social-post"));
        app.run("file.templateGallery", json!({"template": "nope"})).unwrap();
        assert_eq!(field(&app, "template"), json!(TEMPLATES[0].id));
    }

    #[test]
    fn create_opens_the_chosen_template_as_a_new_document_and_closes_the_dialog() {
        let mut app = app();
        app.run("file.templateGallery", json!({"template": "business-card"})).unwrap();
        frame(&mut app, 1400.0);
        crate::dialogs::confirm(&mut app).unwrap();
        assert!(app.ui.dialog.is_none());
        assert_eq!(app.session.documents().len(), 1);
        assert!(app.session.doc().unwrap().doc.title.starts_with("Untitled"));
    }

    #[test]
    fn nothing_is_created_without_a_chosen_template() {
        let mut app = app();
        app.run("file.templateGallery", json!({})).unwrap();
        if let Some(d) = app.ui.dialog.as_mut() {
            d.fields.insert("template".into(), json!(""));
        }
        assert!(crate::dialogs::confirm(&mut app).is_err());
        assert!(app.session.documents().is_empty() && app.ui.dialog.is_some());
    }

    #[test]
    fn a_tab_that_does_not_hold_the_chosen_template_chooses_its_first_card() {
        let mut app = app();
        app.run("file.templateGallery", json!({"template": "poster"})).unwrap();
        if let Some(d) = app.ui.dialog.as_mut() {
            d.fields.insert("category".into(), json!("social"));
        }
        frame(&mut app, 1400.0);
        let first = in_category("social").first().map(|t| t.id);
        assert_eq!(field(&app, "template"), json!(first.unwrap()));
        // A tab that holds it keeps it.
        if let Some(d) = app.ui.dialog.as_mut() {
            d.fields.insert("category".into(), json!("all"));
            d.fields.insert("template".into(), json!("event-flyer"));
        }
        frame(&mut app, 1400.0);
        assert_eq!(field(&app, "template"), json!("event-flyer"));
    }

    #[test]
    fn every_tab_has_cards_and_together_they_hold_every_template() {
        for c in CATEGORIES {
            assert!(!in_category(c.id).is_empty(), "{}", c.id);
        }
        assert_eq!(CATEGORIES.iter().map(|c| in_category(c.id).len()).sum::<usize>(), TEMPLATES.len());
        assert_eq!(in_category("all").len(), TEMPLATES.len());
    }

    /// The cards wrap to the room they have, in rows, and none leaves it: a narrow window shows
    /// fewer to a row instead of cutting one off.
    #[test]
    fn cards_wrap_to_the_width_and_none_is_cut_off() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let mut rows_at = vec![];
        for width in [260.0_f32, 420.0, 760.0, 1200.0] {
            let mut rects = vec![];
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width + 40.0, 3000.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.set_width(width);
                    let left = ui.min_rect().left();
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                        for t in TEMPLATES {
                            rects.push(card(ui, t, Lang::En, false).rect);
                        }
                    });
                    rects.iter_mut().for_each(|r| *r = r.translate(egui::vec2(-left, 0.0)));
                });
            });
            out.textures_delta.clear();
            assert_eq!(rects.len(), TEMPLATES.len());
            assert!(rects.iter().all(|r| r.right() <= width + 0.5), "a card is cut off at {width}: {rects:?}");
            let mut tops: Vec<i32> = rects.iter().map(|r| r.top().round() as i32).collect();
            tops.dedup();
            rows_at.push(tops.len());
        }
        assert!(rows_at.windows(2).all(|w| w[0] >= w[1]), "narrower windows have at least as many rows: {rows_at:?}");
        assert!(rows_at[0] > rows_at[3], "{rows_at:?}");
    }
}
