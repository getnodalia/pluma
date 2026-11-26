//! Help → About Pluma, as every Nodalia app shows it (`nodalia_theme::about`): the heading with its
//! round close button, then the page with Pluma's mark, name and version, what it is, the suite it
//! belongs to and the links. There are no Contributors or Models tabs; the authors of the code this
//! app began from are credited in `NOTICE`.

use nodalia_theme::about as ndl;

use crate::PlumaApp;
use crate::theme;

pub(super) fn show(app: &mut PlumaApp, ctx: &egui::Context) {
    if !app.ui.about {
        return;
    }
    let t = theme::Tokens::get(ctx);
    let look = t.creator();
    let mut close = false;
    let mut open_cmd: Option<&'static str> = None;
    let frame = egui::Frame::window(&ctx.global_style())
        .inner_margin(egui::Margin::symmetric(18, 14))
        .corner_radius(egui::CornerRadius::same(t.card_radius.max(theme::CARD_RADIUS)));
    egui::Window::new(tl!("About Pluma"))
        .id(egui::Id::unique("about-pluma"))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(ndl::measures::WIDTH);
            close = ndl::heading(ui, &look, tl!("About Pluma"), egui::Id::NULL.with("pluma.about.close")).closed;
            let l = app.session.execute("help.links", &serde_json::json!({})).unwrap_or_default();
            let version = crate::i18n::fmt(tl!("Version {version}"), &[("version", env!("CARGO_PKG_VERSION"))]);
            let links = [
                ndl::Link { label: tl!("Pluma Website"), url: l["website"].as_str().unwrap_or("") },
                ndl::Link { label: tl!("Source code on GitHub"), url: l["github"].as_str().unwrap_or("") },
            ];
            const COMMANDS: [&str; 2] = ["help.website", "help.github"];
            let info = ndl::Info {
                name: "Pluma",
                version: &version,
                tagline: tl!("Open-source vector illustration in pure Rust."),
                suite: tl!("Part of the Nodalia suite of open creative tools."),
                links: &links,
                footer: tl!("Open source. No telemetry."),
            };
            let page = ndl::page(ui, &look, &info, |ui, rect| crate::brand::paint_mark(ui, rect));
            if let Some(i) = page.picked {
                open_cmd = COMMANDS.get(i).copied();
            }
            ui.add_space(4.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(tl!("Includes work by other open-source authors (see NOTICE).").to_string()).small().color(t.text_dim));
            });
        });
    // Escape closes it, as it does a dialog.
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        close = true;
    }
    if let Some(cmd) = open_cmd {
        app.open_link(cmd);
    }
    if close {
        app.ui.about = false;
    }
}
