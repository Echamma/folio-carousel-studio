//! Native Folio editor: slides, their copy and pictures, the brand kit, and exports, on the same `.folio.json`
//! files as Electron Folio. Slides are drawn by `render`, the same code exports use.

use crate::model::{FONTS, LAYOUTS, Layer, Project, Slide};
use crate::render::{self, Pictures};
use crate::samples;
use crate::storage;
use base64::Engine;
use canopy_ui::{CanopyApp, Host, Place, Receipt, SharedFile, color, font};
use egui::{Color32, Rect, TextureHandle, TextureOptions, Ui, UiBuilder, pos2, vec2};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A UUID-shaped id that differs from every other this program makes. It starts with the fast-changing part of the
/// clock, because project files are named with its first 12 characters (as Electron names them with a random UUID's).
fn new_id() -> String {
    static COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let count = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 ^ count.rotate_left(17);
    let seconds = n / 1_000_000_000;
    format!("{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}", n as u32 ^ count as u32, (n >> 32) as u16, count & 0xfff, (seconds >> 12) & 0xfff, seconds)
}

/// A picture file as the data URL Folio projects embed (PNG, JPEG or WebP under 15 MB, as in Electron).
fn picture_url(path: &Path) -> Result<String, String> {
    let mime = match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return Err("Use PNG, JPEG or WebP pictures".into()),
    };
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 15 * 1024 * 1024 {
        return Err("Use photos under 15 MB".into());
    }
    Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

pub struct Folio {
    place: Place,
    project: Project,
    selected: usize,
    pictures: Pictures,
    /// The selected slide as drawn, and what it was drawn from.
    preview: Option<(TextureHandle, Project, usize)>,
    reduced: bool,
    undo: Vec<Project>,
    redo: Vec<Project>,
    edited_at: Option<Instant>,
    show_projects: bool,
    notice: Option<String>,
    /// The element chosen on the slide, and one being dragged: its id, whether by its resize corner, and where it
    /// was (x, y, width, height).
    selected_layer: Option<String>,
    element_drag: Option<(String, bool, (f32, f32, f32, f32))>,
    /// The project gallery's search, and project covers by file name (with the time they were saved).
    search: String,
    covers: std::collections::HashMap<String, (u64, TextureHandle)>,
    /// The layout picker, and its thumbnails with the brand they were drawn in (accent, theme, font).
    show_layouts: bool,
    layout_thumbs: Option<((String, String, String), Vec<TextureHandle>)>,
}

impl Folio {
    pub fn new(place: Place) -> Self {
        // A standalone Folio adds the Fun Tech carousels to its folder the first time, as Electron does.
        samples::seed(&place);
        let project = storage::list(&place).iter().max_by_key(|d| d.saved_at)
            .and_then(|doc| storage::load_document(&place, &doc.id).ok())
            .unwrap_or_else(|| Project::starter(&mut new_id));
        Self { place, project, selected: 0, pictures: Pictures::default(), preview: None, reduced: false, undo: Vec::new(), redo: Vec::new(), edited_at: None, show_projects: false, notice: None, selected_layer: None, element_drag: None,
            search: String::new(), covers: Default::default(), show_layouts: false, layout_thumbs: None }
    }

    fn open(&mut self, project: Project) {
        let _ = self.save();
        (self.project, self.selected, self.preview) = (project, 0, None);
        self.undo.clear();
        self.redo.clear();
    }

    fn slide(&mut self) -> &mut Slide {
        let index = self.selected.min(self.project.slides.len() - 1);
        &mut self.project.slides[index]
    }

