//! Slides drawn the way Electron Folio's canvas draws them (`renderer.js`): the same layouts, sizes, colors and
//! text fitting, at full size (1080 wide, 1080 or 1350 tall). Exports and the editor's preview both use it.

use crate::model::{Project, Slide};
use crate::text::{Align, Faces, has_rtl, wrap};
use base64::Engine;
use std::collections::HashMap;
use std::f32::consts::PI;
use tiny_skia::{Color, FillRule, IntSize, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};

/// Pictures in a project, decoded once from their data URLs.
#[derive(Default)]
pub struct Pictures {
    decoded: HashMap<String, Option<Pixmap>>,
}

impl Pictures {
    pub fn get(&mut self, url: &str) -> Option<&Pixmap> {
        if url.is_empty() {
            return None;
        }
        self.decoded.entry(url.to_owned()).or_insert_with(|| decode(url)).as_ref()
    }
}

fn decode(url: &str) -> Option<Pixmap> {
    let (_, encoded) = url.split_once(',')?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).ok()?;
    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = IntSize::from_wh(image.width(), image.height())?;
    let mut pixels = image.into_raw();
    for pixel in pixels.chunks_exact_mut(4) {
        let a = pixel[3] as u16;
        for c in &mut pixel[..3] {
            *c = ((*c as u16 * a + 127) / 255) as u8;
        }
    }
    Pixmap::from_vec(pixels, size)
}

/// A CSS color: `#rrggbb`, or `#rrggbbaa` (Folio appends two digits of alpha to theme colors).
fn color(hex: &str) -> Color {
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255);
    Color::from_rgba8(channel(1), channel(3), channel(5), if hex.len() >= 9 { channel(7) } else { 255 })
}

struct Palette {
    bg: &'static str,
    fg: &'static str,
    muted: &'static str,
}

const LIGHT: Palette = Palette { bg: "#fff2ed", fg: "#20211f", muted: "#6c665f" };
const DARK: Palette = Palette { bg: "#151716", fg: "#faf8f1", muted: "#b6b5ac" };

/// A canvas with an optional clip, and the direction text is laid out in (the project's).
struct Canvas {
    pixmap: Pixmap,
    clip: Option<Mask>,
    rtl: bool,
    reduced: bool,
}

