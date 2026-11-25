//! Thumbnails of the Template Gallery's templates (the gallery dialog and the Home screen): a
//! template's drawing is read and drawn on a thread of its own (never the window's), at most
//! [`SIZE`] on its longer side, once per template and language. The web build has no threads, so
//! it draws one thumbnail per frame instead.

use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex, PoisonError};

use pluma_engine::cmd::gallery::{Lang, Template};

/// The longer side of a thumbnail, in pixels.
const SIZE: f64 = 360.0;

/// A template in a language.
type Key = (&'static str, &'static str);

/// Thumbnails drawn and not uploaded yet (`None`: the template could not be drawn).
#[cfg(not(target_arch = "wasm32"))]
type Finished = Arc<Mutex<Vec<(Key, Option<pluma_render::Rendered>)>>>;

#[derive(Default)]
struct Store {
    /// `None`: asked for and not back yet, or not drawable.
    shown: HashMap<Key, Option<egui::TextureHandle>>,
    #[cfg(not(target_arch = "wasm32"))]
    done: Finished,
    /// The web build draws one per frame: this frame's turn is taken.
    #[cfg(target_arch = "wasm32")]
    drawn_this_frame: u64,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::default());
}

/// The thumbnail of `template` in `lang` (its texture and size) once it is drawn; the first call
/// starts it.
pub(crate) fn get(ctx: &egui::Context, template: &'static Template, lang: Lang) -> Option<(egui::TextureId, egui::Vec2)> {
    let key = (template.id, lang.code());
    STORE.with(|store| {
        let mut store = store.borrow_mut();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let done: Vec<_> = std::mem::take(&mut *store.done.lock().unwrap_or_else(PoisonError::into_inner));
            for (k, img) in done {
                let tex = img.map(|img| {
                    ctx.load_texture(format!("gallery-thumb {} {}", k.0, k.1), super::raster::color_image(&img), egui::TextureOptions::LINEAR)
                });
                store.shown.insert(k, tex);
            }
        }
        if let Some(tex) = store.shown.get(&key) {
            return tex.as_ref().map(|t| (t.id(), t.size_vec2()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            store.shown.insert(key, None);
            let done = Arc::clone(&store.done);
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new().name("gallery-thumb".into()).spawn(move || {
                let img = std::panic::catch_unwind(|| render(template, lang)).ok().flatten();
                done.lock().unwrap_or_else(PoisonError::into_inner).push((key, img));
                ctx.request_repaint();
            });
            // No thread: the card keeps its empty page.
            drop(spawned);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let pass = ctx.cumulative_pass_nr();
            if store.drawn_this_frame != pass {
                store.drawn_this_frame = pass;
                let tex = render(template, lang).map(|img| {
                    ctx.load_texture(format!("gallery-thumb {} {}", key.0, key.1), super::raster::color_image(&img), egui::TextureOptions::LINEAR)
                });
                let shown = tex.as_ref().map(|t| (t.id(), t.size_vec2()));
                store.shown.insert(key, tex);
                ctx.request_repaint();
                return shown;
            }
            ctx.request_repaint();
        }
        None
    })
}

/// Draw a template: its artboards (or its art, without any) on white.
fn render(template: &Template, lang: Lang) -> Option<pluma_render::Rendered> {
    let doc = pluma_svg::import(&template.svg(lang)).ok()?;
    let region = doc.artboards.iter().map(|a| a.rect).reduce(|a, b| a.union(b)).or_else(|| doc.art_bounds())?;
    let longer = region.width().max(region.height());
    if !longer.is_finite() || longer <= 0.0 {
        return None;
    }
    Some(pluma_render::Renderer::new().render_region(&doc, region, SIZE / longer, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluma_engine::cmd::gallery::TEMPLATES;

    #[test]
    fn every_template_is_drawn_in_both_languages_at_the_thumbnail_size() {
        for t in TEMPLATES {
            for lang in [Lang::En, Lang::Es] {
                let img = render(t, lang).unwrap_or_else(|| panic!("{} ({})", t.id, lang.code()));
                let longer = img.width.max(img.height);
                assert!(longer <= SIZE as u32 + 2 && longer + 2 >= SIZE as u32, "{}: {}×{}", t.id, img.width, img.height);
                // Not an empty page: something is drawn on it.
                let first = img.pixels.get(..4).map(<[u8]>::to_vec);
                assert!(img.pixels.as_chunks::<4>().0.iter().any(|p| Some(p.as_slice()) != first.as_deref()), "{} is blank", t.id);
            }
        }
    }
}
