//! The slide as a design surface: pick elements (Shift adds), drag them with guides that line them up, stretch them
//! by eight handles, turn them by the handle above, and style them in the inspector. Text, shapes and pictures all
//! work the same way.

use super::{Folio, new_id, picture_url};
use crate::model::{LAYER_FONTS, Layer, MAX_LAYERS, SHAPES};
use canopy_ui::{color, font};
use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

/// The smallest an element can be made by dragging.
const SMALLEST: f32 = 8.0;
const GUIDE: Color32 = Color32::from_rgb(255, 77, 166);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Mode {
    Move,
    /// A handle: which way it pulls across (-1 left, 1 right, 0 not) and down.
    Stretch(f32, f32),
    Rotate,
}

/// Where an element was when a drag began (its picture or text isn't needed, and can be megabytes).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Frame {
    id: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rotation: f32,
    font_size: Option<f32>,
    shape: bool,
    text: bool,
}

impl Frame {
    fn of(layer: &Layer) -> Self {
        Self { id: layer.id.clone(), x: layer.x, y: layer.y, w: layer.width, h: layer.height, rotation: layer.rotation(), font_size: layer.font_size, shape: layer.kind == "shape", text: layer.kind == "text" }
    }
    fn center(&self) -> Pos2 {
        pos2(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
    /// Whether a point on the slide is inside the element's (turned) box.
    fn contains(&self, p: Pos2) -> bool {
        let local = turned(p - self.center(), -self.rotation);
        local.x.abs() <= self.w / 2.0 && local.y.abs() <= self.h / 2.0
    }
    /// A point of the box, by how far across and down from its middle (-1 to 1), where it is on the slide.
    fn point(&self, across: f32, down: f32) -> Pos2 {
        self.center() + turned(vec2(across * self.w / 2.0, down * self.h / 2.0), self.rotation)
    }
}

#[derive(Clone)]
pub(super) struct Drag {
    mode: Mode,
    start: Vec<Frame>,
}

/// A line an element snapped to: down the slide at an x, or across it at a y.
#[derive(Clone, Copy)]
pub(super) enum Guide {
    Down(f32),
    Across(f32),
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Order {
    Front,
    Forward,
    Backward,
    Back,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Line {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

fn turned(v: Vec2, degrees: f32) -> Vec2 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

/// The box (x, y, width, height) after dragging a handle by `total` on the slide. The handle pulls `sx` across and
/// `sy` down; the opposite corner or side stays where it is, also when the element is turned. `keep` holds a
/// corner drag to the element's proportions.
fn stretched(start: &Frame, sx: f32, sy: f32, total: Vec2, keep: bool) -> (f32, f32, f32, f32) {
    let local = turned(total, -start.rotation);
    let mut w = if sx != 0.0 { (start.w + sx * local.x).max(SMALLEST) } else { start.w };
    let mut h = if sy != 0.0 { (start.h + sy * local.y).max(SMALLEST) } else { start.h };
    if keep && sx != 0.0 && sy != 0.0 {
        let (across, down) = (w / start.w, h / start.h);
        let factor = if (across - 1.0).abs() > (down - 1.0).abs() { across } else { down }.max(SMALLEST / start.w.min(start.h));
        (w, h) = (start.w * factor, start.h * factor);
    }
    let anchor = start.point(-sx, -sy);
    let center = anchor + turned(vec2(sx * w / 2.0, sy * h / 2.0), start.rotation);
    (center.x - w / 2.0, center.y - h / 2.0, w, h)
}

/// How far to shift a box so that its low edge, middle or high edge (`edges`, where the drag would put them) meets
/// the nearest of `lines` within `reach`, and the line it meets.
fn snap(edges: [f32; 3], lines: &[f32], reach: f32) -> Option<(f32, f32)> {
    let mut best: Option<(f32, f32)> = None;
    for edge in edges {
        for &line in lines {
            let shift = line - edge;
            if shift.abs() <= reach && best.is_none_or(|(nearest, _)| shift.abs() < nearest.abs()) {
                best = Some((shift, line));
            }
        }
    }
    best
}

/// The angle a rotate handle at `pointer` gives an element around `center`: 0 with the handle straight up. It
/// settles on the eight compass directions when close, and `steps` holds it to 15° steps.
fn angle_to(center: Pos2, pointer: Pos2, steps: bool) -> f32 {
    let v = pointer - center;
    let mut degrees = v.y.atan2(v.x).to_degrees() + 90.0;
    if degrees > 180.0 {
        degrees -= 360.0;
    }
    if steps {
        degrees = (degrees / 15.0).round() * 15.0;
    } else {
        let near = (degrees / 45.0).round() * 45.0;
        if (degrees - near).abs() < 3.0 {
            degrees = near;
        }
    }
    if degrees <= -180.0 { degrees + 360.0 } else { degrees.round() }
}

/// A color button for a `#rrggbb` string. True when it changed.
pub(super) fn color_button(ui: &mut Ui, hex: &mut String) -> bool {
    let mut rgb = [1, 3, 5].map(|i| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0));
    let changed = egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed();
    if changed {
        *hex = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
    }
    changed
}

impl Folio {
    fn slide_index(&self) -> usize {
        self.selected.min(self.project.slides.len() - 1)
    }

    /// The picked elements' places in the slide's stack, back to front.
    fn picked_indices(&self) -> Vec<usize> {
        self.project.slides[self.slide_index()].layers.iter().enumerate().filter(|(_, l)| self.picked.contains(&l.id)).map(|(i, _)| i).collect()
    }

    fn theme_ink(&self) -> &'static str {
        if self.project.theme == "dark" { "#faf8f1" } else { "#20211f" }
    }

    /// Puts a new element on the slide and picks it.
    pub(super) fn add_layer(&mut self, mut layer: Layer) -> bool {
        if self.slide().layers.len() >= MAX_LAYERS {
            self.notice = Some(format!("A slide can have up to {MAX_LAYERS} elements."));
            return false;
        }
        layer.id = new_id();
        self.picked = vec![layer.id.clone()];
        self.slide().layers.push(layer);
        true
    }

    fn add_text(&mut self, words: &str, size: f32, bold: bool, height: f32) {
        let y = ((self.project.height() - height) / 2.0).round();
        let color = self.theme_ink().to_owned();
        self.add_layer(Layer {
            kind: "text".into(),
            x: 140.0,
            y,
            width: 800.0,
            height,
            text: Some(words.into()),
            color: Some(color),
            font_size: Some(size),
            bold: (!bold).then_some(false),
            ..Layer::default()
        });
    }

    fn add_shape(&mut self, shape: &str, size: (f32, f32), radius: Option<f32>, color: String) {
        let h = self.project.height();
        self.add_layer(Layer {
            kind: "shape".into(),
            shape: Some(shape.into()),
            x: ((1080.0 - size.0) / 2.0).round(),
            y: ((h - size.1) / 2.0).round(),
            width: size.0,
            height: size.1,
            color: Some(color),
            radius,
            ..Layer::default()
        });
    }

    /// The row of things to add: text in three sizes, shapes, a picture.
    pub(super) fn add_bar(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Add").font(font::bold(14.0)).color(color::INK));
        ui.horizontal_wrapped(|ui| {
            if ui.button("Heading").clicked() {
                self.add_text("Add a heading", 96.0, true, 240.0);
            }
            if ui.button("Subheading").clicked() {
                self.add_text("Add a subheading", 56.0, true, 150.0);
            }
            if ui.button("Body text").clicked() {
                self.add_text("Add a little body text", 36.0, false, 200.0);
            }
            ui.menu_button("Shape", |ui| {
                let accent = self.project.accent.clone();
                let ink = self.theme_ink().to_owned();
                let choice = [
                    ("Rectangle", "rect", (360.0, 360.0), None, &accent),
                    ("Rounded rectangle", "rect", (420.0, 300.0), Some(48.0), &accent),
                    ("Circle", "ellipse", (360.0, 360.0), None, &accent),
                    ("Triangle", "triangle", (360.0, 320.0), None, &accent),
                    ("Star", "star", (360.0, 360.0), None, &accent),
                    ("Line", "rect", (600.0, 8.0), None, &ink),
                ]
                .into_iter()
                .find(|(name, ..)| ui.button(*name).clicked());
                if let Some((_, shape, size, radius, fill)) = choice {
                    self.add_shape(shape, size, radius, fill.clone());
                    ui.close();
                }
            });
            if ui.button("Picture…").on_hover_text("Or drop one on the slide, or paste it").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Pictures", &["png", "jpg", "jpeg", "webp"]).pick_file() {
                    if let Err(error) = self.add_image_layer(&path) {
                        self.notice = Some(error);
                    }
                }
            }
        });
    }

    fn delete_picked(&mut self) {
        let picked = std::mem::take(&mut self.picked);
        self.slide().layers.retain(|l| !picked.contains(&l.id));
    }

    fn duplicate_picked(&mut self) {
        let copies: Vec<Layer> = self.picked_indices().into_iter().map(|i| self.slide().layers[i].clone()).collect();
        self.add_copies(copies, true);
    }

    /// Adds copies of elements to the slide and picks them; `aside` sets them a little down and right.
    fn add_copies(&mut self, copies: Vec<Layer>, aside: bool) {
        if copies.is_empty() {
            return;
        }
        if self.slide().layers.len() + copies.len() > MAX_LAYERS {
            self.notice = Some(format!("A slide can have up to {MAX_LAYERS} elements."));
            return;
        }
        let h = self.project.height();
        self.picked.clear();
        for mut layer in copies {
            layer.id = new_id();
            if aside {
                layer.x += 24.0;
                layer.y += 24.0;
            }
            keep_on_slide(&mut layer, h);
            self.picked.push(layer.id.clone());
            self.slide().layers.push(layer);
        }
    }

    fn copy_picked(&mut self, ui: &Ui) {
        let copies: Vec<Layer> = self.picked_indices().into_iter().map(|i| self.slide().layers[i].clone()).collect();
        if !copies.is_empty() {
            // The system clipboard gets a word, so the next paste is these elements and not an older picture.
            ui.ctx().copy_text("Folio elements".into());
            self.clipboard = copies;
        }
    }

    /// Pastes the copied elements: where they were on another slide, a little aside on the one they came from.
    pub(super) fn paste_layers(&mut self) {
        let copies = self.clipboard.clone();
        let here = &self.project.slides[self.slide_index()].layers;
        let aside = copies.iter().any(|c| here.iter().any(|l| (l.x, l.y) == (c.x, c.y)));
        self.add_copies(copies, aside);
    }

    fn reorder(&mut self, order: Order) {
        let picked = self.picked.clone();
        let layers = &mut self.slide().layers;
        let is = |l: &Layer| picked.contains(&l.id);
        match order {
            Order::Front | Order::Back => {
                let (mut chosen, mut rest): (Vec<Layer>, Vec<Layer>) = std::mem::take(layers).into_iter().partition(|l| is(l));
                if order == Order::Front {
                    rest.append(&mut chosen);
                    *layers = rest;
                } else {
                    chosen.append(&mut rest);
                    *layers = chosen;
                }
            }
            Order::Forward => {
                for i in (0..layers.len().saturating_sub(1)).rev() {
                    if is(&layers[i]) && !is(&layers[i + 1]) {
                        layers.swap(i, i + 1);
                    }
                }
            }
            Order::Backward => {
                for i in 1..layers.len() {
                    if is(&layers[i]) && !is(&layers[i - 1]) {
                        layers.swap(i, i - 1);
                    }
                }
            }
        }
    }

    /// Lines the picked elements up: one against the slide, several against the box around them.
    fn line_up(&mut self, line: Line) {
        let indices = self.picked_indices();
        let h = self.project.height();
        let layers = &mut self.slide().layers;
        let (left, top, right, bottom) = if indices.len() == 1 {
            (0.0, 0.0, 1080.0, h)
        } else {
            indices.iter().map(|&i| &layers[i]).fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, l| (b.0.min(l.x), b.1.min(l.y), b.2.max(l.x + l.width), b.3.max(l.y + l.height)))
        };
        for i in indices {
            let l = &mut layers[i];
            match line {
                Line::Left => l.x = left,
                Line::Center => l.x = ((left + right - l.width) / 2.0).round(),
                Line::Right => l.x = right - l.width,
                Line::Top => l.y = top,
                Line::Middle => l.y = ((top + bottom - l.height) / 2.0).round(),
                Line::Bottom => l.y = bottom - l.height,
            }
        }
    }