impl Canvas {
    fn paint(color_hex: &str) -> Paint<'static> {
        let mut paint = Paint::default();
        paint.set_color(color(color_hex));
        paint.anti_alias = true;
        paint
    }

    fn fill(&mut self, path: Option<tiny_skia::Path>, color_hex: &str) {
        if let Some(path) = path {
            self.pixmap.fill_path(&path, &Self::paint(color_hex), FillRule::Winding, Transform::identity(), self.clip.as_ref());
        }
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color_hex: &str) {
        self.fill(Rect::from_xywh(x, y, w, h).map(PathBuilder::from_rect), color_hex);
    }

    fn rounded(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color_hex: &str) {
        self.fill(rounded_rect(x, y, w, h, radius), color_hex);
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, color_hex: &str) {
        self.fill(PathBuilder::from_circle(cx, cy, r), color_hex);
    }

    fn polygon(&mut self, points: &[(f32, f32)], color_hex: &str) {
        let mut path = PathBuilder::new();
        path.move_to(points[0].0, points[0].1);
        for &(x, y) in &points[1..] {
            path.line_to(x, y);
        }
        path.close();
        self.fill(path.finish(), color_hex);
    }

    fn stroke(&mut self, path: Option<tiny_skia::Path>, width: f32, color_hex: &str) {
        if let Some(path) = path {
            let stroke = Stroke { width, ..Stroke::default() };
            self.pixmap.stroke_path(&path, &Self::paint(color_hex), &stroke, Transform::identity(), self.clip.as_ref());
        }
    }

    /// Clips what follows to a path, until [`Canvas::unclip`].
    fn clip_to(&mut self, path: Option<tiny_skia::Path>) {
        let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height()).unwrap();
        if let Some(path) = path {
            mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
        }
        self.clip = Some(mask);
    }

    fn unclip(&mut self) {
        self.clip = None;
    }

    /// Electron's `text()`: wraps `value` into `width`, shrinking it 2 px at a time (not below 16) until it fits in
    /// `max_height`, and draws the lines that fit. Right-to-left text, or any text in a right-to-left project, ends
    /// at the right edge. Returns the height drawn.
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, value: &str, x: f32, y: f32, width: f32, size: f32, color_hex: &str, font: &str, weight: u16, max_height: f32) -> f32 {
        let rtl = has_rtl(value);
        let right = self.rtl || rtl;
        let faces = Faces::new(font, weight);
        let mut actual = size;
        let lines = loop {
            let lines = wrap(value, width, &|s| faces.measure(s, actual, rtl));
            if lines.len() as f32 * actual * 1.18 <= max_height || actual <= 16.0 {
                break lines;
            }
            actual -= 2.0;
        };
        let fit = ((max_height / (actual * 1.18)).floor() as usize).max(1);
        let visible = &lines[..lines.len().min(fit)];
        for (i, line) in visible.iter().enumerate() {
            let line_y = y + i as f32 * actual * 1.18;
            let (at, align) = if right { (x + width, Align::Right) } else { (x, Align::Left) };
            faces.draw(&mut self.pixmap, self.clip.as_ref(), line, at, line_y, actual, rtl, align, color(color_hex));
        }
        if actual < size || visible.len() < lines.len() {
            self.reduced = true;
        }
        visible.len() as f32 * actual * 1.18
    }

    /// A picture cropped to fill a 22 px rounded frame, or Electron's placeholder when there's none.
    fn picture(&mut self, image: Option<&Pixmap>, x: f32, y: f32, w: f32, h: f32, p: &Palette, accent: &str) {
        self.clip_to(rounded_rect(x, y, w, h, 22.0));
        match image {
            Some(image) => {
                let scale = (w / image.width() as f32).max(h / image.height() as f32);
                let (iw, ih) = (image.width() as f32 * scale, image.height() as f32 * scale);
                let transform = Transform::from_scale(scale, scale).post_translate(x + (w - iw) / 2.0, y + (h - ih) / 2.0);
                let paint = PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..PixmapPaint::default() };
                self.pixmap.draw_pixmap(0, 0, image.as_ref(), &paint, transform, self.clip.as_ref());
            }
            None => {
                self.rect(x, y, w, h, &format!("{}0c", p.fg));
                self.circle(x + w * 0.78, y + h * 0.25, w.min(h) * 0.23, &format!("{accent}28"));
                self.polygon(&[(x, y + h), (x + w * 0.4, y + h * 0.4), (x + w * 0.75, y + h)], &format!("{accent}38"));
                self.polygon(&[(x + w * 0.3, y + h), (x + w * 0.75, y + h * 0.52), (x + w, y + h)], &format!("{accent}20"));
                self.text("YOUR IMAGE HERE", x + 25.0, y + h - 46.0, w - 50.0, 18.0, p.muted, "Arial", 600, f32::INFINITY);
            }
        }
        self.unclip();
    }
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Option<tiny_skia::Path> {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let k = r * 0.552_285;
    let mut path = PathBuilder::new();
    path.move_to(x + r, y);
    path.line_to(x + w - r, y);
    path.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    path.line_to(x + w, y + h - r);
    path.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    path.line_to(x + r, y + h);
    path.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    path.line_to(x, y + r);
    path.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    path.close();
    path.finish()
}

/// An arc as canvas `arc()` strokes it: clockwise on screen from `start` to `end` radians.
fn arc(cx: f32, cy: f32, r: f32, start: f32, end: f32) -> Option<tiny_skia::Path> {
    let mut path = PathBuilder::new();
    let steps = 96;
    for i in 0..=steps {
        let a = start + (end - start) * i as f32 / steps as f32;
        let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
        if i == 0 { path.move_to(x, y) } else { path.line_to(x, y) }
    }
    path.finish()
}

