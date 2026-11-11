//! Text as Folio's canvas draws it (`renderer.js` `text()` and `wrap()`): the project's font (Arial, Georgia,
//! Verdana or Trebuchet MS from Windows' fonts folder, bold from weight 600), with Segoe UI Symbol and Segoe UI for
//! characters it lacks; shaped with rustybuzz, so Arabic joins and runs right to left, and laid out in bidi order;
//! wrapped at spaces (characters when a word is wider than the line), shrunk 2 px at a time until it fits, drawn
//! from the top of the em box with lines 1.18 em apart.

use rustybuzz::ttf_parser::{GlyphId, OutlineBuilder};
use rustybuzz::{Direction, Face, UnicodeBuffer};
use std::collections::HashMap;
use std::sync::OnceLock;
use tiny_skia::{FillRule, Mask, Paint, PathBuilder, Pixmap, Transform};
use unicode_bidi::{BidiInfo, Level};

/// A font's file in Windows' fonts folder: regular, then bold.
fn files(family: &str) -> (&'static str, &'static str) {
    match family {
        "Georgia" => ("georgia.ttf", "georgiab.ttf"),
        "Verdana" => ("verdana.ttf", "verdanab.ttf"),
        "Trebuchet MS" => ("trebuc.ttf", "trebucbd.ttf"),
        "Segoe UI" => ("segoeui.ttf", "segoeuib.ttf"),
        "Calibri" => ("calibri.ttf", "calibrib.ttf"),
        "Times New Roman" => ("times.ttf", "timesbd.ttf"),
        "Courier New" => ("cour.ttf", "courbd.ttf"),
        "Impact" => ("impact.ttf", "impact.ttf"),
        "Comic Sans MS" => ("comic.ttf", "comicbd.ttf"),
        _ => ("arial.ttf", "arialbd.ttf"),
    }
}

/// A font file's bytes, read once and kept for the life of the program.
fn data(name: &str) -> Option<&'static [u8]> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, Option<&'static [u8]>>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = cache.lock().unwrap();
    *cache.entry(name.to_owned()).or_insert_with(|| {
        let path = std::path::Path::new(r"C:\Windows\Fonts").join(name);
        std::fs::read(path)
            .ok()
            .map(|bytes| &*Box::leak(bytes.into_boxed_slice()))
    })
}

/// The faces one piece of text can use: the font, then fallbacks for characters it doesn't have.
pub struct Faces {
    faces: Vec<Face<'static>>,
}

impl Faces {
    pub fn new(family: &str, weight: u16) -> Self {
        let bold = weight >= 600;
        let (regular, heavy) = files(family);
        let names = [
            if bold { heavy } else { regular },
            "seguisym.ttf",
            if bold { "segoeuib.ttf" } else { "segoeui.ttf" },
        ];
        Self {
            faces: names
                .iter()
                .filter_map(|name| data(name).and_then(|bytes| Face::from_slice(bytes, 0)))
                .collect(),
        }
    }

    /// Where the top of the em box is above the baseline, per pixel of font size: the first face's typographic
    /// ascent and descent scaled to fill one em, as Chrome places canvas text with `textBaseline = 'top'` (for Arial
    /// 0.776 em; the hhea metrics would put text 0.034 em lower, measured against Electron).
    fn top(&self) -> f32 {
        self.faces.first().map_or(0.8, |face| {
            let ascent = face.typographic_ascender().unwrap_or(face.ascender()) as f32;
            let descent = -(face.typographic_descender().unwrap_or(face.descender()) as f32);
            ascent / (ascent + descent)
        })
    }

    fn face_for(&self, c: char) -> usize {
        self.faces
            .iter()
            .position(|face| face.glyph_index(c).is_some())
            .unwrap_or(0)
    }