    /// Spaces three or more picked elements evenly between the outer two, across or down.
    fn space_out(&mut self, across: bool) {
        let mut indices = self.picked_indices();
        let layers = &mut self.slide().layers;
        if indices.len() < 3 {
            return;
        }
        let low = |l: &Layer| if across { l.x } else { l.y };
        let size = |l: &Layer| if across { l.width } else { l.height };
        indices.sort_by(|a, b| low(&layers[*a]).total_cmp(&low(&layers[*b])));
        let first = low(&layers[indices[0]]);
        let last = indices.last().map(|&i| low(&layers[i]) + size(&layers[i])).unwrap_or(first);
        let filled: f32 = indices.iter().map(|&i| size(&layers[i])).sum();
        let gap = (last - first - filled) / (indices.len() - 1) as f32;
        let mut at = first;
        for i in indices {
            let l = &mut layers[i];
            if across { l.x = at.round() } else { l.y = at.round() }
            at += size(l) + gap;
        }
    }

    /// Delete, Ctrl+D, Ctrl+C/X, Ctrl+A, Escape, arrows (Shift: 10), `[` and `]` (Ctrl: all the way). True when the
    /// arrows moved elements, so they don't also change the slide.
    pub(super) fn element_keys(&mut self, ui: &Ui) -> bool {
        use egui::{Key, Modifiers};
        let (delete, duplicate, all, escape, nudge, order, copy, cut) = ui.input_mut(|i| {
            let step = if i.modifiers.shift { 10.0 } else { 1.0 };
            let mut nudge = Vec2::ZERO;
            if !i.modifiers.command && !i.modifiers.alt {
                for (key, by) in [(Key::ArrowLeft, vec2(-1.0, 0.0)), (Key::ArrowRight, vec2(1.0, 0.0)), (Key::ArrowUp, vec2(0.0, -1.0)), (Key::ArrowDown, vec2(0.0, 1.0))] {
                    if i.key_pressed(key) {
                        nudge += by * step;
                    }
                }
            }
            let order = if i.consume_key(Modifiers::COMMAND, Key::CloseBracket) {
                Some(Order::Front)
            } else if i.consume_key(Modifiers::COMMAND, Key::OpenBracket) {
                Some(Order::Back)
            } else if i.consume_key(Modifiers::NONE, Key::CloseBracket) {
                Some(Order::Forward)
            } else if i.consume_key(Modifiers::NONE, Key::OpenBracket) {
                Some(Order::Backward)
            } else {
                None
            };
            let copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy));
            let cut = i.events.iter().any(|e| matches!(e, egui::Event::Cut));
            (
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.consume_key(Modifiers::COMMAND, Key::D),
                i.consume_key(Modifiers::COMMAND, Key::A),
                i.key_pressed(Key::Escape),
                nudge,
                order,
                i.consume_key(Modifiers::COMMAND, Key::C) || copy,
                i.consume_key(Modifiers::COMMAND, Key::X) || cut,
            )
        });
        if all {
            self.picked = self.project.slides[self.slide_index()].layers.iter().map(|l| l.id.clone()).collect();
        }
        if escape {
            self.picked.clear();
        }
        if self.picked_indices().is_empty() {
            return false;
        }
        if copy || cut {
            self.copy_picked(ui);
        }
        if delete || cut {
            self.delete_picked();
            return false;
        }
        if duplicate {
            self.duplicate_picked();
        }
        if let Some(order) = order {
            self.reorder(order);
        }
        if nudge != Vec2::ZERO {
            let h = self.project.height();
            for i in self.picked_indices() {
                let layer = &mut self.slide().layers[i];
                layer.x += nudge.x;
                layer.y += nudge.y;
                keep_on_slide(layer, h);
            }
            return true;
        }
        false
    }

    /// The elements on the slide: picking, dragging, stretching and turning them, with their outlines and handles.
    /// `slide` is where the slide is drawn, `scale` its pixels per slide unit, `area` the room around it.
    pub(super) fn elements(&mut self, ui: &mut Ui, slide: Rect, scale: f32, area: Rect) {
        let h = self.project.height();
        let index = self.slide_index();
        let frames: Vec<Frame> = self.project.slides[index].layers.iter().map(Frame::of).collect();
        self.picked.retain(|id| frames.iter().any(|f| &f.id == id));
        let to_screen = |p: Pos2| pos2(slide.left() + p.x * scale, slide.top() + p.y * scale);
        let to_slide = |p: Pos2| pos2((p.x - slide.left()) / scale, (p.y - slide.top()) / scale);
        let response = ui.interact(area, ui.scope_id().with("folio-canvas"), Sense::click_and_drag());

        // One picked element shows handles: corners and sides to stretch it, and one above to turn it.
        let single = match self.picked.as_slice() {
            [id] => frames.iter().find(|f| &f.id == id),
            _ => None,
        };
        let mut handles: Vec<(Mode, Pos2)> = Vec::new();
        if let Some(frame) = single {
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                handles.push((Mode::Stretch(sx, sy), frame.point(sx, sy)));
            }
            if frame.w * scale > 44.0 {
                handles.push((Mode::Stretch(0.0, -1.0), frame.point(0.0, -1.0)));
                handles.push((Mode::Stretch(0.0, 1.0), frame.point(0.0, 1.0)));
            }
            if frame.h * scale > 44.0 {
                handles.push((Mode::Stretch(-1.0, 0.0), frame.point(-1.0, 0.0)));
                handles.push((Mode::Stretch(1.0, 0.0), frame.point(1.0, 0.0)));
            }
            handles.push((Mode::Rotate, frame.center() + turned(vec2(0.0, -frame.h / 2.0 - 30.0 / scale), frame.rotation)));
        }
        let handle_at = |p: Pos2| handles.iter().rev().find(|(_, at)| to_screen(*at).distance(p) <= 10.0).map(|(mode, _)| *mode);
        // Topmost (last drawn) first, as overlapping elements stack.
        let layer_at = |p: Pos2| frames.iter().rev().find(|f| f.contains(to_slide(p)));
        let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
        let origin = ui.input(|i| i.pointer.press_origin());

        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(origin) = origin {
                if let Some(mode) = handle_at(origin) {
                    self.drag = Some(Drag { mode, start: single.cloned().into_iter().collect() });
                } else if let Some(frame) = layer_at(origin) {
                    if !self.picked.contains(&frame.id) {
                        if !shift {
                            self.picked.clear();
                        }
                        self.picked.push(frame.id.clone());
                    }
                    self.drag = Some(Drag { mode: Mode::Move, start: frames.iter().filter(|f| self.picked.contains(&f.id)).cloned().collect() });
                } else if !shift {
                    self.picked.clear();
                }
            }
        }
        if response.clicked() || response.secondary_clicked() {
            let under = response.interact_pointer_pos().and_then(|p| layer_at(p)).map(|f| f.id.clone());
            match under {
                // A right-click on one of several picked keeps them all, for the menu.
                Some(id) if response.secondary_clicked() && self.picked.contains(&id) => {}
                Some(id) if shift => match self.picked.iter().position(|p| p == &id) {
                    Some(at) => drop(self.picked.remove(at)),
                    None => self.picked.push(id),
                },
                Some(id) => self.picked = vec![id],
                None if !shift => self.picked.clear(),
                None => {}
            }
        }
        if response.double_clicked() && single.is_some_and(|f| f.text) {
            self.focus_text = true;
        }

        if let (Some(drag), true, Some(origin), Some(pointer)) = (self.drag.clone(), response.dragged(), origin, ui.input(|i| i.pointer.interact_pos())) {
            let total = (pointer - origin) / scale;
            let layers = &mut self.project.slides[index].layers;
            self.guides.clear();
            match drag.mode {
                Mode::Move => {
                    let mut by = total;
                    if !alt && !drag.start.is_empty() {
                        // The box around what's moving lines up with the slide's edges and middle, and with the
                        // edges and middles of everything staying put.
                        let (l, t, r, b) = drag.start.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, f| (b.0.min(f.x), b.1.min(f.y), b.2.max(f.x + f.w), b.3.max(f.y + f.h)));
                        let still: Vec<&Frame> = frames.iter().filter(|f| !drag.start.iter().any(|s| s.id == f.id)).collect();
                        let mut down = vec![0.0, 540.0, 1080.0];
                        let mut across = vec![0.0, h / 2.0, h];
                        for f in still {
                            down.extend([f.x, f.x + f.w / 2.0, f.x + f.w]);
                            across.extend([f.y, f.y + f.h / 2.0, f.y + f.h]);
                        }
                        let reach = 6.0 / scale;
                        if let Some((shift, line)) = snap([l + by.x, (l + r) / 2.0 + by.x, r + by.x], &down, reach) {
                            by.x += shift;
                            self.guides.push(Guide::Down(line));
                        }
                        if let Some((shift, line)) = snap([t + by.y, (t + b) / 2.0 + by.y, b + by.y], &across, reach) {
                            by.y += shift;
                            self.guides.push(Guide::Across(line));
                        }
                    }
                    for frame in &drag.start {
                        if let Some(layer) = layers.iter_mut().find(|l| l.id == frame.id) {
                            (layer.x, layer.y) = ((frame.x + by.x).round(), (frame.y + by.y).round());
                            keep_on_slide(layer, h);
                        }
                    }
                }
                Mode::Stretch(sx, sy) => {
                    if let Some((frame, layer)) = drag.start.first().and_then(|f| layers.iter_mut().find(|l| l.id == f.id).map(|l| (f, l))) {
                        // Pictures and text keep their shape from a corner, shapes stretch freely; Shift swaps that.
                        let keep = frame.shape == shift;
                        let (x, y, w, hh) = stretched(frame, sx, sy, total, keep);
                        (layer.x, layer.y, layer.width, layer.height) = (x.round(), y.round(), w.round().max(SMALLEST), hh.round().max(SMALLEST));
                        if let (true, true, Some(size)) = (frame.text && keep, sx != 0.0 && sy != 0.0, frame.font_size) {
                            // A text box pulled by a corner grows its letters with it.
                            layer.font_size = Some((size * w / frame.w).round().clamp(8.0, 600.0));
                        }
                        keep_on_slide(layer, h);
                    }
                }
                Mode::Rotate => {
                    if let Some((frame, layer)) = drag.start.first().and_then(|f| layers.iter_mut().find(|l| l.id == f.id).map(|l| (f, l))) {
                        let degrees = angle_to(frame.center(), to_slide(pointer), shift);
                        layer.rotation = (degrees != 0.0).then_some(degrees);
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.drag = None;
            self.guides.clear();
        }

        // The pointer says what a press would do.
        let hovered = response.hover_pos().filter(|_| self.drag.is_none());
        if let Some(p) = hovered {
            let icon = match handle_at(p) {
                Some(Mode::Rotate) => Some(CursorIcon::Crosshair),
                Some(Mode::Stretch(sx, sy)) if sx == 0.0 => Some(if sy != 0.0 { CursorIcon::ResizeVertical } else { CursorIcon::Default }),
                Some(Mode::Stretch(_, sy)) if sy == 0.0 => Some(CursorIcon::ResizeHorizontal),
                Some(Mode::Stretch(sx, sy)) => Some(if sx * sy > 0.0 { CursorIcon::ResizeNwSe } else { CursorIcon::ResizeNeSw }),
                _ => layer_at(p).map(|_| CursorIcon::Move),
            };
            if let Some(icon) = icon {
                ui.ctx().set_cursor_icon(icon);
            }
        }
        let mut action = None;
        if !self.picked.is_empty() {
            response.context_menu(|ui| {
                for (label, what) in [("Duplicate", 0), ("Copy", 1), ("Bring to front", 2), ("Send to back", 3), ("Delete", 4)] {
                    if ui.button(label).clicked() {
                        action = Some(what);
                        ui.close();
                    }
                }
            });
        }
        match action {
            Some(0) => self.duplicate_picked(),
            Some(1) => self.copy_picked(ui),
            Some(2) => self.reorder(Order::Front),
            Some(3) => self.reorder(Order::Back),
            Some(4) => self.delete_picked(),
            _ => {}
        }

        // Outlines: a faint one under the pointer, a strong one with handles on what's picked, and the guides.
        let painter = ui.painter().with_clip_rect(area);
        let frames: Vec<Frame> = self.project.slides[index].layers.iter().map(Frame::of).collect();
        let outline = |frame: &Frame, stroke: Stroke| {
            let corners: Vec<Pos2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].iter().map(|(a, d)| to_screen(frame.point(*a, *d))).collect();
            painter.add(egui::Shape::closed_line(corners, stroke));
        };
        if let Some(frame) = hovered.and_then(|p| frames.iter().rev().find(|f| f.contains(to_slide(p)))).filter(|f| !self.picked.contains(&f.id)) {
            outline(frame, Stroke::new(1.0, color::SUN.gamma_multiply(0.7)));
        }
        for frame in frames.iter().filter(|f| self.picked.contains(&f.id)) {
            outline(frame, Stroke::new(1.5, color::SUN));
        }
        if let Some(frame) = frames.iter().find(|f| self.picked.len() == 1 && f.id == self.picked[0]) {
            let knob = frame.center() + turned(vec2(0.0, -frame.h / 2.0 - 30.0 / scale), frame.rotation);
            painter.line_segment([to_screen(frame.point(0.0, -1.0)), to_screen(knob)], Stroke::new(1.0, color::SUN));
            let mut dots: Vec<Pos2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].iter().map(|(a, d)| frame.point(*a, *d)).collect();
            if frame.w * scale > 44.0 {
                dots.extend([frame.point(0.0, -1.0), frame.point(0.0, 1.0)]);
            }
            if frame.h * scale > 44.0 {
                dots.extend([frame.point(-1.0, 0.0), frame.point(1.0, 0.0)]);
            }
            dots.push(knob);
            for dot in dots {
                painter.circle(to_screen(dot), 5.0, Color32::WHITE, Stroke::new(1.5, color::SUN));
            }
            if matches!(self.drag.as_ref().map(|d| d.mode), Some(Mode::Rotate)) {
                painter.text(to_screen(knob) + vec2(12.0, -12.0), egui::Align2::LEFT_BOTTOM, format!("{:.0}°", frame.rotation), font::ui(12.0), color::INK);
            }
        }
        for guide in &self.guides {
            let (from, to) = match *guide {
                Guide::Down(x) => (pos2(x, 0.0), pos2(x, h)),
                Guide::Across(y) => (pos2(0.0, y), pos2(1080.0, y)),
            };
            painter.line_segment([to_screen(from), to_screen(to)], Stroke::new(1.0, GUIDE));
        }
    }

    /// The picked element's settings (or what several picked can do together), then every element on the slide.
    pub(super) fn inspector(&mut self, ui: &mut Ui) {
        let index = self.slide_index();
        let indices = self.picked_indices();
        let slide_h = self.project.height();
        let brand_font = self.project.font.clone();
        let focus = std::mem::take(&mut self.focus_text);
        let mut order = None;
        let mut line = None;
        let mut space = None;
        let (mut duplicate, mut delete, mut replace) = (false, false, false);
        if let [i] = indices.as_slice() {
            let layer = &mut self.project.slides[index].layers[*i];
            let title = match layer.kind.as_str() {
                "text" => "Text",
                "shape" => "Shape",
                _ => "Picture",
            };
            ui.label(egui::RichText::new(title).font(font::bold(14.0)).color(color::INK));
            match layer.kind.as_str() {
                "text" => {
                    let words = ui.add(egui::TextEdit::multiline(layer.text.get_or_insert_default()).desired_width(f32::INFINITY).desired_rows(3).char_limit(2000));
                    if focus {
                        words.request_focus();
                    }
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("folio-layer-font").selected_text(layer.font.clone().unwrap_or_else(|| format!("Brand ({brand_font})"))).width(150.0).show_ui(ui, |ui| {
                            ui.selectable_value(&mut layer.font, None, format!("Brand ({brand_font})"));
                            for name in LAYER_FONTS {
                                ui.selectable_value(&mut layer.font, Some(name.to_owned()), name);
                            }
                        });
                        ui.add(egui::DragValue::new(layer.font_size.get_or_insert(64.0)).range(8.0..=600.0).speed(1.0).suffix(" px")).on_hover_text("Letter size: drag, or type a number");
                    });
                    ui.horizontal(|ui| {
                        let mut bold = layer.bold != Some(false);
                        if ui.checkbox(&mut bold, "Bold").changed() {
                            layer.bold = (!bold).then_some(false);
                        }
                        for (label, value, tip) in [("⟸", "left", "Left"), ("⟺", "center", "Centered"), ("⟹", "right", "Right")] {
                            let on = layer.align.as_deref() == Some(value);
                            if ui.selectable_label(on, label).on_hover_text(tip).clicked() {
                                // Clicking the one that's on goes back to following the project's direction.
                                layer.align = (!on).then(|| value.to_owned());
                            }
                        }
                        color_button(ui, layer.color.get_or_insert_with(|| "#20211f".into()));
                    });
                }
                "shape" => {
                    ui.horizontal(|ui| {
                        let current = layer.shape.clone().unwrap_or_else(|| "rect".into());
                        egui::ComboBox::from_id_salt("folio-layer-shape").selected_text(SHAPES.iter().find(|s| s.0 == current).map_or("Rectangle", |s| s.1)).show_ui(ui, |ui| {
                            for (id, name) in SHAPES {
                                ui.selectable_value(&mut layer.shape, Some(id.to_owned()), name);
                            }
                        });
                        ui.label("Fill");
                        color_button(ui, layer.color.get_or_insert_with(|| "#000000".into()));
                    });
                    ui.horizontal(|ui| {
                        let mut outlined = layer.stroke.is_some();
                        if ui.checkbox(&mut outlined, "Outline").changed() {
                            (layer.stroke, layer.stroke_width) = if outlined { (Some("#20211f".into()), Some(6.0)) } else { (None, None) };
                        }
                        if let Some(stroke) = &mut layer.stroke {
                            color_button(ui, stroke);
                            ui.add(egui::DragValue::new(layer.stroke_width.get_or_insert(6.0)).range(1.0..=120.0).speed(0.5).suffix(" px"));
                        }
                    });
                }
                _ => {
                    replace = ui.button("Replace the picture…").clicked();
                }
            }
            if layer.kind == "image" || (layer.kind == "shape" && layer.shape.as_deref().is_none_or(|s| s == "rect")) {
                let most = (layer.width.min(layer.height) / 2.0).max(1.0);
                let mut radius = layer.radius.unwrap_or(0.0).min(most);
                if ui.add(egui::Slider::new(&mut radius, 0.0..=most).step_by(1.0).text("Round corners")).changed() {
                    layer.radius = (radius > 0.0).then_some(radius);
                }
            }
            let mut percent = layer.opacity() * 100.0;
            if ui.add(egui::Slider::new(&mut percent, 0.0..=100.0).step_by(1.0).suffix(" %").text("Opacity")).changed() {
                layer.opacity = (percent < 100.0).then_some(percent / 100.0);
            }
            egui::Grid::new("folio-layer-place").num_columns(4).spacing([6.0, 4.0]).show(ui, |ui| {
                ui.label("X");
                ui.add(egui::DragValue::new(&mut layer.x).speed(1.0));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut layer.y).speed(1.0));
                ui.end_row();
                ui.label("W");
                ui.add(egui::DragValue::new(&mut layer.width).range(SMALLEST..=4000.0).speed(1.0));
                ui.label("H");
                ui.add(egui::DragValue::new(&mut layer.height).range(SMALLEST..=4000.0).speed(1.0));
                ui.end_row();
                ui.label("Turn");
                let mut degrees = layer.rotation();
                if ui.add(egui::DragValue::new(&mut degrees).range(-180.0..=180.0).speed(1.0).suffix("°")).changed() {
                    layer.rotation = (degrees != 0.0).then_some(degrees);
                }
                ui.end_row();
            });
            keep_on_slide(layer, slide_h);
        } else if indices.len() > 1 {
            ui.label(egui::RichText::new(format!("{} elements", indices.len())).font(font::bold(14.0)).color(color::INK));
            ui.label(egui::RichText::new("Drag any of them to move them together.").font(font::ui(12.0)).color(color::INK_MUTED));
        }
        if !indices.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let lines = [("⇤", Line::Left, "Left"), ("↔", Line::Center, "Center across"), ("⇥", Line::Right, "Right"), ("⤒", Line::Top, "Top"), ("↕", Line::Middle, "Middle"), ("⤓", Line::Bottom, "Bottom")];
                for (label, to, tip) in lines {
                    let tip = if indices.len() == 1 { format!("{tip} of the slide") } else { format!("{tip} of the picked elements") };
                    if ui.small_button(label).on_hover_text(tip).clicked() {
                        line = Some(to);
                    }
                }
                if indices.len() > 2 {
                    if ui.small_button("Space across").on_hover_text("Even gaps, left to right").clicked() {
                        space = Some(true);
                    }
                    if ui.small_button("Space down").on_hover_text("Even gaps, top to bottom").clicked() {
                        space = Some(false);
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for (label, to, tip) in [("To front", Order::Front, "Ctrl+]"), ("Forward", Order::Forward, "]"), ("Backward", Order::Backward, "["), ("To back", Order::Back, "Ctrl+[")] {
                    if ui.small_button(label).on_hover_text(tip).clicked() {
                        order = Some(to);
                    }
                }
            });
            ui.horizontal(|ui| {
                duplicate = ui.button("Duplicate").on_hover_text("Ctrl+D").clicked();
                delete = ui.button("Delete").on_hover_text("Delete").clicked();
            });
        } else {
            ui.label(egui::RichText::new("Click an element on the slide to change it. Shift-click picks several; arrows nudge; Ctrl+D duplicates.").font(font::ui(12.0)).color(color::INK_MUTED));
        }
        if let Some(to) = line {
            self.line_up(to);
        }
        if let Some(across) = space {
            self.space_out(across);
        }
        if let Some(to) = order {
            self.reorder(to);
        }
        if duplicate {
            self.duplicate_picked();
        }
        if delete {
            self.delete_picked();
        }
        if replace {
            if let (Some(path), [i]) = (rfd::FileDialog::new().add_filter("Pictures", &["png", "jpg", "jpeg", "webp"]).pick_file(), indices.as_slice()) {
                match picture_url(&path).and_then(|url| image::image_dimensions(&path).map(|size| (url, size)).map_err(|e| e.to_string())) {
                    Ok((url, (w, h))) if w > 0 && h > 0 => {
                        // The new picture keeps the old one's width and takes its own shape.
                        let layer = &mut self.project.slides[index].layers[*i];
                        layer.src = Some(url);
                        layer.height = (layer.width * h as f32 / w as f32).round().max(SMALLEST);
                        keep_on_slide(layer, slide_h);
                    }
                    Ok(_) => self.notice = Some("The picture is empty".into()),
                    Err(error) => self.notice = Some(error),
                }
            }
        }

        // Everything on the slide, front first.
        let rows: Vec<(String, String)> = self.project.slides[index].layers.iter().rev().map(|l| (l.id.clone(), l.label())).collect();
        if !rows.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new("On this slide (front to back)").font(font::ui(12.0)).color(color::INK_MUTED));
            let shift = ui.input(|i| i.modifiers.shift);
            for (id, label) in rows {
                if ui.selectable_label(self.picked.contains(&id), label).clicked() {
                    match (shift, self.picked.iter().position(|p| p == &id)) {
                        (true, Some(at)) => drop(self.picked.remove(at)),
                        (true, None) => self.picked.push(id),
                        _ => self.picked = vec![id],
                    }
                }
            }
        }
    }
}