/// The slide at `index`, 1080 wide; and whether any text had to shrink or be cut to fit.
pub fn slide(project: &Project, index: usize, pictures: &mut Pictures) -> (Pixmap, bool) {
    let slide = &project.slides[index];
    let h = project.height();
    let mut canvas = Canvas { pixmap: Pixmap::new(1080, h as u32).unwrap(), clip: None, rtl: project.direction.as_deref() == Some("rtl"), reduced: false };
    if slide.layout == "navy-ribbon" {
        navy(&mut canvas, slide, project, index, h);
    } else {
        base(&mut canvas, slide, project, index, h, pictures);
    }
    // Movable elements, each clipped to its box.
    for layer in &slide.layers {
        canvas.clip_to(Rect::from_xywh(layer.x, layer.y, layer.width, layer.height).map(PathBuilder::from_rect));
        match layer.kind.as_str() {
            "image" => {
                if let Some(image) = layer.src.as_deref().and_then(|src| pictures.get(src)) {
                    let transform = Transform::from_scale(layer.width / image.width() as f32, layer.height / image.height() as f32).post_translate(layer.x, layer.y);
                    let paint = PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..PixmapPaint::default() };
                    canvas.pixmap.draw_pixmap(0, 0, image.as_ref(), &paint, transform, canvas.clip.as_ref());
                }
            }
            "text" => {
                let (text, fill, size) = (layer.text.as_deref().unwrap_or(""), layer.color.as_deref().unwrap_or("#000000"), layer.font_size.unwrap_or(64.0));
                canvas.text(text, layer.x, layer.y, layer.width, size, fill, &project.font, 700, layer.height);
            }
            _ => {}
        }
        canvas.unclip();
    }
    (canvas.pixmap, canvas.reduced)
}