    /// One line laid out left to right at `size` px: each glyph's face, id, x and y offset, and the line's width.
    fn shape(&self, line: &str, size: f32, rtl: bool) -> (Vec<(usize, GlyphId, f32, f32)>, f32) {
        let mut glyphs = Vec::new();
        let mut pen = 0.0;
        if line.is_empty() || self.faces.is_empty() {
            return (glyphs, 0.0);
        }
        let bidi = BidiInfo::new(line, Some(if rtl { Level::rtl() } else { Level::ltr() }));
        let Some(paragraph) = bidi.paragraphs.first() else {
            return (glyphs, 0.0);
        };
        let (levels, runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
        for run in runs {
            let right_to_left = levels[run.start].is_rtl();
            // Pieces of the run that one face covers; in a right-to-left run they go on the line in reverse.
            let mut pieces: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
            for (offset, c) in line[run.clone()].char_indices() {
                let at = run.start + offset;
                let face = self.face_for(c);
                match pieces.last_mut() {
                    Some((last, range)) if *last == face => range.end = at + c.len_utf8(),
                    _ => pieces.push((face, at..at + c.len_utf8())),
                }
            }
            if right_to_left {
                pieces.reverse();
            }
            for (face_index, range) in pieces {
                let face = &self.faces[face_index];
                let scale = size / face.units_per_em() as f32;
                let mut buffer = UnicodeBuffer::new();
                buffer.push_str(&line[range]);
                buffer.set_direction(if right_to_left {
                    Direction::RightToLeft
                } else {
                    Direction::LeftToRight
                });
                buffer.guess_segment_properties();
                let shaped = rustybuzz::shape(face, &[], buffer);
                for (info, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                    glyphs.push((
                        face_index,
                        GlyphId(info.glyph_id as u16),
                        pen + position.x_offset as f32 * scale,
                        position.y_offset as f32 * scale,
                    ));
                    pen += position.x_advance as f32 * scale;
                }
            }
        }
        (glyphs, pen)
    }

    pub fn measure(&self, line: &str, size: f32, rtl: bool) -> f32 {
        self.shape(line, size, rtl).1
    }

    /// Draws one line with the top of its em box at `y`, starting at `x`, or ending there when `align` is
    /// [`Align::Right`], or centered on it.
    pub fn draw(
        &self,
        canvas: &mut Pixmap,
        clip: Option<&Mask>,
        line: &str,
        x: f32,
        y: f32,
        size: f32,
        rtl: bool,
        align: Align,
        color: tiny_skia::Color,
    ) {
        let (glyphs, width) = self.shape(line, size, rtl);
        let left = match align {
            Align::Left => x,
            Align::Right => x - width,
            Align::Center => x - width / 2.0,
        };
        let baseline = y + size * self.top();
        let mut path = PathBuilder::new();
        for (face_index, glyph, gx, gy) in glyphs {
            let face = &self.faces[face_index];
            let scale = size / face.units_per_em() as f32;
            let mut pen = Pen {
                path: &mut path,
                x: left + gx,
                y: baseline - gy,
                scale,
            };
            face.outline_glyph(glyph, &mut pen);
        }
        let Some(path) = path.finish() else { return };
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        canvas.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            clip,
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    Center,
}

struct Pen<'a> {
    path: &'a mut PathBuilder,
    x: f32,
    y: f32,
    scale: f32,
}

impl Pen<'_> {
    fn at(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x * self.scale, self.y - y * self.scale)
    }
}

impl OutlineBuilder for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.path.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.path.line_to(x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let ((x1, y1), (x, y)) = (self.at(x1, y1), self.at(x, y));
        self.path.quad_to(x1, y1, x, y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let ((x1, y1), (x2, y2), (x, y)) = (self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        self.path.cubic_to(x1, y1, x2, y2, x, y);
    }
    fn close(&mut self) {
        self.path.close();
    }
}

/// Lines of `value` no wider than `width` (Electron's `wrap()`): paragraphs at newlines, words at spaces, a word
/// wider than the line broken between characters.
pub fn wrap(value: &str, width: f32, measure: &dyn Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in value.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if measure(word) > width {
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                for c in word.chars() {
                    let mut longer = line.clone();
                    longer.push(c);
                    if !line.is_empty() && measure(&longer) > width {
                        lines.push(std::mem::take(&mut line));
                    }
                    line.push(c);
                }
            } else if !line.is_empty() && measure(&format!("{line} {word}")) > width {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
        }
        lines.push(line);
    }
    lines
}

/// Whether text has right-to-left letters (Hebrew, Arabic and the scripts between), as Folio checks it.
pub fn has_rtl(value: &str) -> bool {
    value
        .chars()
        .any(|c| ('\u{0590}'..='\u{08ff}').contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_wrap_and_long_words_break() {
        let measure = |s: &str| s.chars().count() as f32 * 10.0;
        assert_eq!(
            wrap("one two three", 75.0, &measure),
            vec!["one two", "three"]
        );
        assert_eq!(
            wrap("abcdefghij", 40.0, &measure),
            vec!["abcd", "efgh", "ij"]
        );
        assert_eq!(wrap("a\n\nb", 100.0, &measure), vec!["a", "", "b"]);
    }

    #[test]
    fn arabic_is_joined_and_measured() {
        let faces = Faces::new("Arial", 400);
        let joined = faces.measure("سلام", 40.0, true);
        let apart: f32 = "سلام"
            .chars()
            .map(|c| faces.measure(&c.to_string(), 40.0, true))
            .sum();
        assert!(
            joined > 10.0 && (joined - apart).abs() > 1.0,
            "joined letters take other forms: {joined} vs {apart}"
        );
        assert!(has_rtl("Hello سلام") && !has_rtl("Hello"));
    }
}
