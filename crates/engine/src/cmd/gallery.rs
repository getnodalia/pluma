//! File → Template Gallery: the starting documents Pluma ships, each a calm, finished piece of art
//! made for Pluma (a business card, a poster, a social post, a brand board…), with its text in
//! English and Spanish. A template is an original SVG drawing kept next to this file (`gallery/`):
//! `file.newFromGallery` reads it as a new untitled document, so every shape, gradient and line of
//! type is live and editable, and the layers carry the names the SVG gives its groups.
//!
//! `file.templates` lists them by category for the gallery dialog and the Home screen. Both
//! commands take `lang` (`en` or `es`; default: the language the interface is drawn in, English
//! for any other), which picks the names, the summaries and the text inside the artwork.
//!
//! The artwork's English text is the key of its Spanish table: the SVG holds the English, and
//! [`Template::svg`] swaps each text node for its Spanish one. A test checks that every text node
//! of every template is either in its table or a name, address or number that stays as it is.

use serde_json::{Value, json};

use super::{CommandSpec, Result, Session, always, bad, cmd, str_param};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "file.templates",
            "Template Gallery",
            [],
            None,
            "{lang?: \"en\"|\"es\" (default: the interface language, English for others), category?: one of `categories`'s ids} the built-in templates → {lang, categories: [{id, name}], templates: [{id, category, name, summary, width, height (pt, one artboard), size: label such as \"210 × 297 mm\", artboards}]}; file.newFromGallery starts from one",
            always,
            list
        ),
        cmd!(
            "file.newFromGallery",
            "New from Gallery",
            [],
            None,
            "{template: an id from file.templates, lang?: \"en\"|\"es\" (default: the interface language), name?|title?: document name (default Untitled-N)} open a built-in template as a new untitled document, with its art live and editable → {index, title, template, lang, warnings}",
            always,
            new_from_gallery
        ),
    ]
}

/// The languages the templates' text is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
}

impl Lang {
    /// The language of a code (`es`, `es-MX`, `en`…): Spanish for Spanish, English for everything
    /// else, as the interface does for a language the templates are not written in.
    pub fn from_code(code: &str) -> Self {
        let primary = code.split(['-', '_']).next().unwrap_or("");
        if primary.eq_ignore_ascii_case("es") { Self::Es } else { Self::En }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
        }
    }

    fn pick(self, en: &'static str, es: &'static str) -> &'static str {
        match self {
            Self::En => en,
            Self::Es => es,
        }
    }
}

/// A group of templates in the gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Category {
    pub id: &'static str,
    en: &'static str,
    es: &'static str,
}

impl Category {
    pub fn name(self, lang: Lang) -> &'static str {
        lang.pick(self.en, self.es)
    }
}

const PRINT: Category = Category { id: "print", en: "Print", es: "Impresión" };
const BRANDING: Category = Category { id: "branding", en: "Branding", es: "Identidad de marca" };
const SOCIAL: Category = Category { id: "social", en: "Social & Web", es: "Redes y web" };

/// The categories, in the order the gallery shows them.
pub const CATEGORIES: [Category; 3] = [PRINT, BRANDING, SOCIAL];

/// How a template's size is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Units {
    Pixels,
    Millimeters,
    Inches,
}

/// A built-in template.
pub struct Template {
    /// The stable id scripts and the UI use (English, kebab-case).
    pub id: &'static str,
    pub category: Category,
    name: (&'static str, &'static str),
    summary: (&'static str, &'static str),
    /// One artboard's size in points.
    pub width: f64,
    pub height: f64,
    units: Units,
    /// How many artboards the document has.
    pub artboards: usize,
    source: &'static str,
    /// Each English text of the artwork and its Spanish.
    es: &'static [(&'static str, &'static str)],
}

