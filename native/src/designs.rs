//! Ready-made blank-layout slides to start from: a few shapes and text boxes in the project's own colors, every
//! one of them an element to move, restyle or delete.

use crate::model::{Layer, Project, Slide};

/// Each design's id, name, and what it's for.
pub const DESIGNS: [(&str, &str, &str); 5] = [
    ("empty", "Empty", "Nothing on it yet"),
    ("statement", "Big statement", "One line that fills the slide"),
    ("card", "Card", "A heading and a few words on a card"),
    ("tip", "Numbered tip", "A big number, a title and the detail"),
    ("cta", "Call to action", "A full-color slide with a button"),
];

struct Kit<'a> {
    fg: &'a str,
    muted: &'a str,
    card: &'a str,
    accent: &'a str,
}

fn text(value: &str, at: (f32, f32, f32, f32), size: f32, color: &str) -> Layer {
    Layer { kind: "text".into(), x: at.0, y: at.1, width: at.2, height: at.3, text: Some(value.into()), color: Some(color.into()), font_size: Some(size), ..Layer::default() }
}

fn shape(kind: &str, at: (f32, f32, f32, f32), color: &str) -> Layer {
    Layer { kind: "shape".into(), shape: Some(kind.into()), x: at.0, y: at.1, width: at.2, height: at.3, color: Some(color.into()), ..Layer::default() }
}

/// A blank slide laid out as `design`, for this project's size, theme and accent.
pub fn slide(design: &str, project: &Project, new_id: &mut dyn FnMut() -> String) -> Slide {
    let h = project.height();
    let dark = project.theme == "dark";
    let kit = Kit {
        fg: if dark { "#faf8f1" } else { "#20211f" },
        muted: if dark { "#b6b5ac" } else { "#6c665f" },
        card: if dark { "#232625" } else { "#ffffff" },
        accent: &project.accent,
    };
    let handle = if project.handle.trim().is_empty() { "@yourstudio" } else { project.handle.trim() };
    let mut layers = match design {
        "statement" => vec![
            Layer { radius: Some(7.0), ..shape("rect", (88.0, 96.0, 120.0, 14.0), kit.accent) },
            text("Say one thing\nthat matters.", (88.0, 160.0, 904.0, 540.0), 112.0, kit.fg),
            Layer { bold: Some(false), ..text("Then give it room. A line or two underneath is all it needs.", (88.0, h - 340.0, 760.0, 160.0), 40.0, kit.muted) },
            text(handle, (88.0, h - 130.0, 700.0, 50.0), 30.0, kit.accent),
        ],
        "card" => vec![
            Layer { opacity: Some(0.9), ..shape("ellipse", (640.0, -180.0, 620.0, 620.0), kit.accent) },
            Layer { opacity: Some(0.08), ..shape("ellipse", (-200.0, h - 420.0, 560.0, 560.0), kit.fg) },
            Layer { radius: Some(40.0), ..shape("rect", (110.0, h / 2.0 - 260.0, 860.0, 520.0), kit.card) },
            text("A heading\nfor your card", (170.0, h / 2.0 - 190.0, 740.0, 230.0), 76.0, kit.fg),
            Layer { bold: Some(false), ..text("A few words that say what this is about and why it's worth a look.", (170.0, h / 2.0 + 70.0, 740.0, 150.0), 36.0, kit.muted) },
        ],
        "tip" => vec![
            Layer { opacity: Some(0.22), ..text("01", (70.0, 60.0, 760.0, 400.0), 360.0, kit.accent) },
            text("Name the tip\nin a few words", (88.0, 440.0, 904.0, 250.0), 92.0, kit.fg),
            Layer { opacity: Some(0.2), ..shape("rect", (88.0, 730.0, 904.0, 4.0), kit.fg) },
            Layer { bold: Some(false), ..text("Explain it here. Keep it to what someone can do today.", (88.0, 770.0, 904.0, (h - 860.0).max(120.0)), 42.0, kit.muted) },
        ],
        "cta" => vec![
            shape("rect", (0.0, 0.0, 1080.0, h), kit.accent),
            Layer { align: Some("center".into()), ..text("Ready when\nyou are.", (90.0, h / 2.0 - 290.0, 900.0, 300.0), 104.0, "#ffffff") },
            Layer { radius: Some(55.0), ..shape("rect", (290.0, h / 2.0 + 90.0, 500.0, 110.0), "#ffffff") },
            Layer { align: Some("center".into()), ..text("Follow for more", (290.0, h / 2.0 + 122.0, 500.0, 60.0), 40.0, kit.accent) },
        ],
        _ => Vec::new(),
    };
    for layer in &mut layers {
        layer.id = new_id();
    }
    let name = DESIGNS.iter().find(|d| d.0 == design).map_or("Blank", |d| d.1);
    Slide { eyebrow: String::new(), title: name.into(), body: String::new(), texture: false, swipe: false, layers, ..Slide::new("blank", new_id()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes every design as a PNG into the folder `FOLIO_DESIGNS` names, to look at: run with `--ignored`.
    #[test]
    #[ignore]
    fn write_designs() {
        let Some(dir) = std::env::var_os("FOLIO_DESIGNS").map(std::path::PathBuf::from) else { return };
        std::fs::create_dir_all(&dir).unwrap();
        let mut n = 0;
        let mut ids = || {
            n += 1;
            format!("id-{n}")
        };
        let mut project = Project::starter(&mut ids);
        for (format, theme) in [("portrait", "light"), ("square", "dark")] {
            (project.format, project.theme) = (format.into(), theme.into());
            project.slides = DESIGNS.iter().map(|d| slide(d.0, &project, &mut ids)).collect();
            for (i, design) in DESIGNS.iter().enumerate() {
                let png = crate::render::png(&project, i, &mut Default::default()).unwrap();
                std::fs::write(dir.join(format!("{}-{format}.png", design.0)), png).unwrap();
            }
        }
    }

    #[test]
    fn every_design_is_a_valid_slide_in_every_size() {
        let mut n = 0;
        let mut ids = || {
            n += 1;
            format!("id-{n}")
        };
        let mut project = Project::starter(&mut ids);
        for format in ["portrait", "square", "story"] {
            project.format = format.into();
            project.slides = DESIGNS.iter().map(|d| slide(d.0, &project, &mut ids)).collect();
            assert_eq!(project.validate(), Ok(()), "{format}");
            assert!(project.slides[0].layers.is_empty() && project.slides[1].layers.len() == 4);
        }
    }
}
