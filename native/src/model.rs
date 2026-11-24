//! Folio projects: the same version-one `.folio.json` that Electron Folio saves, checked the way its `validate()`
//! checks them. Fields Folio native doesn't use are kept, so a project survives a round trip through either app.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub name: String,
    /// "portrait" (1080 × 1350), "square" (1080 × 1080), or native Folio's "story" (1080 × 1920), "landscape"
    /// (1920 × 1080, 16:9) and "custom" (`custom_size`: a picture another app sent, edited at its own size).
    pub format: String,
    /// Width and height when the format is "custom".
    #[serde(default, rename = "customSize", skip_serializing_if = "Option::is_none")]
    pub custom_size: Option<[u32; 2]>,
    /// A picture another app sent to be edited here, and where Save hands it back.
    #[serde(default, rename = "pictureEdit", skip_serializing_if = "Option::is_none")]
    pub picture_edit: Option<PictureEdit>,
    /// "light" or "dark".
    pub theme: String,
    pub accent: String,
    pub brand: String,
    pub handle: String,
    /// Arial, Georgia, Verdana or Trebuchet MS.
    pub font: String,
    /// "rtl" lays slides out for Arabic and other right-to-left copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    pub slides: Vec<Slide>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A picture sent from another app to edit ("edit-picture:" documents, see `app::PictureRequest`): the file it came
/// from, the file Save writes the edited picture to, and the app to hand that back to with the reference it gave.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PictureEdit {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    pub app: String,
    /// The sender's own reference, given back as it came (StreamCuts: its project and the picture's media).
    #[serde(default)]
    pub reply: String,
}

/// The sizes a "custom" project may have: what a picture can be, within reason.
pub const CUSTOM_SIZES: std::ops::RangeInclusive<u32> = 16..=8000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slide {
    #[serde(default)]
    pub id: String,
    pub layout: String,
    pub eyebrow: String,
    pub title: String,
    pub body: String,
    pub items: Vec<String>,
    /// A data URL, or empty.
    pub image: String,
    pub image2: String,
    #[serde(default)]
    pub layers: Vec<Layer>,
    pub title_size: f32,
    pub texture: bool,
    pub swipe: bool,
    /// A blank slide's own background color; without one it takes the theme's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// No fill behind the slide at all (native Folio only): PNG and WebP exports keep it see-through, JPG fills it
    /// with [`Slide::fill`]'s color, and the editor shows a checkerboard there.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub transparent: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A movable element on a slide: a picture (`src`, a data URL), text, or a shape. Electron Folio knows pictures and
/// text inside the slide; shapes and the styling below (`shape` onward) are native Folio's, and a project that uses
/// them no longer opens in Electron Folio.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    /// A shape's outline: "rect", "ellipse", "triangle" or "star". Its fill is `color`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    /// Rounded corners, for a rectangle or a picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
    /// Degrees clockwise around the element's middle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f32>,
    /// 0 (invisible) to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    /// Text: "left", "center" or "right"; without one it follows the project's direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// Text: bold unless this says otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    /// Text: its own font; without one, the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// A shape's outline color and width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f32>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Layer {
    pub fn rotation(&self) -> f32 {
        self.rotation.filter(|r| r.is_finite()).unwrap_or(0.0)
    }
    pub fn opacity(&self) -> f32 {
        self.opacity.filter(|o| o.is_finite()).unwrap_or(1.0).clamp(0.0, 1.0)
    }
    /// The middle of its box, which it turns around.
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
    /// What the element list calls it.
    pub fn label(&self) -> String {
        match self.kind.as_str() {
            "text" => self.text.as_deref().filter(|t| !t.trim().is_empty()).unwrap_or("Text").lines().next().unwrap_or("Text").chars().take(28).collect(),
            "shape" => match self.shape.as_deref() {
                Some("ellipse") => "Ellipse".into(),
                Some("triangle") => "Triangle".into(),
                Some("star") => "Star".into(),
                _ => "Rectangle".into(),
            },
            _ => "Picture".into(),
        }
    }
}

pub const SHAPES: [(&str, &str); 4] = [("rect", "Rectangle"), ("ellipse", "Ellipse"), ("triangle", "Triangle"), ("star", "Star")];

/// Fonts a text element can have of its own: the brand fonts and a few more that Windows ships.
pub const LAYER_FONTS: [&str; 10] = ["Arial", "Georgia", "Verdana", "Trebuchet MS", "Segoe UI", "Calibri", "Times New Roman", "Courier New", "Impact", "Comic Sans MS"];