    fn toolbar(&mut self, ui: &mut Ui, host: &mut dyn Host) {
        ui.horizontal(|ui| {
            if ui.button("Projects").clicked() {
                self.show_projects = true;
            }
            ui.menu_button("New", |ui| {
                let start = if ui.button("Starter carousel").on_hover_text("The art of a good carousel: five layouts to rewrite").clicked() {
                    Some(Project::starter(&mut new_id))
                } else if ui.button("Navy ribbon carousel").on_hover_text("Eight square slides with torn white ribbons").clicked() {
                    Some(samples::navy(&mut new_id))
                } else {
                    None
                };
                if let Some(project) = start {
                    self.open(project);
                    self.edited_at = Some(Instant::now());
                    ui.close();
                }
            });
            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo")).clicked() {
                if let Some(previous) = self.undo.pop() {
                    self.redo.push(std::mem::replace(&mut self.project, previous));
                }
            }
            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo")).clicked() {
                if let Some(next) = self.redo.pop() {
                    self.undo.push(std::mem::replace(&mut self.project, next));
                }
            }
            ui.add(egui::TextEdit::singleline(&mut self.project.name).desired_width(220.0).font(font::bold(14.0)));
            if ui.button("Save").clicked() {
                match self.save() {
                    Ok(()) => host.toast("Folio project saved".into()),
                    Err(error) => host.toast(error),
                }
            }
            if ui.button("Export slide").on_hover_text("This slide as a PNG, into Files").clicked() {
                self.export(host, Some(self.selected));
            }
            if ui.button("Export carousel").on_hover_text("Every slide as numbered PNGs in a ZIP (one slide: a PNG)").clicked() {
                self.export(host, None);
            }
        });
    }

    /// Electron Folio's exports: one slide as `<name>.png`; a carousel as `<name>.zip` of `01-<name>.png`…
    fn export(&mut self, host: &mut dyn Host, only: Option<usize>) {
        let name = storage::safe_name(&self.project.name);
        let slides: Vec<usize> = only.map_or_else(|| (0..self.project.slides.len()).collect(), |i| vec![i]);
        let result = (|| -> Result<std::path::PathBuf, String> {
            std::fs::create_dir_all(&self.place.exports_dir).map_err(|e| e.to_string())?;
            let pngs: Vec<Vec<u8>> = slides.iter().map(|&i| render::png(&self.project, i, &mut self.pictures)).collect::<Result<_, _>>()?;
            if pngs.len() == 1 {
                let path = self.place.exports_dir.join(canopy_ui::canopy_core::files::unique_name(&self.place.exports_dir, &format!("{name}.png")));
                std::fs::write(&path, &pngs[0]).map_err(|e| e.to_string())?;
                return Ok(path);
            }
            let path = self.place.exports_dir.join(canopy_ui::canopy_core::files::unique_name(&self.place.exports_dir, &format!("{name}.zip")));
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).map_err(|e| e.to_string())?);
            for (i, png) in pngs.iter().enumerate() {
                zip.start_file(format!("{:02}-{name}.png", i + 1), zip::write::SimpleFileOptions::default()).map_err(|e| e.to_string())?;
                std::io::Write::write_all(&mut zip, png).map_err(|e| e.to_string())?;
            }
            zip.finish().map_err(|e| e.to_string())?;
            Ok(path)
        })();
        match result {
            Ok(path) => host.exported(path),
            Err(error) => host.toast(error),
        }
    }

    fn slides(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Slides").font(font::bold(14.0)).color(color::INK));
        let count = self.project.slides.len();
        egui::ScrollArea::vertical().id_salt("folio-slides").max_height(ui.available_height() - 120.0).show(ui, |ui| {
            for i in 0..count {
                let layout = LAYOUTS.iter().find(|l| l.0 == self.project.slides[i].layout).map_or("", |l| l.1);
                let first_line = self.project.slides[i].title.lines().next().unwrap_or("").chars().take(22).collect::<String>();
                if ui.selectable_label(i == self.selected, format!("{:02}  {layout}\n     {first_line}", i + 1)).clicked() {
                    self.selected = i;
                }
            }
        });
        ui.separator();
        ui.menu_button("+ Add slide", |ui| {
            for (id, name, icon, description) in LAYOUTS {
                if ui.button(format!("{icon}  {name}")).on_hover_text(description).clicked() {
                    if count < 20 {
                        self.project.slides.insert(self.selected + 1, Slide::new(id, new_id()));
                        self.selected += 1;
                    } else {
                        self.notice = Some("A carousel can have up to 20 slides".into());
                    }
                    ui.close();
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(count < 20, egui::Button::new("Duplicate")).clicked() {
                let mut copy = self.project.slides[self.selected].clone();
                copy.id = new_id();
                self.project.slides.insert(self.selected + 1, copy);
                self.selected += 1;
            }
            if ui.add_enabled(self.selected > 0, egui::Button::new("↑")).on_hover_text("Move earlier").clicked() {
                self.project.slides.swap(self.selected, self.selected - 1);
                self.selected -= 1;
            }
            if ui.add_enabled(self.selected + 1 < count, egui::Button::new("↓")).on_hover_text("Move later").clicked() {
                self.project.slides.swap(self.selected, self.selected + 1);
                self.selected += 1;
            }
            if ui.add_enabled(count > 1, egui::Button::new("Delete")).clicked() {
                self.project.slides.remove(self.selected);
                self.selected = self.selected.min(self.project.slides.len() - 1);
            }
        });
    }

    fn details(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Slide details").font(font::bold(14.0)).color(color::INK));
        let slide = self.slide();
        let mut layout = slide.layout.clone();
        egui::ComboBox::from_label("Layout").selected_text(LAYOUTS.iter().find(|l| l.0 == layout).map_or("", |l| l.1)).show_ui(ui, |ui| {
            for (id, name, ..) in LAYOUTS {
                ui.selectable_value(&mut layout, id.to_owned(), name);
            }
        });
        let changed = layout != slide.layout;
        slide.layout = layout;
        if changed {
            self.apply_layout_extras();
        }
        if ui.button("Pick a layout…").clicked() {
            self.show_layouts = true;
        }
        let slide = self.slide();
        ui.label("Eyebrow");
        ui.add(egui::TextEdit::singleline(&mut slide.eyebrow).desired_width(f32::INFINITY));
        ui.label("Title");
        ui.add(egui::TextEdit::multiline(&mut slide.title).desired_rows(2).desired_width(f32::INFINITY));
        ui.label("Body");
        ui.add(egui::TextEdit::multiline(&mut slide.body).desired_rows(3).desired_width(f32::INFINITY));
        if slide.layout == "checklist" {
            ui.label("Checklist");
            for item in &mut slide.items {
                ui.add(egui::TextEdit::singleline(item).desired_width(f32::INFINITY));
            }
            ui.horizontal(|ui| {
                if slide.items.len() < 5 && ui.small_button("+ item").clicked() {
                    slide.items.push("Another step".into());
                }
                if slide.items.len() > 1 && ui.small_button("− item").clicked() {
                    slide.items.pop();
                }
            });
        }
        ui.add(egui::Slider::new(&mut slide.title_size, 44.0..=120.0).step_by(1.0).text("Title size"));
        ui.checkbox(&mut slide.texture, "Background texture");
        ui.checkbox(&mut slide.swipe, "Swipe cue and page number");
        let layout = slide.layout.clone();
        let pictures: &[(&str, bool)] = match layout.as_str() {
            "image-center" | "image-left" | "image-right" => &[("Picture", false)],
            "compare" => &[("Left picture", false), ("Right picture", true)],
            _ => &[],
        };
        for &(label, second) in pictures {
            ui.horizontal(|ui| {
                if ui.button(format!("{label}…")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Pictures", &["png", "jpg", "jpeg", "webp"]).pick_file() {
                        match picture_url(&path) {
                            Ok(url) => {
                                let slide = self.slide();
                                if second { slide.image2 = url } else { slide.image = url }
                            }
                            Err(error) => self.notice = Some(error),
                        }
                    }
                }
                let slide = self.slide();
                let set = if second { &mut slide.image2 } else { &mut slide.image };
                if !set.is_empty() && ui.small_button("Remove").clicked() {
                    set.clear();
                }
            });
        }
        if ui.button("+ Add image…").on_hover_text("A movable picture on this slide").clicked() {
            if let Some(path) = rfd::FileDialog::new().add_filter("Pictures", &["png", "jpg", "jpeg", "webp"]).pick_file() {
                if let Err(error) = self.add_image_layer(&path) {
                    self.notice = Some(error);
                }
            }
        }
        if ui.button("+ Add text").on_hover_text("A movable caption on this slide").clicked() {
            let dark = self.project.theme == "dark";
            let y = if self.project.format == "square" { 410.0 } else { 520.0 };
            let id = new_id();
            if self.slide().layers.len() >= 30 {
                self.notice = Some("A slide can have up to 30 extra elements.".into());
                return;
            }
            self.slide().layers.push(Layer {
                id: id.clone(), kind: "text".into(), x: 140.0, y, width: 650.0, height: 180.0, src: None, text: Some("Your text here".into()),
                color: Some(if dark { "#f9dec9" } else { "#20211f" }.into()), font_size: Some(64.0), extra: Default::default(),
            });
            self.selected_layer = Some(id);
        }
        ui.separator();
        ui.label("Elements (front to back)");
        let slide_index = self.selected.min(self.project.slides.len() - 1);
        for layer in self.project.slides[slide_index].layers.iter().rev() {
            let name = layer.text.as_deref().filter(|s| !s.is_empty()).unwrap_or("Picture");
            let label: String = name.chars().take(28).collect();
            if ui.selectable_label(self.selected_layer.as_deref() == Some(&layer.id), label).clicked() {
                self.selected_layer = Some(layer.id.clone());
            }
        }
        let selected = self.selected_layer.as_deref().and_then(|id| self.project.slides[slide_index].layers.iter().position(|layer| layer.id == id));
        if let Some(index) = selected {
            let layers = &mut self.project.slides[slide_index].layers;
            let mut remove = false;
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(index > 0, egui::Button::new("Send backward")).clicked() { layers.swap(index, index - 1); }
                if ui.add_enabled(index + 1 < layers.len(), egui::Button::new("Bring forward")).clicked() { layers.swap(index, index + 1); }
                if ui.button("Remove").clicked() { remove = true; }
            });
            let index = layers.iter().position(|layer| Some(&layer.id) == self.selected_layer.as_ref()).unwrap();
            if remove {
                layers.remove(index);
                self.selected_layer = None;
            } else if layers[index].kind == "text" {
                let layer = &mut layers[index];
                ui.label("Text");
                ui.add(egui::TextEdit::multiline(layer.text.get_or_insert_default()).desired_width(f32::INFINITY).char_limit(500));
                ui.add(egui::Slider::new(layer.font_size.get_or_insert(64.0), 24.0..=180.0).step_by(1.0).text("Size"));
                let color = layer.color.get_or_insert_with(|| "#20211f".into());
                let mut rgb = [1, 3, 5].map(|i| u8::from_str_radix(color.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0));
                ui.horizontal(|ui| {
                    ui.label("Color");
                    if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
                        *color = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                    }
                });
            }
        }
        ui.separator();
        ui.label(egui::RichText::new("Brand kit").font(font::bold(14.0)).color(color::INK));
        let project = &mut self.project;
        ui.add(egui::TextEdit::singleline(&mut project.brand).desired_width(f32::INFINITY).hint_text("Name"));
        ui.add(egui::TextEdit::singleline(&mut project.handle).desired_width(f32::INFINITY).hint_text("@handle"));
        let channel = |i: usize| u8::from_str_radix(project.accent.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
        let mut rgb = [channel(1), channel(3), channel(5)];
        ui.horizontal(|ui| {
            ui.label("Accent");
            if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
                project.accent = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
            }
            egui::ComboBox::from_id_salt("folio-font").selected_text(&project.font).show_ui(ui, |ui| {
                for name in FONTS {
                    ui.selectable_value(&mut project.font, name.to_owned(), name);
                }
            });
        });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut project.theme, "light".into(), "Light");
            ui.selectable_value(&mut project.theme, "dark".into(), "Dark");
            ui.separator();
            ui.selectable_value(&mut project.format, "portrait".into(), "Portrait");
            ui.selectable_value(&mut project.format, "square".into(), "Square");
        });
        let mut rtl = project.direction.as_deref() == Some("rtl");
        if ui.checkbox(&mut rtl, "Right-to-left (Arabic)").changed() {
            project.direction = rtl.then(|| "rtl".to_owned());
        }
    }

    /// A picture as a movable element, 440 px on its longer side, centered (Electron's `addImageLayer`).
    fn add_image_layer(&mut self, path: &Path) -> Result<(), String> {
        let url = picture_url(path)?;
        let (w, h) = image::image_dimensions(path).map_err(|e| e.to_string())?;
        let scale = 440.0 / w.max(h) as f32;
        let (width, height) = ((w as f32 * scale).round().max(40.0), (h as f32 * scale).round().max(40.0));
        let slide_h = self.project.height();
        let slide = self.slide();
        if slide.layers.len() >= 30 {
            return Err("A slide can have up to 30 extra elements.".into());
        }
        let id = new_id();
        slide.layers.push(Layer {
            id: id.clone(), kind: "image".into(), x: ((1080.0 - width) / 2.0).round(), y: ((slide_h - height) / 2.0).round(), width, height,
            src: Some(url), text: None, color: None, font_size: None, extra: Default::default(),
        });
        self.selected_layer = Some(id);
        Ok(())
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let area = ui.max_rect().shrink(12.0);
        let index = self.selected.min(self.project.slides.len() - 1);
        let stale = self.preview.as_ref().is_none_or(|(_, project, i)| *project != self.project || *i != index);
        if stale {
            let (pixmap, reduced) = render::slide(&self.project, index, &mut self.pictures);
            let image = egui::ColorImage::from_rgba_premultiplied([pixmap.width() as usize, pixmap.height() as usize], pixmap.data());
            self.reduced = reduced;
            match &mut self.preview {
                Some((texture, project, i)) => {
                    texture.set(image, TextureOptions::LINEAR);
                    (*project, *i) = (self.project.clone(), index);
                }
                None => self.preview = Some((ui.ctx().load_texture("folio-slide", image, TextureOptions::LINEAR), self.project.clone(), index)),
            }
        }
        let h = self.project.height();
        let scale = (area.width() / 1080.0).min((area.height() - 24.0) / h);
        let rect = Rect::from_center_size(pos2(area.center().x, area.top() + h * scale / 2.0), vec2(1080.0 * scale, h * scale));
        if let Some((texture, ..)) = &self.preview {
            ui.painter().image(texture.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        if self.reduced {
            ui.painter().text(pos2(rect.left(), rect.bottom() + 6.0), egui::Align2::LEFT_TOP, "Some text was made smaller or cut to fit this slide.", font::ui(12.0), color::INK_MUTED);
        }
        self.elements(ui, rect, scale);
    }

    /// Electron's element handles: drag an element to move it, its lower-right corner to resize it (a picture keeps
    /// its shape; text stretches), kept inside the slide; click to select it, topmost first.
    fn elements(&mut self, ui: &mut Ui, slide: Rect, scale: f32) {
        let h = self.project.height();
        let index = self.selected.min(self.project.slides.len() - 1);
        let to_screen = |x: f32, y: f32| pos2(slide.left() + x * scale, slide.top() + y * scale);
        let layers: Vec<(String, Rect)> = self.project.slides[index].layers.iter()
            .map(|l| (l.id.clone(), Rect::from_min_max(to_screen(l.x, l.y), to_screen(l.x + l.width, l.y + l.height))))
            .collect();
        for (id, rect) in &layers {
            let selected = self.selected_layer.as_ref() == Some(id);
            ui.painter().rect_stroke(*rect, 2.0, egui::Stroke::new(if selected { 1.5 } else { 0.8 }, if selected { color::SUN } else { color::INK_MUTED.gamma_multiply(0.6) }), egui::StrokeKind::Outside);
            if selected {
                ui.painter().rect_filled(Rect::from_center_size(rect.max, vec2(10.0, 10.0)), 2.0, color::SUN);
            }
        }
        // Topmost (last drawn) wins, as overlapping elements stack.
        for (id, rect) in layers.iter().rev() {
            let corner = Rect::from_center_size(rect.max, vec2(16.0, 16.0));
            let response = ui.interact(rect.union(corner), ui.scope_id().with(("folio-element", id)), egui::Sense::click_and_drag());
            if let Some(p) = response.hover_pos() {
                ui.ctx().set_cursor_icon(if corner.contains(p) { egui::CursorIcon::ResizeNwSe } else { egui::CursorIcon::Grab });
            }
            if response.drag_started() || response.clicked() {
                self.selected_layer = Some(id.clone());
                let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(rect.center());
                let layer = self.project.slides[index].layers.iter().find(|l| &l.id == id).unwrap();
                self.element_drag = Some((id.clone(), corner.contains(origin), (layer.x, layer.y, layer.width, layer.height)));
            }
            if response.dragged() {
                let Some((dragged, resizing, (x, y, w, h0))) = self.element_drag.clone().filter(|d| &d.0 == id) else { continue };
                let delta = response.drag_delta();
                let total = ui.input(|i| i.pointer.interact_pos()).zip(ui.input(|i| i.pointer.press_origin())).map(|(now, from)| (now - from) / scale).unwrap_or(delta);
                let Some(layer) = self.project.slides[index].layers.iter_mut().find(|l| l.id == dragged) else { continue };
                if resizing {
                    if layer.kind == "image" {
                        let factor = 1.0 + if (total.x / w).abs() > (total.y / h0).abs() { total.x / w } else { total.y / h0 };
                        let bounded = factor.clamp((40.0 / w).max(40.0 / h0), ((1080.0 - x) / w).min((h - y) / h0));
                        (layer.width, layer.height) = ((w * bounded).round(), (h0 * bounded).round());
                    } else {
                        layer.width = (w + total.x).clamp(50.0, 1080.0 - x).round();
                        layer.height = (h0 + total.y).clamp(40.0, h - y).round();
                    }
                } else {
                    layer.x = (x + total.x).clamp(0.0, 1080.0 - layer.width).round();
                    layer.y = (y + total.y).clamp(0.0, h - layer.height).round();
                }
            }
            if response.drag_stopped() {
                self.element_drag = None;
            }
            if response.contains_pointer() {
                break;
            }
        }
    }

    /// Picking the navy ribbon layout also sets its look, as Electron's layout grid does.
    fn apply_layout_extras(&mut self) {
        if self.slide().layout == "navy-ribbon" {
            (self.project.theme, self.project.accent) = ("dark".into(), "#f3ddc1".into());
            let slide = self.slide();
            slide.eyebrow.clear();
            slide.swipe = false;
        }
    }

    /// A slide drawn small, as a texture (covers and layout thumbnails).
    fn thumbnail(ctx: &egui::Context, name: String, project: &Project, index: usize, pictures: &mut Pictures, width: u32) -> TextureHandle {
        let pixmap = render::slide(project, index, pictures).0;
        let full = image::RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec()).unwrap();
        let small = image::imageops::thumbnail(&full, width, width * pixmap.height() / pixmap.width());
        let image = egui::ColorImage::from_rgba_premultiplied([small.width() as usize, small.height() as usize], small.as_raw());
        ctx.load_texture(name, image, TextureOptions::LINEAR)
    }

    /// Electron's layout grid: each layout drawn as a sample square slide in this project's brand; picking one sets
    /// the current slide's layout.
    fn layouts_window(&mut self, ui: &mut Ui) {
        if !self.show_layouts {
            return;
        }
        let brand = (self.project.accent.clone(), self.project.theme.clone(), self.project.font.clone());
        if self.layout_thumbs.as_ref().is_none_or(|(drawn, _)| *drawn != brand) {
            let thumbs = LAYOUTS.iter().map(|(id, name, ..)| {
                let navy = *id == "navy-ribbon";
                let mut slide = Slide::new(id, String::new());
                slide.eyebrow = if navy { String::new() } else { "YOUR STORY".into() };
                slide.title = match *id {
                    "navy-ribbon" => "What Is A\nQR Code?",
                    "quote" => "A thought worth sharing.",
                    "thank-you" => "Thank you.",
                    "introduction" => "Your next\nbig idea.",
                    _ => "Title space",
                }.into();
                slide.body = if navy { String::new() } else { "A little room for your ideas.".into() };
                (slide.title_size, slide.swipe) = (104.0, !navy);
                slide.items = vec!["First idea".into(), "Second idea".into(), "Third idea".into()];
                let sample = Project {
                    accent: if navy { "#f3ddc1".into() } else { self.project.accent.clone() }, format: "square".into(),
                    brand: "YOUR STUDIO".into(), handle: "@yourstudio".into(), slides: vec![slide], ..self.project.clone()
                };
                Self::thumbnail(ui.ctx(), format!("folio-layout-{name}"), &sample, 0, &mut self.pictures, 180)
            }).collect();
            self.layout_thumbs = Some((brand, thumbs));
        }
        let current = self.slide().layout.clone();
        let mut picked = None;
        let modal = egui::Modal::new(ui.scope_id().with("folio-layouts")).show(ui.ctx(), |ui| {
            ui.set_width(620.0);
            ui.label(egui::RichText::new("Pick a layout").font(font::bold(18.0)));
            let thumbs = &self.layout_thumbs.as_ref().unwrap().1;
            egui::Grid::new("folio-layout-grid").spacing([10.0, 10.0]).show(ui, |ui| {
                for (i, ((id, name, _, description), texture)) in LAYOUTS.iter().zip(thumbs).enumerate() {
                    ui.vertical(|ui| {
                        let image = egui::Image::new(texture).fit_to_exact_size(vec2(110.0, 110.0));
                        let response = ui.add(egui::Button::image(image).selected(*id == current)).on_hover_text(*description);
                        ui.label(egui::RichText::new(*name).font(font::ui(12.0)));
                        if response.clicked() {
                            picked = Some(id.to_string());
                        }
                    });
                    if i % 5 == 4 {
                        ui.end_row();
                    }
                }
            });
        });
        if let Some(id) = picked {
            self.slide().layout = id;
            self.apply_layout_extras();
            self.show_layouts = false;
        } else if modal.should_close() {
            self.show_layouts = false;
        }
    }

    /// Electron's project gallery: every project in the folder by its cover, searchable by name.
    fn projects_window(&mut self, ui: &mut Ui) {
        if !self.show_projects {
            return;
        }
        let docs = storage::list(&self.place);
        // One new cover a frame, so a big folder opens at once and fills in.
        if let Some(doc) = docs.iter().find(|d| self.covers.get(&d.id).is_none_or(|(at, _)| *at != d.saved_at)) {
            if let Ok(project) = storage::load_document(&self.place, &doc.id) {
                let texture = Self::thumbnail(ui.ctx(), format!("folio-cover-{}", doc.id), &project, 0, &mut self.pictures, 160);
                self.covers.insert(doc.id.clone(), (doc.saved_at, texture));
            }
            ui.ctx().request_repaint();
        }
        let mut opened = None;
        let modal = egui::Modal::new(ui.scope_id().with("folio-projects")).show(ui.ctx(), |ui| {
            ui.set_width(640.0);
            ui.label(egui::RichText::new("Open a project").font(font::bold(18.0)));
            ui.label(egui::RichText::new("Choose a carousel by its cover to continue editing.").color(color::INK_MUTED));
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search projects").desired_width(f32::INFINITY));
            let query = self.search.to_lowercase();
            egui::ScrollArea::vertical().id_salt("folio-gallery").max_height(460.0).show(ui, |ui| {
                // Four covers a row (a wrapping layout doesn't wrap columns of widgets).
                egui::Grid::new("folio-gallery-grid").spacing([10.0, 12.0]).show(ui, |ui| {
                    for (i, doc) in docs.iter().filter(|d| d.name.to_lowercase().contains(&query)).enumerate() {
                        if i > 0 && i % 4 == 0 {
                            ui.end_row();
                        }
                        ui.vertical(|ui| {
                            ui.set_width(140.0);
                            let clicked = match self.covers.get(&doc.id) {
                                Some((_, texture)) => ui.add(egui::Button::image(egui::Image::new(texture).fit_to_exact_size(vec2(130.0, 162.0)))).clicked(),
                                None => ui.add_sized([130.0, 162.0], egui::Button::new("…")).clicked(),
                            };
                            let name: String = doc.name.chars().take(34).collect();
                            ui.add(egui::Label::new(egui::RichText::new(name).font(font::ui(12.0))).wrap());
                            ui.label(egui::RichText::new(&doc.detail).font(font::ui(11.0)).color(color::INK_MUTED));
                            if clicked {
                                opened = Some(doc.id.clone());
                            }
                        });
                    }
                });
            });
        });
        if let Some(id) = opened {
            match storage::load_document(&self.place, &id) {
                Ok(project) => self.open(project),
                Err(error) => self.notice = Some(error),
            }
            self.show_projects = false;
        } else if modal.should_close() {
            self.show_projects = false;
        }
    }
}