fn base(c: &mut Canvas, slide: &Slide, project: &Project, index: usize, h: f32, pictures: &mut Pictures) {
    let p = if project.theme == "dark" { &DARK } else { &LIGHT };
    let (accent, font) = (project.accent.as_str(), project.font.as_str());
    let (margin, width, bottom) = (88.0, 904.0, h - 190.0);
    let count = project.slides.len();
    c.rect(0.0, 0.0, 1080.0, h, p.bg);
    if slide.texture {
        let line = format!("{}05", p.fg);
        let mut x = -h;
        while x < 1080.0 {
            let mut path = PathBuilder::new();
            path.move_to(x, 0.0);
            path.line_to(x + h, h);
            c.stroke(path.finish(), 1.0, &line);
            x += 24.0;
        }
        let dot = format!("{accent}06");
        let mut x = 18.0;
        while x < 1080.0 {
            let mut y = 18.0;
            while y < h {
                c.circle(x, y, 1.2, &dot);
                y += 28.0;
            }
            x += 28.0;
        }
    }
    c.rect(0.0, 0.0, 1080.0 * (index + 1) as f32 / count as f32, 8.0, accent);
    c.text(&slide.eyebrow.to_uppercase(), margin, 87.0, width - 180.0, 22.0, accent, font, 700, 60.0);
    if slide.swipe {
        let label = format!("{:02} / {:02}", index + 1, count);
        let reduced = c.reduced;
        c.text(&label, 880.0, 88.0, 140.0, 22.0, p.muted, font, 400, f32::INFINITY);
        c.reduced = reduced;
    }
    let (title, body, size) = (slide.title.as_str(), slide.body.as_str(), slide.title_size);
    match slide.layout.as_str() {
        "introduction" => {
            let title_y = if h == 1350.0 { 290.0 } else { 215.0 };
            let th = c.text(title, margin, title_y, width, size + 18.0, p.fg, font, 700, h * 0.36);
            c.text(body, margin + 4.0, title_y + th + 34.0, width - 100.0, 32.0, p.muted, font, 400, bottom - title_y - th - 45.0);
            c.rounded(margin, bottom - 30.0, 216.0, 52.0, 26.0, accent);
            c.text("A LITTLE INSPIRATION", margin + 18.0, bottom - 12.0, 180.0, 14.0, p.bg, font, 700, f32::INFINITY);
            c.stroke(arc(897.0, bottom - 38.0, 56.0, 0.15, PI * 1.85), 4.0, accent);
            c.text("↗", 854.0, bottom - 86.0, 130.0, 83.0, accent, "Arial", 400, f32::INFINITY);
        }
        "quote" => {
            let y = if h == 1350.0 { 275.0 } else { 210.0 };
            let box_h = bottom - y - 30.0;
            c.rounded(margin, y, width, box_h, 30.0, &format!("{accent}12"));
            c.text("“", margin + 48.0, y + 25.0, 180.0, 150.0, accent, "Georgia", 700, f32::INFINITY);
            c.text(title, margin + 54.0, y + 146.0, width - 108.0, size * 0.78, accent, font, 700, box_h - 230.0);
            c.text(body, margin + 54.0, y + box_h - 65.0, width - 108.0, 24.0, p.muted, font, 400, 55.0);
        }
        "checklist" => {
            let th = c.text(title, margin, 200.0, width, size * 0.86, p.fg, font, 700, 185.0);
            let bh = c.text(body, margin, 220.0 + th, width, 28.0, p.muted, font, 400, 105.0);
            let y = 255.0 + th + bh;
            let items: Vec<&String> = slide.items.iter().take(5).collect();
            let gap = 16.0;
            let row_h = 120.0f32.min((bottom - y - gap * (items.len() as f32 - 1.0)) / items.len().max(1) as f32);
            for (i, item) in items.iter().enumerate() {
                let ry = y + i as f32 * (row_h + gap);
                c.rounded(margin, ry, width, row_h, 16.0, &format!("{accent}12"));
                c.rounded(margin + 27.0, ry + (row_h - 42.0) / 2.0, 42.0, 42.0, 21.0, accent);
                c.text("✓", margin + 34.0, ry + (row_h - 30.0) / 2.0, 35.0, 27.0, p.bg, "Arial", 700, f32::INFINITY);
                c.text(item, margin + 95.0, ry + 12.0f32.max((row_h - 48.0) / 2.0), width - 125.0, 38.0, p.fg, font, 600, row_h - 20.0);
            }
        }
        "image-left" | "image-right" => {
            let left = slide.layout == "image-left";
            let y = 238.0;
            c.picture(pictures.get(&slide.image), if left { margin } else { 571.0 }, y, 421.0, bottom - y, p, accent);
            let tx = if left { 558.0 } else { margin };
            let th = c.text(title, tx, y + 18.0, 420.0, size * 0.73, p.fg, font, 700, (bottom - y) * 0.52);
            c.text(body, tx, y + th + 50.0, 420.0, 28.0, p.muted, font, 400, bottom - y - th - 55.0);
        }
        "image-center" | "compare" => {
            let th = c.text(title, margin, 191.0, width, size * 0.84, p.fg, font, 700, 175.0);
            let y = 231.0 + th;
            let image_h = bottom - y - 110.0;
            if slide.layout == "compare" {
                c.picture(pictures.get(&slide.image), margin, y, 440.0, image_h, p, accent);
                c.picture(pictures.get(&slide.image2), 552.0, y, 440.0, image_h, p, accent);
            } else {
                c.picture(pictures.get(&slide.image), margin, y, width, image_h, p, accent);
            }
            c.text(body, margin, y + image_h + 26.0, width, 28.0, p.muted, font, 400, 90.0);
        }
        "thank-you" => {
            c.text("✳", 462.0, h * 0.23, 200.0, 132.0, accent, "Arial", 400, f32::INFINITY);
            let y = h * 0.4;
            let th = c.text(title, margin, y, width, size + 8.0, p.fg, font, 700, 260.0);
            c.text(body, margin + 4.0, y + th + 32.0, width - 60.0, 32.0, p.muted, font, 400, bottom - y - th - 45.0);
        }
        _ => {
            let th = c.text(title, margin, 235.0, width, size, p.fg, font, 700, 310.0);
            c.rounded(margin, 267.0 + th, 80.0, 6.0, 3.0, accent);
            c.text(body, margin, 307.0 + th, width, 37.0, p.muted, font, 400, bottom - th - 320.0);
        }
    }
    // Brand, handle and the swipe cue don't count as text that had to shrink (Electron draws them outside `draw`).
    let reduced = c.reduced;
    c.text(&project.brand, margin, h - 123.0, 650.0, 25.0, p.fg, font, 700, 34.0);
    c.text(&project.handle, margin, h - 86.0, 650.0, 20.0, p.muted, font, 400, 28.0);
    if slide.swipe {
        let last = index == count - 1;
        c.text(if last { "SAVE & SHARE" } else { "SWIPE FOR MORE" }, 734.0, h - 102.0, 200.0, 16.0, p.muted, font, 600, f32::INFINITY);
        c.rounded(936.0, h - 122.0, 56.0, 56.0, 28.0, accent);
        c.text(if last { "♡" } else { "→" }, 950.0, h - 111.0, 50.0, 29.0, p.bg, "Arial", 400, f32::INFINITY);
    }
    c.reduced = reduced;
}