pub const MAX_LAYERS: usize = 100;

/// Electron Folio's ten layouts, and native Folio's blank one: id, name, icon, and what it's for.
pub const LAYOUTS: [(&str, &str, &str, &str); 11] = [
    (
        "introduction",
        "Introduction",
        "↗",
        "Make a first impression",
    ),
    ("text", "Text only", "T", "Give your idea some space"),
    ("checklist", "Checklist", "✓", "Make the next steps clear"),
    ("quote", "Quote", "“", "A thought worth saving"),
    (
        "image-center",
        "Image center",
        "▣",
        "Let your image do the talking",
    ),
    (
        "image-left",
        "Image left",
        "◧",
        "A little balance goes a long way",
    ),
    ("image-right", "Image right", "◨", "Show and tell"),
    ("compare", "Image compare", "⊞", "Two sides of a story"),
    ("thank-you", "Thank you", "♡", "Keep the conversation going"),
    (
        "navy-ribbon",
        "Navy ribbon",
        "≋",
        "A continuous story, inspired by Untitled.png",
    ),
    ("blank", "Blank", "□", "An empty slide for your own text, shapes and pictures"),
];

/// Every slide size: id, name, width, height.
pub const FORMATS: [(&str, &str, f32, f32); 4] = [
    ("portrait", "Portrait", 1080.0, 1350.0),
    ("square", "Square", 1080.0, 1080.0),
    ("story", "Story", 1080.0, 1920.0),
    ("landscape", "Landscape", 1920.0, 1080.0),
];

pub const FONTS: [&str; 4] = ["Arial", "Georgia", "Verdana", "Trebuchet MS"];

const IMAGE_URL: [&str; 3] = [
    "data:image/png;base64,",
    "data:image/jpeg;base64,",
    "data:image/webp;base64,",
];

fn image_url(value: &str) -> bool {
    value.is_empty()
        || IMAGE_URL.iter().any(|prefix| {
            value.strip_prefix(prefix).is_some_and(|rest| {
                rest.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
            })
        })
}

fn hex_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

impl Project {
    fn size(&self) -> (f32, f32) {
        if let (true, Some([w, h])) = (self.format == "custom", self.custom_size) {
            return (w as f32, h as f32);
        }
        FORMATS.iter().find(|f| f.0 == self.format).map_or((1080.0, 1350.0), |f| (f.2, f.3))
    }

    pub fn width(&self) -> f32 {
        self.size().0
    }

    pub fn height(&self) -> f32 {
        self.size().1
    }