impl CanopyApp for Folio {
    fn ui(&mut self, ui: &mut Ui, host: &mut dyn Host) {
        let before = self.project.clone();
        if host.focused() && !ui.ctx().egui_wants_keyboard_input() {
            let (undo, redo, left, right) = ui.input(|i| (
                i.modifiers.command && i.key_pressed(egui::Key::Z) && !i.modifiers.shift,
                i.modifiers.command && (i.key_pressed(egui::Key::Y) || (i.modifiers.shift && i.key_pressed(egui::Key::Z))),
                i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown),
            ));
            if undo {
                if let Some(previous) = self.undo.pop() {
                    self.redo.push(std::mem::replace(&mut self.project, previous));
                }
            } else if redo {
                if let Some(next) = self.redo.pop() {
                    self.undo.push(std::mem::replace(&mut self.project, next));
                }
            }
            if left {
                self.selected = self.selected.saturating_sub(1);
            }
            if right {
                self.selected = (self.selected + 1).min(self.project.slides.len() - 1);
            }
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            if let Err(error) = self.save() {
                host.toast(error);
            }
        }
        let rect = ui.max_rect();
        let top = Rect::from_min_size(rect.min, vec2(rect.width(), 40.0));
        ui.scope_builder(UiBuilder::new().max_rect(top.shrink2(vec2(6.0, 4.0))), |ui| self.toolbar(ui, host));
        let body = Rect::from_min_max(pos2(rect.left(), top.bottom() + 4.0), rect.max);
        let left = Rect::from_min_size(body.min, vec2(200.0, body.height()));
        let right = Rect::from_min_max(pos2(body.right() - 280.0, body.top()), body.max);
        let middle = Rect::from_min_max(pos2(left.right() + 6.0, body.top()), pos2(right.left() - 6.0, body.bottom()));
        for panel in [left, right] {
            ui.painter().rect_filled(panel, 8.0, Color32::from_rgb(31, 37, 48));
        }
        ui.scope_builder(UiBuilder::new().max_rect(left.shrink(8.0)), |ui| self.slides(ui));
        ui.scope_builder(UiBuilder::new().max_rect(right.shrink(8.0)), |ui| {
            egui::ScrollArea::vertical().id_salt("folio-details").show(ui, |ui| self.details(ui));
        });
        ui.scope_builder(UiBuilder::new().max_rect(middle), |ui| self.canvas(ui));
        self.projects_window(ui);
        self.layouts_window(ui);
        if let Some(notice) = self.notice.take() {
            host.toast(notice);
        }
        if self.project != before {
            // Typing and dragging make one undo step until the keyboard or pointer rests.
            if self.undo.last() != Some(&before) {
                self.undo.push(before);
                if self.undo.len() > 100 {
                    self.undo.remove(0);
                }
            }
            self.redo.clear();
            self.edited_at = Some(Instant::now());
        }
        match self.edited_at {
            Some(at) if at.elapsed() > Duration::from_millis(800) => {
                if let Err(error) = self.save() {
                    host.toast(format!("Folio couldn't save: {error}"));
                }
            }
            Some(_) => ui.ctx().request_repaint_after(Duration::from_millis(850)),
            None => {}
        }
    }

    fn receive(&mut self, files: Vec<SharedFile>, host: &mut dyn Host) -> Receipt {
        let mut receipt = Receipt::default();
        for file in files {
            let name = file.name.to_lowercase();
            let taken = if name.ends_with(".folio.json") {
                storage::load(&file.path).map(|mut project| {
                    project.extra.remove("folderFileName");
                    for slide in &mut project.slides {
                        slide.id = new_id();
                    }
                    self.open(project);
                    self.edited_at = Some(Instant::now());
                })
            } else {
                self.add_image_layer(&file.path)
            };
            match taken {
                Ok(()) => receipt.accepted += 1,
                Err(_) => receipt.skipped.push(file.name),
            }
        }
        if receipt.accepted > 0 {
            host.toast(format!("Folio took {} file(s)", receipt.accepted));
        }
        receipt
    }

    fn open_document(&mut self, id: &str, host: &mut dyn Host) {
        match storage::load_document(&self.place, id) {
            Ok(project) => self.open(project),
            Err(error) => host.toast(error),
        }
    }

    fn save(&mut self) -> Result<(), String> {
        self.edited_at = None;
        storage::save(&self.place, &mut self.project, new_id).map(|_| ())
    }

    fn title(&self) -> String {
        self.project.name.clone()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ids_name_files_apart() {
        let ids: Vec<String> = (0..1000).map(|_| super::new_id()).collect();
        let prefixes: std::collections::HashSet<&str> = ids.iter().map(|id| &id[..12]).collect();
        assert_eq!(prefixes.len(), ids.len());
    }
}