impl Template {
    pub fn name(&self, lang: Lang) -> &'static str {
        lang.pick(self.name.0, self.name.1)
    }

    pub fn summary(&self, lang: Lang) -> &'static str {
        lang.pick(self.summary.0, self.summary.1)
    }

    /// The size of one artboard as the gallery shows it: `210 × 297 mm`, `1080 × 1080 px`.
    pub fn size_label(&self) -> String {
        let number = |v: f64| {
            let r = (v * 10.0).round() / 10.0;
            if (r - r.round()).abs() < 1e-9 { format!("{}", r as i64) } else { format!("{r}") }
        };
        match self.units {
            Units::Pixels => format!("{} × {} px", number(self.width), number(self.height)),
            // Paper sizes in points (595 × 842 for A4) are a fraction of a millimetre off: a value
            // within a quarter of a millimetre of a whole one reads as that one.
            Units::Millimeters => {
                let mm = |pt: f64| {
                    let v = pt * 25.4 / 72.0;
                    if (v - v.round()).abs() < 0.25 { v.round() } else { v }
                };
                format!("{} × {} mm", number(mm(self.width)), number(mm(self.height)))
            }
            Units::Inches => format!("{} × {} in", number(self.width / 72.0), number(self.height / 72.0)),
        }
    }

    /// The drawing in `lang`: the SVG, with each text swapped for its Spanish one in Spanish.
    pub fn svg(&self, lang: Lang) -> String {
        match lang {
            Lang::En => self.source.to_string(),
            Lang::Es => {
                let mut svg = self.source.to_string();
                for (en, es) in self.es {
                    svg = svg.replace(&format!(">{}<", escape(en)), &format!(">{}<", escape(es)));
                }
                svg
            }
        }
    }

    fn to_json(&self, lang: Lang) -> Value {
        json!({
            "id": self.id,
            "category": self.category.id,
            "name": self.name(lang),
            "summary": self.summary(lang),
            "width": self.width,
            "height": self.height,
            "size": self.size_label(),
            "artboards": self.artboards,
        })
    }
}

