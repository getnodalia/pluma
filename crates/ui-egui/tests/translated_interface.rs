// SPDX-License-Identifier: MIT OR Apache-2.0
//! Every screen draws its text in the interface language. The dialog or panel behind every menu
//! command is drawn in Spanish; a text that is still the English key of a Spanish catalog row was
//! drawn without going through the catalog.
// Integration tests: unwrapping and panicking on failure is fine here, unlike in shipped code (AGENTS.md › Robustness).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};

use egui::epaint::Shape;
use egui_kittest::Harness;
use pluma_engine::Session;
use pluma_ui_egui::{PlumaApp, i18n, menus};
use serde_json::json;

/// Names of what the app ships or the walk makes, drawn as stored: a colour profile, its inks, an action in
/// the Actions panel. They are data the person can rename or replace, not interface text.
const DATA: &[&str] = &[
    // The ink names Color Settings lists for the working CMYK space.
    "Black",
    "Cyan",
    "Yellow",
    "Pluma Generic CMYK (SWOP-like)",
    "Center on Artboard",
    "Duplicate and Offset 10 pt",
    "Outline Text",
    "Reflect Horizontal",
    "Rotate 90° CW",
    "Unite and Simplify",
];

/// Texts whose draw site is still to be found. Each is a bug, never a place to add to: remove one
/// when its site goes through `tl!`.
const KNOWN: &[&str] = &["[Default]", "Bevel", "Custom", "H:", "Media", "Points", "Seed:", "W:"];

fn harness() -> Harness<'static, PlumaApp> {
    let mut app = PlumaApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width": 400, "height": 250})).unwrap();
    app.run("shape.rectangle", json!({"x": 20, "y": 20, "width": 100, "height": 60})).ok();
    app.run("text.create", json!({"x": 30, "y": 120, "text": "Sample", "font": "Inter"})).ok();
    app.session.prefs.interface_language = "es".into();
    let mut h = Harness::builder().with_size(egui::vec2(1440.0, 900.0)).with_accessibility_check(false).allow_missing_glyphs().build_ui_state(
        |ui, app: &mut PlumaApp| {
            // What the desktop host does once per iteration: it is `tick` that sets the language.
            app.begin_host_iteration();
            app.tick(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

fn collect_shape(shape: &Shape, out: &mut BTreeMap<String, String>) {
    match shape {
        Shape::Text(t) => {
            for line in t.galley.text().split('\n') {
                let line = line.trim();
                if !line.is_empty() {
                    out.entry(line.to_owned()).or_insert_with(|| "painted".to_owned());
                }
            }
        }
        Shape::Vec(v) => v.iter().for_each(|s| collect_shape(s, out)),
        _ => {}
    }
}

/// Everything the frame paints as text. Accessible names are not read: Pluma names its widgets for
/// screen readers in English throughout, which is a separate piece of work from what a sighted
/// person sees.
fn shown(h: &Harness<'_, PlumaApp>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for clipped in &h.output().shapes {
        collect_shape(&clipped.shape, &mut out);
    }
    out
}

/// Texts of the frame that are English keys of a Spanish row with another reading.
fn untranslated(frame: &BTreeMap<String, String>) -> Vec<String> {
    let es = i18n::Lang::from_code("es").expect("Spanish is registered");
    frame
        .iter()
        .filter(|(t, _)| !DATA.contains(&t.as_str()) && !KNOWN.contains(&t.as_str()))
        .filter(|(t, _)| i18n::has(es, t) && i18n::tr(es, t) != t.as_str())
        .map(|(t, kind)| format!("{t:?} ({kind})"))
        .collect()
}

/// Commands that leave the app or close the document the walk draws on.
fn skipped(id: &str) -> bool {
    id.contains("quit") || id.contains("exit") || id.starts_with("file.close") || id == "app.quit"
}

fn walk() -> Vec<String> {
    let mut failures: Vec<String> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut h = harness();
    let mut note = |what: &str, frame: &BTreeMap<String, String>, failures: &mut Vec<String>| {
        for line in untranslated(frame) {
            if seen.insert(line.clone()) {
                failures.push(format!("{what}: {line}"));
            }
        }
    };
    note("main window", &shown(&h), &mut failures);
    let entries: Vec<(String, serde_json::Value)> =
        menus::menu_entries(h.state()).into_iter().filter(|e| e.enabled).filter_map(|e| e.command.map(|c| (c, e.params))).collect();
    for (id, params) in entries {
        if skipped(&id) {
            continue;
        }
        menus::invoke(h.state_mut(), &id, params);
        h.run_steps(3);
        note(&format!("command {id}"), &shown(&h), &mut failures);
        for _ in 0..3 {
            if h.state().ui.dialog.is_some() {
                pluma_ui_egui::dialogs::cancel(h.state_mut());
            }
            h.key_press(egui::Key::Escape);
            h.run_steps(2);
        }
    }
    failures
}

/// A screen that draws a text the catalog translates, without going through the catalog, shows
/// English to everyone who runs Pluma in Spanish.
#[test]
fn spanish_interface_has_no_untranslated_text() {
    let failures = walk();
    assert!(failures.is_empty(), "texts drawn in English although Spanish has them:\n{}", failures.join("\n"));
}