/// Electron's `ribbonNoise`: a hash of a world position, so ribbons continue across neighboring slides.
pub fn ribbon_noise(x: i32, seed: i32) -> f32 {
    let mut n = (x ^ seed).wrapping_mul(0x45d9f3b);
    n = (n ^ ((n as u32 >> 16) as i32)).wrapping_mul(0x45d9f3b);
    ((n ^ ((n as u32 >> 16) as i32)) as u32) as f32 / 4294967295.0
}

/// A torn white ribbon across the slide at `y`, with crumbs along both edges.
fn ribbon(c: &mut Canvas, y: f32, index: usize, seed: i32) {
    let world = |x: i32| index as i32 * 1080 + x;
    let mut path = PathBuilder::new();
    let mut x = -4;
    while x <= 1084 {
        let w = world(x);
        let edge = y - 24.0 - ribbon_noise(w, seed) * 15.0 + ribbon_noise(w.div_euclid(14), seed + 1) * 10.0;
        if x == -4 { path.move_to(x as f32, edge) } else { path.line_to(x as f32, edge) }
        x += 2;
    }
    let mut x = 1084;
    while x >= -4 {
        let w = world(x);
        path.line_to(x as f32, y + 24.0 + ribbon_noise(w, seed + 2) * 18.0 - ribbon_noise(w.div_euclid(18), seed + 3) * 12.0);
        x -= 2;
    }
    path.close();
    c.fill(path.finish(), "#ffffff");
    let mut x = -4;
    while x < 1084 {
        let w = world(x);
        for direction in [-1.0, 1.0] {
            let offset = 25.0 + ribbon_noise(w, seed + 4) * 38.0;
            let radius = 1.5 + ribbon_noise(w, seed + 5) * 3.8;
            c.circle(x as f32, y + direction * offset, radius, "#ffffff");
        }
        x += 3;
    }
}

fn navy(c: &mut Canvas, slide: &Slide, project: &Project, index: usize, height: f32) {
    let cream = project.accent.as_str();
    let (top, bottom) = (height * 0.17, height * 0.81);
    c.rect(0.0, 0.0, 1080.0, height, "#3c415d");
    if slide.texture {
        ribbon(c, top, index, 87);
        ribbon(c, bottom, index, 321);
    } else {
        c.rect(0.0, top - 24.0, 1080.0, 48.0, "#ffffff");
        c.rect(0.0, bottom - 24.0, 1080.0, 48.0, "#ffffff");
    }
    let title_size = slide.title_size;
    let centered = slide.body.trim().is_empty() && slide.eyebrow.trim().is_empty();
    if centered {
        let faces = Faces::new(&project.font, 700);
        let rtl = has_rtl(&slide.title);
        let mut actual = title_size;
        let lines = loop {
            let lines = wrap(&slide.title, 830.0, &|s| faces.measure(s, actual, rtl));
            if lines.len() as f32 * actual * 1.18 <= bottom - top - 190.0 || actual <= 18.0 {
                break lines;
            }
            actual -= 2.0;
        };
        if actual < title_size {
            c.reduced = true;
        }
        let y = (top + bottom - lines.len() as f32 * actual * 1.18) / 2.0;
        for (i, line) in lines.iter().enumerate() {
            faces.draw(&mut c.pixmap, None, line, 540.0, y + i as f32 * actual * 1.18, actual, rtl, Align::Center, color(cream));
        }
    } else {
        let (x, width) = (88.0, 900.0);
        let mut y = top + 164.0;
        if !slide.eyebrow.is_empty() {
            c.text(&slide.eyebrow, x, y - 64.0, width, 22.0, cream, &project.font, 700, 55.0);
        }
        let th = c.text(&slide.title, x, y, width, title_size, cream, &project.font, 700, (bottom - y - 100.0) * 0.5);
        y += th + 78.0;
        c.text(&slide.body, x, y, width, 34.0, cream, &project.font, 600, bottom - y - 85.0);
    }
    if slide.swipe {
        let reduced = c.reduced;
        let line = if project.handle.is_empty() { project.brand.clone() } else { format!("{}  {}", project.brand, project.handle) };
        c.text(&line, 88.0, height - 95.0, 720.0, 20.0, cream, &project.font, 600, 32.0);
        c.text(&format!("{} / {}  →", index + 1, project.slides.len()), 880.0, height - 95.0, 150.0, 20.0, cream, &project.font, 600, 32.0);
        c.reduced = reduced;
    }
}