    /// Reads a project, refusing what Electron Folio's `validate()` refuses.
    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        let project: Self = serde_json::from_slice(bytes)
            .map_err(|_| "This is not a supported Folio project.".to_owned())?;
        project.validate()?;
        Ok(project)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.slides.is_empty()
            || self.slides.len() > 20
            || !(FORMATS.iter().any(|f| f.0 == self.format)
                || self.format == "custom" && self.custom_size.is_some_and(|s| s.iter().all(|n| CUSTOM_SIZES.contains(n))))
            || !["light", "dark"].contains(&self.theme.as_str())
        {
            return Err("This is not a supported Folio project.".into());
        }
        if self
            .direction
            .as_deref()
            .is_some_and(|d| d != "ltr" && d != "rtl")
        {
            return Err("Invalid project text direction.".into());
        }
        if !hex_color(&self.accent) || !FONTS.contains(&self.font.as_str()) {
            return Err("Project brand settings are invalid.".into());
        }
        if [&self.name, &self.brand, &self.handle]
            .iter()
            .any(|s| s.chars().count() > 100)
        {
            return Err("Invalid project text.".into());
        }
        let (width, height) = (self.width(), self.height());
        for slide in &self.slides {
            if slide.background.as_deref().is_some_and(|c| !hex_color(c)) {
                return Err("Invalid slide background.".into());
            }
            if !LAYOUTS.iter().any(|l| l.0 == slide.layout)
                || !(44.0..=120.0).contains(&slide.title_size)
            {
                return Err("Invalid slide layout.".into());
            }
            if [&slide.eyebrow, &slide.title, &slide.body]
                .iter()
                .any(|s| s.chars().count() > 1500)
            {
                return Err("Invalid slide text.".into());
            }
            if slide.items.len() > 5 || slide.items.iter().any(|t| t.chars().count() > 1000) {
                return Err("Invalid checklist.".into());
            }
            if !image_url(&slide.image) || !image_url(&slide.image2) {
                return Err("Invalid slide image.".into());
            }
            if slide.layers.len() > MAX_LAYERS {
                return Err("Invalid slide elements.".into());
            }
            for layer in &slide.layers {
                // An element may hang over the slide's edge (a circle half off a corner), but not leave it.
                let numbers = [layer.x, layer.y, layer.width, layer.height];
                if numbers.iter().any(|n| !n.is_finite())
                    || layer.width < 2.0
                    || layer.height < 2.0
                    || layer.width > 8000.0
                    || layer.height > 8000.0
                    || layer.x + layer.width <= 0.0
                    || layer.y + layer.height <= 0.0
                    || layer.x >= width
                    || layer.y >= height
                {
                    return Err("Element is outside the slide.".into());
                }
                let styled = layer.rotation.is_none_or(f32::is_finite)
                    && layer.opacity.is_none_or(|o| (0.0..=1.0).contains(&o))
                    && layer.radius.is_none_or(|r| (0.0..=4000.0).contains(&r))
                    && layer.align.as_deref().is_none_or(|a| ["left", "center", "right"].contains(&a))
                    && layer.font.as_deref().is_none_or(|f| LAYER_FONTS.contains(&f))
                    && layer.stroke.as_deref().is_none_or(hex_color)
                    && layer.stroke_width.is_none_or(|w| (0.0..=200.0).contains(&w));
                if !styled {
                    return Err("Invalid slide element.".into());
                }
                let valid = match layer.kind.as_str() {
                    "image" => layer
                        .src
                        .as_deref()
                        .is_some_and(|s| !s.is_empty() && image_url(s)),
                    "text" => {
                        layer
                            .text
                            .as_ref()
                            .is_some_and(|t| t.chars().count() <= 2000)
                            && layer.color.as_deref().is_some_and(hex_color)
                            && layer.font_size.is_some_and(|s| (8.0..=600.0).contains(&s))
                    }
                    "shape" => {
                        layer.shape.as_deref().is_some_and(|s| SHAPES.iter().any(|known| known.0 == s))
                            && layer.color.as_deref().is_some_and(hex_color)
                    }
                    _ => false,
                };
                if !valid {
                    return Err("Invalid slide element.".into());
                }
            }
        }
        Ok(())
    }

    /// Electron Folio's starting project, "The art of a good carousel".
    pub fn starter(new_id: &mut impl FnMut() -> String) -> Self {
        let slide = |layout: &str,
                     eyebrow: Option<&str>,
                     title: &str,
                     body: &str,
                     new_id: &mut dyn FnMut() -> String| Slide {
            eyebrow: eyebrow.unwrap_or("THE CREATIVE NOTES").into(),
            title: title.into(),
            body: body.into(),
            ..Slide::new(layout, new_id())
        };
        let slides = vec![
            slide(
                "introduction",
                Some("A NOTE FOR THE CREATORS"),
                "Good design.\nGreat stories.",
                "Turn your next big idea into something\nworth swiping for.",
                new_id,
            ),
            slide(
                "text",
                None,
                "Start with\nsomething real.",
                "The best carousels begin with an idea your audience cares about.\n\nTeach something. Share a perspective. Tell a story only you can tell.",
                new_id,
            ),
            slide(
                "checklist",
                None,
                "Make every\nslide count.",
                "A few small things make a big difference.",
                new_id,
            ),
            slide(
                "quote",
                Some("A THOUGHT WORTH SAVING"),
                "Great things are done by a series of small things brought together.",
                "— Vincent van Gogh",
                new_id,
            ),
            slide(
                "thank-you",
                Some("THAT’S A WRAP"),
                "Good ideas\nare better shared.",
                "Save this for your next creative day.\nFollow along for a little more inspiration.",
                new_id,
            ),
        ];
        Self {
            version: 1,
            name: "The art of a good carousel".into(),
            format: "portrait".into(),
            theme: "light".into(),
            accent: "#e65443".into(),
            brand: "YOUR STUDIO".into(),
            handle: "@yourstudio".into(),
            font: "Arial".into(),
            direction: None,
            custom_size: None,
            picture_edit: None,
            slides,
            extra: Map::new(),
        }
    }
}