/// `text` as it is written inside an SVG element.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Every template, in the order the gallery shows them.
pub static TEMPLATES: &[Template] = &[
    Template {
        id: "poster",
        category: PRINT,
        name: ("Evening light poster", "Póster «Luz de la tarde»"),
        summary: ("Layered hills under a low sun, with room for a title.", "Colinas en capas bajo un sol bajo, con espacio para el título."),
        width: 595.0,
        height: 842.0,
        units: Units::Millimeters,
        artboards: 1,
        source: include_str!("gallery/poster.svg"),
        es: &[
            ("OPEN AIR SERIES", "SERIE AL AIRE LIBRE"),
            ("Evening Light", "Luz de la tarde"),
            ("A quiet walk through the valley", "Un paseo tranquilo por el valle"),
        ],
    },
    Template {
        id: "event-flyer",
        category: PRINT,
        name: ("Event flyer", "Cartel de evento"),
        summary: ("An arched window onto the sunset, with date, time and place.", "Una ventana en arco al atardecer, con fecha, hora y lugar."),
        width: 420.0,
        height: 595.0,
        units: Units::Millimeters,
        artboards: 1,
        source: include_str!("gallery/event-flyer.svg"),
        es: &[
            ("AN EVENING WITH FRIENDS", "UNA TARDE ENTRE AMIGOS"),
            ("Open Studio", "Estudio abierto"),
            ("Art, music and good company", "Arte, música y buena compañía"),
            ("DATE", "FECHA"),
            ("Saturday", "Sábado"),
            ("TIME", "HORA"),
            ("7 pm", "19:00"),
            ("PLACE", "LUGAR"),
            ("12 Garden Street", "Calle del Jardín 12"),
        ],
    },
    Template {
        id: "letterhead",
        category: PRINT,
        name: ("Letterhead", "Papel con membrete"),
        summary: (
            "A quiet header, a letter ready to write and a soft footer.",
            "Un encabezado sereno, una carta lista para escribir y un pie suave.",
        ),
        width: 595.0,
        height: 842.0,
        units: Units::Millimeters,
        artboards: 1,
        source: include_str!("gallery/letterhead.svg"),
        es: &[
            ("Design and illustration", "Diseño e ilustración"),
            ("12 Garden Street", "Calle del Jardín 12"),
            ("Your City, 00000", "Tu ciudad, 00000"),
            ("1 January", "1 de enero"),
            ("Recipient name", "Nombre del destinatario"),
            ("Company or address", "Empresa o dirección"),
            ("City, 00000", "Ciudad, 00000"),
            ("Subject of your letter", "Asunto de tu carta"),
            ("Dear name,", "Estimado/a nombre:"),
            (
                "Write your message here. This letterhead keeps the page calm: a quiet header,",
                "Escribe aquí tu mensaje. Este membrete mantiene la página serena: un encabezado discreto,",
            ),
            (
                "generous margins and a footer with your details. Replace this text, or delete it",
                "márgenes amplios y un pie con tus datos. Sustituye este texto, o bórralo",
            ),
            ("and paste your own.", "y pega el tuyo."),
            (
                "Keep paragraphs short and leave a little air between them, so the letter reads",
                "Mantén los párrafos cortos y deja un poco de aire entre ellos, para que la carta",
            ),
            ("easily at a glance.", "se lea con facilidad de un vistazo."),
            ("Kind regards,", "Un cordial saludo,"),
            ("Your Name", "Tu nombre"),
            ("Creative Director", "Dirección creativa"),
        ],
    },
    Template {
        id: "infographic",
        category: PRINT,
        name: ("Year in numbers", "Un año en cifras"),
        summary: (
            "Four ring gauges and a bar chart on a calm summary page.",
            "Cuatro indicadores en anillo y un gráfico de barras en una página de resumen.",
        ),
        width: 595.0,
        height: 842.0,
        units: Units::Millimeters,
        artboards: 1,
        source: include_str!("gallery/infographic.svg"),
        es: &[
            ("ANNUAL SUMMARY", "RESUMEN ANUAL"),
            ("A year in numbers", "Un año en cifras"),
            ("A calm, clear snapshot of what we made and how it went.", "Una imagen serena y clara de lo que hicimos y cómo fue."),
            ("Projects delivered on time", "Proyectos entregados a tiempo"),
            ("New clients welcomed", "Clientes nuevos"),
            ("Illustrations published", "Ilustraciones publicadas"),
            ("Average client rating", "Valoración media de clientes"),
            ("PROJECTS PER MONTH", "PROYECTOS POR MES"),
            ("Jan", "Ene"),
            ("Apr", "Abr"),
        ],
    },
    Template {
        id: "business-card",
        category: BRANDING,
        name: ("Business card", "Tarjeta de visita"),
        summary: (
            "Front and back on two artboards, in an arch-and-sun mark.",
            "Anverso y reverso en dos mesas de trabajo, con una marca de arco y sol.",
        ),
        width: 252.0,
        height: 144.0,
        units: Units::Inches,
        artboards: 2,
        source: include_str!("gallery/business-card.svg"),
        es: &[("Your Name", "Tu nombre"), ("CREATIVE DIRECTOR", "DIRECCIÓN CREATIVA")],
    },
    Template {
        id: "brand-board",
        category: BRANDING,
        name: ("Brand board", "Panel de marca"),
        summary: ("Logo, six colours and a type specimen on one page.", "Logotipo, seis colores y una muestra tipográfica en una página."),
        width: 1600.0,
        height: 1000.0,
        units: Units::Pixels,
        artboards: 1,
        source: include_str!("gallery/brand-board.svg"),
        es: &[
            ("BRAND BOARD", "PANEL DE MARCA"),
            ("Logo, colour and type in one calm page", "Logotipo, color y tipografía en una página serena"),
            ("Design and illustration", "Diseño e ilustración"),
            ("Ink", "Tinta"),
            ("Teal", "Mar"),
            ("Sage", "Salvia"),
            ("Sand", "Arena"),
            ("Apricot", "Albaricoque"),
            ("Clay", "Arcilla"),
            ("HEADLINE", "TITULAR"),
            ("Quiet, beautiful work", "Trabajo sereno y bello"),
            ("BODY", "TEXTO"),
            ("We make calm, considered design for people who", "Hacemos un diseño sereno y cuidado para quienes"),
            ("value clarity: identities, illustrations and print.", "valoran la claridad: identidades, ilustraciones e impresos."),
            ("Regular 400  ·  Medium 500  ·  Semibold 600", "Regular 400  ·  Media 500  ·  Seminegrita 600"),
        ],
    },
    Template {
        id: "app-icon",
        category: BRANDING,
        name: ("App icon", "Icono de aplicación"),
        summary: (
            "A rounded tile with a centred mark and construction guides.",
            "Un icono redondeado con una marca centrada y guías de construcción.",
        ),
        width: 1024.0,
        height: 1024.0,
        units: Units::Pixels,
        artboards: 1,
        source: include_str!("gallery/app-icon.svg"),
        es: &[],
    },
    Template {
        id: "pattern",
        category: BRANDING,
        name: ("Arches pattern", "Motivo de arcos"),
        summary: ("A seamless tile of arches, rings and dots to repeat.", "Una baldosa continua de arcos, anillos y puntos para repetir."),
        width: 600.0,
        height: 600.0,
        units: Units::Pixels,
        artboards: 1,
        source: include_str!("gallery/pattern.svg"),
        es: &[],
    },
    Template {
        id: "social-post",
        category: SOCIAL,
        name: ("Quote post", "Publicación con cita"),
        summary: ("A square quote card with nested arches.", "Una tarjeta cuadrada con una cita y arcos anidados."),
        width: 1080.0,
        height: 1080.0,
        units: Units::Pixels,
        artboards: 1,
        source: include_str!("gallery/social-post.svg"),
        es: &[
            ("Make something", "Crea algo"),
            ("quiet and beautiful", "sereno y bello"),
            ("today.", "hoy."),
            ("Your Name", "Tu nombre"),
            ("Creative Director", "Dirección creativa"),
        ],
    },
    Template {
        id: "social-story",
        category: SOCIAL,
        name: ("Story announcement", "Historia de anuncio"),
        summary: ("A tall story with a big headline over a setting sun.", "Una historia vertical con un gran titular sobre un sol poniente."),
        width: 1080.0,
        height: 1920.0,
        units: Units::Pixels,
        artboards: 1,
        source: include_str!("gallery/social-story.svg"),
        es: &[
            ("NEW COLLECTION", "NUEVA COLECCIÓN"),
            ("Soft light,", "Luz suave,"),
            ("slow days", "días lentos"),
            ("Prints and posters for quiet rooms", "Láminas y pósteres para habitaciones tranquilas"),
            ("See the collection", "Ver la colección"),
        ],
    },
    Template {
        id: "video-thumbnail",
        category: SOCIAL,
        name: ("Video thumbnail", "Miniatura de vídeo"),
        summary: (
            "A 16:9 cover with a bold title beside a round landscape.",
            "Una portada 16:9 con un título contundente junto a un paisaje redondo.",
        ),
        width: 1280.0,
        height: 720.0,
        units: Units::Pixels,
        artboards: 1,
        source: include_str!("gallery/video-thumbnail.svg"),
        es: &[
            ("TUTORIAL", "TUTORIAL"),
            ("Paint a calm", "Pinta un paisaje"),
            ("landscape", "sereno"),
            ("in just ten minutes", "en solo diez minutos"),
        ],
    },
    Template {
        id: "web-banner",
        category: SOCIAL,
        name: ("Website banner", "Banner web"),
        summary: ("A wide hero with headline, short text and two buttons.", "Una portada ancha con titular, texto breve y dos botones."),
        width: 1440.0,
        height: 560.0,
        units: Units::Pixels,
        artboards: 1,
        source: include_str!("gallery/web-banner.svg"),
        es: &[
            ("Design that feels", "Un diseño que se siente"),
            ("calm and clear", "sereno y claro"),
            ("Brand identity, illustration and print for people", "Identidad de marca, ilustración e impresos para quienes"),
            ("who prefer quiet, beautiful work.", "prefieren un trabajo sereno y bello."),
            ("See our work", "Ver nuestro trabajo"),
            ("Get in touch  →", "Contacta con nosotros  →"),
        ],
    },
];