/// An element can hang over the slide's edge, but a strip of it stays on the slide to grab.
pub(super) fn keep_on_slide(layer: &mut Layer, slide_height: f32) {
    layer.width = layer.width.clamp(2.0, 8000.0);
    layer.height = layer.height.clamp(2.0, 8000.0);
    layer.x = layer.x.clamp(SMALLEST - layer.width, 1080.0 - SMALLEST);
    layer.y = layer.y.clamp(SMALLEST - layer.height, slide_height - SMALLEST);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: f32, y: f32, w: f32, h: f32, rotation: f32) -> Frame {
        Frame { id: "a".into(), x, y, w, h, rotation, font_size: None, shape: true, text: false }
    }

    fn close(a: Pos2, b: Pos2) -> bool {
        a.distance(b) < 0.01
    }

    #[test]
    fn a_handle_moves_its_side_and_leaves_the_opposite_one() {
        let start = frame(100.0, 100.0, 200.0, 100.0, 0.0);
        assert_eq!(stretched(&start, 1.0, 1.0, vec2(50.0, 20.0), false), (100.0, 100.0, 250.0, 120.0));
        assert_eq!(stretched(&start, -1.0, -1.0, vec2(50.0, 20.0), false), (150.0, 120.0, 150.0, 80.0));
        assert_eq!(stretched(&start, 1.0, 0.0, vec2(50.0, 999.0), false), (100.0, 100.0, 250.0, 100.0));
        // Held to its proportions, the bigger change decides.
        assert_eq!(stretched(&start, 1.0, 1.0, vec2(100.0, 10.0), true), (100.0, 100.0, 300.0, 150.0));
        // Never smaller than the smallest.
        assert_eq!(stretched(&start, 1.0, 1.0, vec2(-500.0, -500.0), false), (100.0, 100.0, SMALLEST, SMALLEST));
    }

    #[test]
    fn a_turned_element_keeps_its_opposite_corner_where_it_is() {
        let start = frame(100.0, 100.0, 200.0, 100.0, 90.0);
        let anchor = start.point(-1.0, -1.0);
        // Turned a quarter, pulling the lower-right handle down the screen makes it wider.
        let (x, y, w, h) = stretched(&start, 1.0, 1.0, vec2(0.0, 60.0), false);
        assert!((w - 260.0).abs() < 0.01 && (h - 100.0).abs() < 0.01);
        let after = frame(x, y, w, h, 90.0);
        assert!(close(after.point(-1.0, -1.0), anchor));
        assert!(start.contains(pos2(200.0, 60.0)) && !start.contains(pos2(110.0, 110.0)));
    }

    #[test]
    fn edges_snap_to_the_nearest_line_in_reach() {
        let lines = [0.0, 540.0, 1080.0];
        assert_eq!(snap([536.0, 636.0, 736.0], &lines, 6.0), Some((4.0, 540.0)));
        assert_eq!(snap([300.0, 537.0, 774.0], &lines, 6.0), Some((3.0, 540.0)));
        assert_eq!(snap([100.0, 200.0, 300.0], &lines, 6.0), None);
        // The closest wins when two are in reach.
        assert_eq!(snap([-5.0, 250.0, 538.0], &lines, 6.0), Some((2.0, 540.0)));
    }

    #[test]
    fn the_rotate_handle_gives_degrees_clockwise_from_straight_up() {
        let center = pos2(500.0, 500.0);
        assert_eq!(angle_to(center, pos2(500.0, 300.0), false), 0.0);
        assert_eq!(angle_to(center, pos2(700.0, 500.0), false), 90.0);
        assert_eq!(angle_to(center, pos2(300.0, 500.0), false), -90.0);
        assert_eq!(angle_to(center, pos2(505.0, 300.0), false), 0.0, "settles on straight up when close");
        assert_eq!(angle_to(center, pos2(560.0, 300.0), true), 15.0);
        assert_eq!(angle_to(center, pos2(500.0, 700.0), false), 180.0);
    }

    #[test]
    fn elements_stay_reachable() {
        let mut layer = Layer { x: 5000.0, y: -5000.0, width: 300.0, height: 200.0, ..Layer::default() };
        keep_on_slide(&mut layer, 1350.0);
        assert_eq!((layer.x, layer.y), (1080.0 - SMALLEST, SMALLEST - 200.0));
    }
}