/// A slide as PNG bytes.
pub fn png(project: &Project, index: usize, pictures: &mut Pictures) -> Result<Vec<u8>, String> {
    // Slides are opaque, so their premultiplied pixels are the plain ones.
    let pixmap = slide(project, index, pictures).0;
    let mut png = Vec::new();
    image::ImageEncoder::write_image(image::codecs::png::PngEncoder::new(&mut png), pixmap.data(), pixmap.width(), pixmap.height(), image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ribbon_noise_matches_javascript() {
        // Values from Electron's ribbonNoise, run in node.
        for (x, seed, expected) in [(0, 87, 0.398701), (-4, 88, 0.328496), (1080 * 3 + 517, 321, 0.858987), ((-4f32 / 14.0).floor() as i32, 88, 0.669261)] {
            assert!((ribbon_noise(x, seed) - expected).abs() < 1e-5, "{x} {seed}: {}", ribbon_noise(x, seed));
        }
    }
}

#[cfg(test)]
mod compare {
    use super::*;

    /// Every slide of `tests/fixtures/compare.folio.json` (all ten layouts' ways of drawing, an image placeholder,
    /// shrinking text, Arabic, the navy ribbon) against Electron Folio's own render of it, made with its
    /// `renderer.js` in Electron on 2026-11-01. Text antialiasing differs a little; layout, color and position may not.
    #[test]
    fn slides_match_what_electron_folio_draws() {
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let project = Project::read(&std::fs::read(fixtures.join("compare.folio.json")).unwrap()).unwrap();
        let mut pictures = Pictures::default();
        for i in 0..project.slides.len() {
            let ours = slide(&project, i, &mut pictures).0;
            let theirs = image::open(fixtures.join(format!("electron-{i}.png"))).unwrap().to_rgba8();
            assert_eq!((ours.width(), ours.height()), theirs.dimensions());
            let total: u64 = ours.data().iter().zip(theirs.as_raw()).map(|(a, b)| (*a as i32 - *b as i32).unsigned_abs() as u64).sum();
            let mean = total as f64 / ours.data().len() as f64;
            assert!(mean < 1.0, "slide {i} ({}): mean difference {mean:.3} per channel", project.slides[i].layout);
        }
    }

    /// Writes what Folio native draws for the comparison project into `FOLIO_COMPARE` (to remake the fixtures).
    #[test]
    #[ignore]
    fn write_native_slides() {
        let dir = std::path::PathBuf::from(std::env::var("FOLIO_COMPARE").unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let project = Project::read(&std::fs::read(fixtures.join("compare.folio.json")).unwrap()).unwrap();
        let mut pictures = Pictures::default();
        for i in 0..project.slides.len() {
            std::fs::write(dir.join(format!("native-{i}.png")), png(&project, i, &mut pictures).unwrap()).unwrap();
        }
    }
}
