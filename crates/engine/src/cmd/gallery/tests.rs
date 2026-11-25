use pluma_doc::NodeKind;
use serde_json::json;

use super::*;

fn texts(s: &Session) -> Vec<String> {
    let mut all = vec![];
    if let Ok(d) = s.doc() {
        d.doc.walk(|n| {
            if let NodeKind::Text(t) = &n.kind {
                all.push(t.plain_text());
            }
        });
    }
    all
}

#[test]
fn every_template_has_a_unique_id_a_category_and_both_languages() {
    let mut ids = std::collections::HashSet::new();
    for t in TEMPLATES {
        assert!(ids.insert(t.id), "duplicate template id {}", t.id);
        assert!(CATEGORIES.iter().any(|c| c.id == t.category.id), "{} has no category", t.id);
        assert!(!t.name(Lang::En).is_empty() && !t.name(Lang::Es).is_empty(), "{}", t.id);
        assert!(!t.summary(Lang::En).is_empty() && !t.summary(Lang::Es).is_empty(), "{}", t.id);
    }
    assert!(TEMPLATES.len() >= 12);
    for c in CATEGORIES {
        assert!(TEMPLATES.iter().any(|t| t.category.id == c.id), "category {} is empty", c.id);
    }
}

#[test]
fn every_template_opens_in_both_languages_with_its_declared_artboards() {
    for lang in ["en", "es"] {
        for t in TEMPLATES {
            let mut s = Session::new();
            let r = s.execute("file.newFromGallery", &json!({"template": t.id, "lang": lang})).unwrap_or_else(|e| panic!("{} ({lang}): {e}", t.id));
            assert_eq!(r["template"], t.id);
            assert_eq!(r["lang"], lang);
            let d = &s.doc().unwrap().doc;
            assert_eq!(d.artboards.len(), t.artboards, "{} artboards", t.id);
            let a = d.artboards[0].rect;
            assert!((a.width() - t.width).abs() < 0.5 && (a.height() - t.height).abs() < 0.5, "{}: {a:?} vs {}x{}", t.id, t.width, t.height);
            assert!(d.node_count() > 5, "{} has art", t.id);
            assert!(!texts(&s).is_empty() || t.id == "pattern" || t.id == "app-icon", "{} has live text", t.id);
        }
    }
}

#[test]
fn artboards_declared_in_the_drawing_match_the_catalog() {
    for t in TEMPLATES {
        let pages = declared_pages(t.source);
        if pages.is_empty() {
            continue;
        }
        assert_eq!(pages.len(), t.artboards, "{}", t.id);
    }
}

#[test]
fn spanish_replaces_every_english_text_and_english_keeps_its_own() {
    for t in TEMPLATES {
        let en = t.svg(Lang::En);
        let es = t.svg(Lang::Es);
        for (from, to) in t.es {
            assert!(en.contains(&format!(">{}<", escape(from))), "{}: `{from}` is not in the drawing", t.id);
            assert!(es.contains(&format!(">{}<", escape(to))), "{}: `{to}` was not put in", t.id);
        }
    }
    let mut en = Session::new();
    en.execute("file.newFromGallery", &json!({"template": "poster", "lang": "en"})).unwrap();
    let mut es = Session::new();
    es.execute("file.newFromGallery", &json!({"template": "poster", "lang": "es"})).unwrap();
    assert_ne!(texts(&en), texts(&es));
}

#[test]
fn a_spanish_interface_gets_spanish_templates_by_default() {
    let mut s = Session::new();
    s.ui_language = Some("es-MX".into());
    let r = s.execute("file.templates", &json!({})).unwrap();
    assert_eq!(r["lang"], "es");
    let mut s = Session::new();
    s.ui_language = Some("ja".into());
    assert_eq!(s.execute("file.templates", &json!({})).unwrap()["lang"], "en");
}

#[test]
fn the_list_filters_by_category_and_names_the_size() {
    let mut s = Session::new();
    let all = s.execute("file.templates", &json!({"lang": "en"})).unwrap();
    let n = all["templates"].as_array().unwrap().len();
    assert_eq!(n, TEMPLATES.len());
    let social = s.execute("file.templates", &json!({"category": "social"})).unwrap();
    let list = social["templates"].as_array().unwrap();
    assert!(!list.is_empty() && list.len() < n);
    assert!(list.iter().all(|t| t["category"] == "social"));
    assert!(list.iter().all(|t| t["size"].as_str().is_some_and(|s| s.contains('×'))));
}

#[test]
fn bad_requests_are_refused_with_what_to_do() {
    let mut s = Session::new();
    for params in [
        json!({}),
        json!({"template": "nope"}),
        json!({"template": "poster", "lang": "xx"}),
        json!({"template": "poster", "lang": 3}),
        json!({"template": 5}),
    ] {
        let e = s.execute("file.newFromGallery", &params).unwrap_err().to_string();
        assert!(e.contains("file.newFromGallery"), "{e}");
    }
    assert!(s.execute("file.templates", &json!({"category": "nope"})).is_err());
    assert!(s.execute("file.templates", &json!({"lang": "fr"})).is_err());
    assert!(s.active().is_none(), "nothing was opened");
}

#[test]
fn the_document_is_new_and_untitled_and_can_be_named() {
    let mut s = Session::new();
    let r = s.execute("file.newFromGallery", &json!({"template": "business-card", "lang": "en"})).unwrap();
    assert!(r["title"].as_str().is_some_and(|t| t.starts_with("Untitled")), "{r}");
    let r = s.execute("file.newFromGallery", &json!({"template": "letterhead", "name": "  Acme  "})).unwrap();
    assert_eq!(r["title"], "Acme");
    assert_eq!(s.doc().unwrap().doc.title, "Acme");
}

#[test]
fn size_labels_read_in_the_template_units() {
    assert_eq!(find("social-post").map(|t| t.size_label()).as_deref(), Some("1080 × 1080 px"));
    assert_eq!(find("poster").map(|t| t.size_label()).as_deref(), Some("210 × 297 mm"));
    assert_eq!(find("business-card").map(|t| t.size_label()).as_deref(), Some("3.5 × 2 in"));
    assert!(find("POSTER ").is_some());
    assert!(find("").is_none());
}
