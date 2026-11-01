//! Folio projects: the same version-one `.folio.json` that Electron Folio saves, checked the way its `validate()`
//! checks them. Fields Folio native doesn't use are kept, so a project survives a round trip through either app.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub name: String,
    /// "portrait" (1080 × 1350) or "square" (1080 × 1080).
    pub format: String,
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
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A movable element on a slide: a picture (`src`, a data URL) or text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Electron Folio's ten layouts: id, name, icon, and what it's for.
pub const LAYOUTS: [(&str, &str, &str, &str); 10] = [
    ("introduction", "Introduction", "↗", "Make a first impression"),
    ("text", "Text only", "T", "Give your idea some space"),
    ("checklist", "Checklist", "✓", "Make the next steps clear"),
    ("quote", "Quote", "“", "A thought worth saving"),
    ("image-center", "Image center", "▣", "Let your image do the talking"),
    ("image-left", "Image left", "◧", "A little balance goes a long way"),
    ("image-right", "Image right", "◨", "Show and tell"),
    ("compare", "Image compare", "⊞", "Two sides of a story"),
    ("thank-you", "Thank you", "♡", "Keep the conversation going"),
    ("navy-ribbon", "Navy ribbon", "≋", "A continuous story, inspired by Untitled.png"),
];

pub const FONTS: [&str; 4] = ["Arial", "Georgia", "Verdana", "Trebuchet MS"];

const IMAGE_URL: [&str; 3] = ["data:image/png;base64,", "data:image/jpeg;base64,", "data:image/webp;base64,"];

fn image_url(value: &str) -> bool {
    value.is_empty() || IMAGE_URL.iter().any(|prefix| value.strip_prefix(prefix).is_some_and(|rest| rest.bytes().all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))))
}

fn hex_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

impl Project {
    pub fn height(&self) -> f32 {
        if self.format == "square" { 1080.0 } else { 1350.0 }
    }

    /// Reads a project, refusing what Electron Folio's `validate()` refuses.
    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        let project: Self = serde_json::from_slice(bytes).map_err(|_| "This is not a supported Folio project.".to_owned())?;
        project.validate()?;
        Ok(project)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.slides.is_empty() || self.slides.len() > 20 || !["portrait", "square"].contains(&self.format.as_str()) || !["light", "dark"].contains(&self.theme.as_str()) {
            return Err("This is not a supported Folio project.".into());
        }
        if self.direction.as_deref().is_some_and(|d| d != "ltr" && d != "rtl") {
            return Err("Invalid project text direction.".into());
        }
        if !hex_color(&self.accent) || !FONTS.contains(&self.font.as_str()) {
            return Err("Project brand settings are invalid.".into());
        }
        if [&self.name, &self.brand, &self.handle].iter().any(|s| s.chars().count() > 100) {
            return Err("Invalid project text.".into());
        }
        let height = self.height() + 0.01;
        for slide in &self.slides {
            if !LAYOUTS.iter().any(|l| l.0 == slide.layout) || !(44.0..=120.0).contains(&slide.title_size) {
                return Err("Invalid slide layout.".into());
            }
            if [&slide.eyebrow, &slide.title, &slide.body].iter().any(|s| s.chars().count() > 1500) {
                return Err("Invalid slide text.".into());
            }
            if slide.items.len() > 5 || slide.items.iter().any(|t| t.chars().count() > 1000) {
                return Err("Invalid checklist.".into());
            }
            if !image_url(&slide.image) || !image_url(&slide.image2) {
                return Err("Invalid slide image.".into());
            }
            if slide.layers.len() > 30 {
                return Err("Invalid slide elements.".into());
            }
            for layer in &slide.layers {
                if layer.x < 0.0 || layer.y < 0.0 || layer.width < 20.0 || layer.height < 20.0 || layer.x + layer.width > 1080.01 || layer.y + layer.height > height {
                    return Err("Element is outside the slide.".into());
                }
                let valid = match layer.kind.as_str() {
                    "image" => layer.src.as_deref().is_some_and(|s| !s.is_empty() && image_url(s)),
                    "text" => layer.text.as_ref().is_some_and(|t| t.chars().count() <= 500)
                        && layer.color.as_deref().is_some_and(hex_color)
                        && layer.font_size.is_some_and(|s| (24.0..=180.0).contains(&s)),
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
        let slide = |layout: &str, eyebrow: Option<&str>, title: &str, body: &str, new_id: &mut dyn FnMut() -> String| Slide {
            eyebrow: eyebrow.unwrap_or("THE CREATIVE NOTES").into(),
            title: title.into(),
            body: body.into(),
            ..Slide::new(layout, new_id())
        };
        let slides = vec![
            slide("introduction", Some("A NOTE FOR THE CREATORS"), "Good design.\nGreat stories.", "Turn your next big idea into something\nworth swiping for.", new_id),
            slide("text", None, "Start with\nsomething real.", "The best carousels begin with an idea your audience cares about.\n\nTeach something. Share a perspective. Tell a story only you can tell.", new_id),
            slide("checklist", None, "Make every\nslide count.", "A few small things make a big difference.", new_id),
            slide("quote", Some("A THOUGHT WORTH SAVING"), "Great things are done by a series of small things brought together.", "— Vincent van Gogh", new_id),
            slide("thank-you", Some("THAT’S A WRAP"), "Good ideas\nare better shared.", "Save this for your next creative day.\nFollow along for a little more inspiration.", new_id),
        ];
        Self {
            version: 1, name: "The art of a good carousel".into(), format: "portrait".into(), theme: "light".into(), accent: "#e65443".into(),
            brand: "YOUR STUDIO".into(), handle: "@yourstudio".into(), font: "Arial".into(), direction: None, slides, extra: Map::new(),
        }
    }
}

impl Slide {
    /// A slide like Electron Folio's `createSlide(layout)`.
    pub fn new(layout: &str, id: String) -> Self {
        Self {
            id, layout: layout.into(), eyebrow: "THE CREATIVE NOTES".into(), title: "One idea.\nA little possibility.".into(),
            body: "Share something useful. Make it simple, make it thoughtful, make it yours.".into(),
            items: vec!["Start with a clear idea".into(), "Make every slide count".into(), "Finish with a next step".into()],
            image: String::new(), image2: String::new(), layers: Vec::new(), title_size: 88.0, texture: true, swipe: true, extra: Map::new(),
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
        project.extra.insert("folderFileName".into(), "art--1.folio.json".into());
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
        project.slides[0].layers.push(Layer { id: String::new(), kind: "text".into(), x: 1000.0, y: 0.0, width: 200.0, height: 50.0, src: None, text: Some("x".into()), color: Some("#000000".into()), font_size: Some(40.0), extra: Map::new() });
        assert_eq!(project.validate(), Err("Element is outside the slide.".into()));
        let mut project = Project::starter(&mut ids());
        project.slides[1].image = "data:image/gif;base64,AAAA".into();
        assert!(project.validate().is_err());
    }
}