/// The template with this id.
pub fn find(id: &str) -> Option<&'static Template> {
    let id = id.trim();
    TEMPLATES.iter().find(|t| t.id.eq_ignore_ascii_case(id))
}

/// The language the templates are written in by default: the one the interface is drawn in.
pub fn language(s: &Session) -> Lang {
    Lang::from_code(s.ui_language.as_deref().unwrap_or(&s.prefs.interface_language))
}

/// The language a request asks for: its `lang`, else the one the interface is drawn in.
fn lang_of(s: &Session, p: &Value, command: &str) -> Result<Lang> {
    match p.get("lang") {
        None | Some(Value::Null) => Ok(language(s)),
        Some(Value::String(code)) => match code.trim().to_ascii_lowercase().as_str() {
            "en" => Ok(Lang::En),
            "es" => Ok(Lang::Es),
            other => Err(bad(command, format!("no templates in `{other}` (use `en` or `es`)"))),
        },
        Some(_) => Err(bad(command, "`lang` must be a string (`en` or `es`)")),
    }
}

fn list(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "file.templates";
    let lang = lang_of(s, p, C)?;
    let only = match str_param(p, "category") {
        None => None,
        Some(id) => Some(
            CATEGORIES
                .iter()
                .find(|c| c.id.eq_ignore_ascii_case(id.trim()))
                .ok_or_else(|| bad(C, format!("no category `{id}` ({})", CATEGORIES.map(|c| c.id).join(", "))))?,
        ),
    };
    let templates: Vec<Value> = TEMPLATES.iter().filter(|t| only.is_none_or(|c| c.id == t.category.id)).map(|t| t.to_json(lang)).collect();
    let categories: Vec<Value> = CATEGORIES.iter().map(|c| json!({ "id": c.id, "name": c.name(lang) })).collect();
    Ok(json!({ "lang": lang.code(), "categories": categories, "templates": templates }))
}