impl Slide {
    /// A slide like Electron Folio's `createSlide(layout)`.
    pub fn new(layout: &str, id: String) -> Self {
        Self {
            id,
            layout: layout.into(),
            eyebrow: "THE CREATIVE NOTES".into(),
            title: "One idea.\nA little possibility.".into(),
            body: "Share something useful. Make it simple, make it thoughtful, make it yours."
                .into(),
            items: vec![
                "Start with a clear idea".into(),
                "Make every slide count".into(),
                "Finish with a next step".into(),
            ],
            image: String::new(),
            image2: String::new(),
            layers: Vec::new(),
            title_size: 88.0,
            texture: true,
            swipe: true,
            background: None,
            transparent: false,
            extra: Map::new(),
        }
    }
}

impl Project {
    /// What fills `slide` when it isn't see-through: its own color on a blank slide, navy on the navy ribbon,
    /// otherwise the theme's background. JPG exports put transparent slides on this.
    pub fn fill<'a>(&self, slide: &'a Slide) -> &'a str {
        let theme = if self.theme == "dark" { "#151716" } else { "#fff2ed" };
        match slide.layout.as_str() {
            "blank" => slide.background.as_deref().unwrap_or(theme),
            "navy-ribbon" => "#3c415d",
            _ => theme,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> impl FnMut() -> String {
        let mut n = 0;
        move || {
            n += 1;
            format!("id-{n}")
        }
    }

    #[test]
    fn projects_round_trip_and_keep_unknown_fields() {
        let mut project = Project::starter(&mut ids());
        project
            .extra
            .insert("folderFileName".into(), "art--1.folio.json".into());
        project.slides[0].extra.insert("note".into(), 5.into());
        let bytes = serde_json::to_vec(&project).unwrap();
        let read = Project::read(&bytes).unwrap();
        assert_eq!(read, project);
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["slides"][0]["titleSize"], 88.0);
        assert!(json.get("direction").is_none());
    }

    #[test]
    fn what_electron_refuses_is_refused() {
        let mut project = Project::starter(&mut ids());
        project.font = "Comic Sans MS".into();
        assert!(project.validate().is_err());
        let mut project = Project::starter(&mut ids());
        let text = Layer {
            kind: "text".into(),
            x: 1000.0,
            y: 0.0,
            width: 200.0,
            height: 50.0,
            text: Some("x".into()),
            color: Some("#000000".into()),
            font_size: Some(40.0),
            ..Layer::default()
        };
        // Over the edge is fine; off the slide isn't.
        project.slides[0].layers.push(text.clone());
        assert_eq!(project.validate(), Ok(()));
        project.slides[0].layers[0].x = 1080.0;
        assert_eq!(
            project.validate(),
            Err("Element is outside the slide.".into())
        );
        project.slides[0].layers[0] = Layer { kind: "shape".into(), shape: Some("blob".into()), ..text.clone() };
        assert_eq!(project.validate(), Err("Invalid slide element.".into()));
        project.slides[0].layers[0] = Layer { kind: "shape".into(), shape: Some("star".into()), rotation: Some(30.0), opacity: Some(0.5), ..text.clone() };
        assert_eq!(project.validate(), Ok(()));
        project.slides[0].layers[0].opacity = Some(1.5);
        assert!(project.validate().is_err());
        project.slides[0].layout = "blank".into();
        project.slides[0].layers.clear();
        project.format = "story".into();
        assert_eq!((project.validate(), project.height()), (Ok(()), 1920.0));
        project.format = "landscape".into();
        assert_eq!((project.validate(), project.width(), project.height()), (Ok(()), 1920.0, 1080.0));
        // A landscape slide is wider, so an element further right is still on it.
        project.slides[0].layers.push(Layer { x: 1500.0, ..text.clone() });
        assert_eq!(project.validate(), Ok(()));
        project.slides[0].layers[0].x = 1920.0;
        assert!(project.validate().is_err());
        project.format = "widescreen".into();
        assert!(project.validate().is_err());
        // A picture's own size: only with a size, and a sensible one.
        project.format = "custom".into();
        assert!(project.validate().is_err());
        project.custom_size = Some([3000, 1700]);
        assert_eq!((project.validate(), project.width(), project.height()), (Ok(()), 3000.0, 1700.0));
        project.custom_size = Some([9000, 1700]);
        assert!(project.validate().is_err());
        let mut project = Project::starter(&mut ids());
        project.slides[1].image = "data:image/gif;base64,AAAA".into();
        assert!(project.validate().is_err());
    }
}