fn new_from_gallery(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "file.newFromGallery";
    let id = str_param(p, "template").ok_or_else(|| bad(C, "give the template's id (see file.templates)"))?;
    let template = find(id).ok_or_else(|| bad(C, format!("no template called `{id}` (see file.templates)")))?;
    let lang = lang_of(s, p, C)?;
    let svg = template.svg(lang);
    let mut opened = super::fileio::open_template(s, &format!("{}.svg", template.id), svg.as_bytes(), None)?;
    if let Some(name) = str_param(p, "name").or(str_param(p, "title")).map(str::trim).filter(|n| !n.is_empty()) {
        std::sync::Arc::make_mut(&mut s.doc_mut()?.doc).title = name.to_string();
        opened["title"] = json!(name);
    }
    opened["template"] = json!(template.id);
    opened["lang"] = json!(lang.code());
    Ok(opened)
}

/// The artboards of a template's SVG, as its `<inkscape:page>` rectangles say (the whole drawing
/// when it has none): what the tests compare with [`Template::width`] and [`Template::height`].
#[cfg(test)]
fn declared_pages(svg: &str) -> Vec<pluma_geom::Rect> {
    let mut pages = vec![];
    for tag in svg.split("<inkscape:page ").skip(1) {
        let attr = |name: &str| -> f64 {
            let key = format!("{name}=\"");
            tag.find(&key).and_then(|i| tag[i + key.len()..].split('"').next()).and_then(|v| v.parse().ok()).unwrap_or(0.0)
        };
        let (x, y) = (attr(" x"), attr(" y"));
        pages.push(pluma_geom::Rect::new(x, y, x + attr(" width"), y + attr(" height")));
    }
    pages
}

#[cfg(test)]
mod tests;
