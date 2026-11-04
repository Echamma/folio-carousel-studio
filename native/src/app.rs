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
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        ^ count.rotate_left(17);
    let seconds = n / 1_000_000_000;
    format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        n as u32 ^ count as u32,
        (n >> 32) as u16,
        count & 0xfff,
        (seconds >> 12) & 0xfff,
        seconds
    )
}

/// The Figma reference names, in the order of the first nine layouts (Electron's `referenceNames`).
const REFERENCE_NAMES: [&str; 9] = [
    "Introduction",
    "Text Only",
    "Checklist",
    "Quote",
    "Image Center",
    "Image Left",
    "Image Right",
    "Image Compare",
    "Thank You",
];

/// Electron Folio's bundled Figma references: `assets\references` next to the exe or above it, or in this checkout.
fn references_folder() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok();
    let beside = exe
        .iter()
        .flat_map(|exe| exe.ancestors().skip(1).take(6))
        .map(|dir| dir.join("assets").join("references"));
    let checkout = std::iter::once(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("assets")
            .join("references"),
    );
    beside.chain(checkout).find(|dir| dir.is_dir())
}

/// A reference drawn about 260 px tall: SVGs through resvg, pictures through `image`.
fn reference_picture(path: &Path) -> Option<egui::ColorImage> {
    let image = if path.extension().is_some_and(|e| e == "svg") {
        let tree = resvg::usvg::Tree::from_data(
            &std::fs::read(path).ok()?,
            &resvg::usvg::Options::default(),
        )
        .ok()?;
        let size = tree.size();
        let scale = 260.0 / size.height();
        let mut pixmap = tiny_skia::Pixmap::new((size.width() * scale).ceil() as u32, 260)?;
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        image::RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.take())?
    } else {
        let picture = image::open(path).ok()?;
        picture.thumbnail(1000, 260).to_rgba8()
    };
    Some(egui::ColorImage::from_rgba_premultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    ))
}

/// A picture file as the data URL Folio projects embed (PNG, JPEG or WebP under 15 MB, as in Electron).
fn picture_url(path: &Path) -> Result<String, String> {
    let mime = match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return Err("Use PNG, JPEG or WebP pictures".into()),
    };
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 15 * 1024 * 1024 {
        return Err("Use photos under 15 MB".into());
    }
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

pub struct Folio {
    place: Place,
    project: Project,
    selected: usize,
    pictures: Pictures,
    /// The selected slide as drawn, and what it was drawn from.
    preview: Option<(TextureHandle, Project, usize)>,
    reduced: bool,
    /// Last drawn slide rectangle in window coordinates, for file drop placement.
    slide_rect: Option<Rect>,
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
    /// The preview dialog's slide, and its picture (with what it was drawn from).
    preview_index: Option<usize>,
    preview_big: Option<(TextureHandle, Project, usize)>,
    /// The Figma reference library: open, the size and theme shown, and pictures read so far by file name.
    show_references: bool,
    reference_look: (String, String),
    reference_pictures: std::collections::HashMap<String, Option<TextureHandle>>,
}

impl Folio {
    pub fn new(place: Place) -> Self {
        // A standalone Folio adds the Fun Tech carousels to its folder the first time, as Electron does.
        samples::seed(&place);
        let documents = storage::list(&place);
        let latest = documents.iter().max_by_key(|d| d.saved_at);
        let project = storage::load_recovery(&place, latest.map_or(0, |d| d.saved_at))
            .or_else(|| latest.and_then(|doc| storage::load_document(&place, &doc.id).ok()))
            .unwrap_or_else(|| Project::starter(&mut new_id));
        Self {
            place,
            project,
            selected: 0,
            pictures: Pictures::default(),
            preview: None,
            reduced: false,
            slide_rect: None,
            undo: Vec::new(),
            redo: Vec::new(),
            edited_at: None,
            show_projects: false,
            notice: None,
            selected_layer: None,
            element_drag: None,
            search: String::new(),
            covers: Default::default(),
            show_layouts: false,
            layout_thumbs: None,
            preview_index: None,
            preview_big: None,
            show_references: false,
            reference_look: (String::new(), String::new()),
            reference_pictures: Default::default(),
        }
    }

    fn open(&mut self, project: Project) -> Result<(), String> {
        self.save()
            .map_err(|error| format!("Couldn't save the current project: {error}"))?;
        (self.project, self.selected, self.preview) = (project, 0, None);
        self.selected_layer = None;
        self.undo.clear();
        self.redo.clear();
        Ok(())
    }

    fn receive_files(
        &mut self,
        files: Vec<SharedFile>,
        at: Option<(f32, f32)>,
        host: &mut dyn Host,
    ) -> Receipt {
        let mut receipt = Receipt::default();
        for file in files {
            let name = file.name.to_lowercase();
            let taken = if name.ends_with(".folio.json") {
                storage::load(&file.path).and_then(|mut project| {
                    project.extra.remove("folderFileName");
                    for slide in &mut project.slides {
                        slide.id = new_id();
                    }
                    self.open(project)?;
                    self.edited_at = Some(Instant::now());
                    Ok(())
                })
            } else {
                let before = self.project.clone();
                let result = picture_url(&file.path).and_then(|url| {
                    let (w, h) = image::image_dimensions(&file.path).map_err(|e| e.to_string())?;
                    self.add_image_data(url, w, h, at)
                });
                if result.is_ok() {
                    self.undo.push(before);
                    if self.undo.len() > 100 {
                        self.undo.remove(0);
                    }
                    self.redo.clear();
                    self.edited_at = Some(Instant::now());
                }
                result
            };
            match taken {
                Ok(()) => receipt.accepted += 1,
                Err(error) => {
                    receipt.skipped.push(file.name);
                    receipt.note = Some(error);
                }
            }
        }
        if receipt.accepted > 0 {
            host.toast(format!("Folio took {} file(s)", receipt.accepted));
        }
        receipt
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
                let start = if ui
                    .button("Starter carousel")
                    .on_hover_text("The art of a good carousel: five layouts to rewrite")
                    .clicked()
                {
                    Some(Project::starter(&mut new_id))
                } else if ui
                    .button("Navy ribbon carousel")
                    .on_hover_text("Eight square slides with torn white ribbons")
                    .clicked()
                {
                    Some(samples::navy(&mut new_id))
                } else {
                    None
                };
                if let Some(project) = start {
                    match self.open(project) {
                        Ok(()) => self.edited_at = Some(Instant::now()),
                        Err(error) => self.notice = Some(error),
                    }
                    ui.close();
                }
            });
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                .clicked()
            {
                if let Some(previous) = self.undo.pop() {
                    self.redo
                        .push(std::mem::replace(&mut self.project, previous));
                }
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                .clicked()
            {
                if let Some(next) = self.redo.pop() {
                    self.undo.push(std::mem::replace(&mut self.project, next));
                }
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.project.name)
                    .desired_width(220.0)
                    .font(font::bold(14.0)),
            );
            if ui.button("Save").clicked() {
                match self.save() {
                    Ok(()) => host.toast("Folio project saved".into()),
                    Err(error) => host.toast(error),
                }
            }
            if ui
                .button("Preview")
                .on_hover_text("Swipe through the carousel")
                .clicked()
            {
                self.preview_index = Some(self.selected);
            }
            ui.menu_button("File", |ui| {
                if ui.button("Open a project file…").clicked() {
                    ui.close();
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Folio project", &["json"])
                        .pick_file()
                    {
                        // Opened as a new project in the folder, as Electron imports one.
                        match storage::load(&path) {
                            Ok(mut project) => {
                                project.extra.remove("folderFileName");
                                project.extra.remove("libraryId");
                                match self.open(project) {
                                    Ok(()) => self.edited_at = Some(Instant::now()),
                                    Err(error) => self.notice = Some(error),
                                }
                            }
                            Err(error) => self.notice = Some(error),
                        }
                    }
                }
                if ui.button("Save a copy to file…").clicked() {
                    ui.close();
                    let name = format!("{}.folio.json", storage::safe_name(&self.project.name));
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(name)
                        .add_filter("Folio project", &["json"])
                        .save_file()
                    {
                        let mut copy = self.project.clone();
                        copy.extra.remove("folderFileName");
                        let written = serde_json::to_string_pretty(&copy)
                            .map_err(|e| e.to_string())
                            .and_then(|json| {
                                std::fs::write(&path, json).map_err(|e| e.to_string())
                            });
                        self.notice = Some(match written {
                            Ok(()) => "Project file saved.".into(),
                            Err(error) => error,
                        });
                    }
                }
            });
            if ui
                .button("Export slide")
                .on_hover_text("This slide as a PNG, into Files")
                .clicked()
            {
                self.export(host, Some(self.selected));
            }
            if ui
                .button("Export carousel")
                .on_hover_text("Every slide as numbered PNGs in a ZIP (one slide: a PNG)")
                .clicked()
            {
                self.export(host, None);
            }
        });
    }

    /// Electron Folio's exports: one slide as `<name>.png`; a carousel as `<name>.zip` of `01-<name>.png`…
    fn export(&mut self, host: &mut dyn Host, only: Option<usize>) {
        let name = storage::safe_name(&self.project.name);
        let slides: Vec<usize> =
            only.map_or_else(|| (0..self.project.slides.len()).collect(), |i| vec![i]);
        let result = (|| -> Result<std::path::PathBuf, String> {
            std::fs::create_dir_all(&self.place.exports_dir).map_err(|e| e.to_string())?;
            let pngs: Vec<Vec<u8>> = slides
                .iter()
                .map(|&i| render::png(&self.project, i, &mut self.pictures))
                .collect::<Result<_, _>>()?;
            if pngs.len() == 1 {
                let path = self
                    .place
                    .exports_dir
                    .join(canopy_ui::canopy_core::files::unique_name(
                        &self.place.exports_dir,
                        &format!("{name}.png"),
                    ));
                std::fs::write(&path, &pngs[0]).map_err(|e| e.to_string())?;
                return Ok(path);
            }
            let path = self
                .place
                .exports_dir
                .join(canopy_ui::canopy_core::files::unique_name(
                    &self.place.exports_dir,
                    &format!("{name}.zip"),
                ));
            let mut zip =
                zip::ZipWriter::new(std::fs::File::create(&path).map_err(|e| e.to_string())?);
            for (i, png) in pngs.iter().enumerate() {
                zip.start_file(
                    format!("{:02}-{name}.png", i + 1),
                    zip::write::SimpleFileOptions::default(),
                )
                .map_err(|e| e.to_string())?;
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
        ui.label(
            egui::RichText::new("Slides")
                .font(font::bold(14.0))
                .color(color::INK),
        );
        let count = self.project.slides.len();
        egui::ScrollArea::vertical()
            .id_salt("folio-slides")
            .max_height(ui.available_height() - 120.0)
            .show(ui, |ui| {
                for i in 0..count {
                    let layout = LAYOUTS
                        .iter()
                        .find(|l| l.0 == self.project.slides[i].layout)
                        .map_or("", |l| l.1);
                    let first_line = self.project.slides[i]
                        .title
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(22)
                        .collect::<String>();
                    if ui
                        .selectable_label(
                            i == self.selected,
                            format!("{:02}  {layout}\n     {first_line}", i + 1),
                        )
                        .clicked()
                    {
                        self.selected = i;
                    }
                }
            });
        ui.separator();
        ui.menu_button("+ Add slide", |ui| {
            for (id, name, icon, description) in LAYOUTS {
                if ui
                    .button(format!("{icon}  {name}"))
                    .on_hover_text(description)
                    .clicked()
                {
                    if count < 20 {
                        self.project
                            .slides
                            .insert(self.selected + 1, Slide::new(id, new_id()));
                        self.selected += 1;
                    } else {
                        self.notice = Some("A carousel can have up to 20 slides".into());
                    }
                    ui.close();
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(count < 20, egui::Button::new("Duplicate"))
                .clicked()
            {
                let mut copy = self.project.slides[self.selected].clone();
                copy.id = new_id();
                self.project.slides.insert(self.selected + 1, copy);
                self.selected += 1;
            }
            if ui
                .add_enabled(self.selected > 0, egui::Button::new("↑"))
                .on_hover_text("Move earlier")
                .clicked()
            {
                self.project.slides.swap(self.selected, self.selected - 1);
                self.selected -= 1;
            }
            if ui
                .add_enabled(self.selected + 1 < count, egui::Button::new("↓"))
                .on_hover_text("Move later")
                .clicked()
            {
                self.project.slides.swap(self.selected, self.selected + 1);
                self.selected += 1;
            }
            if ui
                .add_enabled(count > 1, egui::Button::new("Delete"))
                .clicked()
            {
                self.project.slides.remove(self.selected);
                self.selected = self.selected.min(self.project.slides.len() - 1);
            }
        });
    }

    fn details(&mut self, ui: &mut Ui) {
        ui.label(
            egui::RichText::new("Slide details")
                .font(font::bold(14.0))
                .color(color::INK),
        );
        let slide = self.slide();
        let mut layout = slide.layout.clone();
        egui::ComboBox::from_label("Layout")
            .selected_text(LAYOUTS.iter().find(|l| l.0 == layout).map_or("", |l| l.1))
            .show_ui(ui, |ui| {
                for (id, name, ..) in LAYOUTS {
                    ui.selectable_value(&mut layout, id.to_owned(), name);
                }
            });
        let changed = layout != slide.layout;
        slide.layout = layout;
        if changed {
            self.apply_layout_extras();
        }
        ui.horizontal(|ui| {
            if ui.button("Pick a layout…").clicked() {
                self.show_layouts = true;
            }
            if references_folder().is_some()
                && ui
                    .button("Figma references…")
                    .on_hover_text("The 36 original layout references")
                    .clicked()
            {
                self.show_references = true;
                self.reference_look = (
                    if self.project.format == "square" {
                        "Square"
                    } else {
                        "Portrait"
                    }
                    .into(),
                    if self.project.theme == "dark" {
                        "Dark"
                    } else {
                        "Light"
                    }
                    .into(),
                );
            }
        });
        let slide = self.slide();
        ui.label("Eyebrow");
        ui.add(egui::TextEdit::singleline(&mut slide.eyebrow).desired_width(f32::INFINITY));
        ui.label("Title");
        ui.add(
            egui::TextEdit::multiline(&mut slide.title)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );
        ui.label("Body");
        ui.add(
            egui::TextEdit::multiline(&mut slide.body)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
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
        ui.add(
            egui::Slider::new(&mut slide.title_size, 44.0..=120.0)
                .step_by(1.0)
                .text("Title size"),
        );
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
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Pictures", &["png", "jpg", "jpeg", "webp"])
                        .pick_file()
                    {
                        match picture_url(&path) {
                            Ok(url) => {
                                let slide = self.slide();
                                if second {
                                    slide.image2 = url
                                } else {
                                    slide.image = url
                                }
                            }
                            Err(error) => self.notice = Some(error),
                        }
                    }
                }
                let slide = self.slide();
                let set = if second {
                    &mut slide.image2
                } else {
                    &mut slide.image
                };
                if !set.is_empty() && ui.small_button("Remove").clicked() {
                    set.clear();
                }
            });
        }
        if ui
            .button("+ Add image…")
            .on_hover_text("A movable picture on this slide")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Pictures", &["png", "jpg", "jpeg", "webp"])
                .pick_file()
            {
                if let Err(error) = self.add_image_layer(&path) {
                    self.notice = Some(error);
                }
            }
        }
        if ui
            .button("+ Add text")
            .on_hover_text("A movable caption on this slide")
            .clicked()
        {
            let dark = self.project.theme == "dark";
            let y = if self.project.format == "square" {
                410.0
            } else {
                520.0
            };
            let id = new_id();
            if self.slide().layers.len() >= 30 {
                self.notice = Some("A slide can have up to 30 extra elements.".into());
                return;
            }
            self.slide().layers.push(Layer {
                id: id.clone(),
                kind: "text".into(),
                x: 140.0,
                y,
                width: 650.0,
                height: 180.0,
                src: None,
                text: Some("Your text here".into()),
                color: Some(if dark { "#f9dec9" } else { "#20211f" }.into()),
                font_size: Some(64.0),
                extra: Default::default(),
            });
            self.selected_layer = Some(id);
        }
        ui.separator();
        ui.label("Elements (front to back)");
        let slide_index = self.selected.min(self.project.slides.len() - 1);
        for layer in self.project.slides[slide_index].layers.iter().rev() {
            let name = layer
                .text
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or("Picture");
            let label: String = name.chars().take(28).collect();
            if ui
                .selectable_label(self.selected_layer.as_deref() == Some(&layer.id), label)
                .clicked()
            {
                self.selected_layer = Some(layer.id.clone());
            }
        }
        let selected = self.selected_layer.as_deref().and_then(|id| {
            self.project.slides[slide_index]
                .layers
                .iter()
                .position(|layer| layer.id == id)
        });
        if let Some(index) = selected {
            let layers = &mut self.project.slides[slide_index].layers;
            let mut remove = false;
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(index > 0, egui::Button::new("Send backward"))
                    .clicked()
                {
                    layers.swap(index, index - 1);
                }
                if ui
                    .add_enabled(index + 1 < layers.len(), egui::Button::new("Bring forward"))
                    .clicked()
                {
                    layers.swap(index, index + 1);
                }
                if ui.button("Remove").clicked() {
                    remove = true;
                }
            });
            let index = layers
                .iter()
                .position(|layer| Some(&layer.id) == self.selected_layer.as_ref())
                .unwrap();
            if remove {
                layers.remove(index);
                self.selected_layer = None;
            } else if layers[index].kind == "text" {
                let layer = &mut layers[index];
                ui.label("Text");
                ui.add(
                    egui::TextEdit::multiline(layer.text.get_or_insert_default())
                        .desired_width(f32::INFINITY)
                        .char_limit(500),
                );
                ui.add(
                    egui::Slider::new(layer.font_size.get_or_insert(64.0), 24.0..=180.0)
                        .step_by(1.0)
                        .text("Size"),
                );
                let color = layer.color.get_or_insert_with(|| "#20211f".into());
                let mut rgb = [1, 3, 5].map(|i| {
                    u8::from_str_radix(color.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0)
                });
                ui.horizontal(|ui| {
                    ui.label("Color");
                    if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
                        *color = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                    }
                });
            }
        }
        ui.separator();
        ui.label(
            egui::RichText::new("Brand kit")
                .font(font::bold(14.0))
                .color(color::INK),
        );
        let project = &mut self.project;
        ui.add(
            egui::TextEdit::singleline(&mut project.brand)
                .desired_width(f32::INFINITY)
                .hint_text("Name"),
        );
        ui.add(
            egui::TextEdit::singleline(&mut project.handle)
                .desired_width(f32::INFINITY)
                .hint_text("@handle"),
        );
        let channel = |i: usize| {
            u8::from_str_radix(project.accent.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0)
        };
        let mut rgb = [channel(1), channel(3), channel(5)];
        ui.horizontal(|ui| {
            ui.label("Accent");
            if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
                project.accent = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
            }
            egui::ComboBox::from_id_salt("folio-font")
                .selected_text(&project.font)
                .show_ui(ui, |ui| {
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
        self.add_image_data(url, w, h, None)
    }

    fn add_image_data(
        &mut self,
        url: String,
        w: u32,
        h: u32,
        at: Option<(f32, f32)>,
    ) -> Result<(), String> {
        if w == 0 || h == 0 {
            return Err("The picture is empty".into());
        }
        let scale = 440.0 / w.max(h) as f32;
        let (width, height) = (
            (w as f32 * scale).round().max(40.0),
            (h as f32 * scale).round().max(40.0),
        );
        let slide_h = self.project.height();
        let slide = self.slide();
        if slide.layers.len() >= 30 {
            return Err("A slide can have up to 30 extra elements.".into());
        }
        let id = new_id();
        let (x, y) = at.unwrap_or(((1080.0 - width) / 2.0, (slide_h - height) / 2.0));
        slide.layers.push(Layer {
            id: id.clone(),
            kind: "image".into(),
            x: x.clamp(0.0, 1080.0 - width).round(),
            y: y.clamp(0.0, slide_h - height).round(),
            width,
            height,
            src: Some(url),
            text: None,
            color: None,
            font_size: None,
            extra: Default::default(),
        });
        self.selected_layer = Some(id);
        Ok(())
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let area = ui.max_rect().shrink(12.0);
        let index = self.selected.min(self.project.slides.len() - 1);
        let stale = self
            .preview
            .as_ref()
            .is_none_or(|(_, project, i)| *project != self.project || *i != index);
        if stale {
            let (pixmap, reduced) = render::slide(&self.project, index, &mut self.pictures);
            let image = egui::ColorImage::from_rgba_premultiplied(
                [pixmap.width() as usize, pixmap.height() as usize],
                pixmap.data(),
            );
            self.reduced = reduced;
            match &mut self.preview {
                Some((texture, project, i)) => {
                    texture.set(image, TextureOptions::LINEAR);
                    (*project, *i) = (self.project.clone(), index);
                }
                None => {
                    self.preview = Some((
                        ui.ctx()
                            .load_texture("folio-slide", image, TextureOptions::LINEAR),
                        self.project.clone(),
                        index,
                    ))
                }
            }
        }
        let h = self.project.height();
        let scale = (area.width() / 1080.0).min((area.height() - 24.0) / h);
        let rect = Rect::from_center_size(
            pos2(area.center().x, area.top() + h * scale / 2.0),
            vec2(1080.0 * scale, h * scale),
        );
        self.slide_rect = Some(rect);
        if let Some((texture, ..)) = &self.preview {
            ui.painter().image(
                texture.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        if self.reduced {
            ui.painter().text(
                pos2(rect.left(), rect.bottom() + 6.0),
                egui::Align2::LEFT_TOP,
                "Some text was made smaller or cut to fit this slide.",
                font::ui(12.0),
                color::INK_MUTED,
            );
        }
        self.elements(ui, rect, scale);
    }

    /// Electron's element handles: drag an element to move it, its lower-right corner to resize it (a picture keeps
    /// its shape; text stretches), kept inside the slide; click to select it, topmost first.
    fn elements(&mut self, ui: &mut Ui, slide: Rect, scale: f32) {
        let h = self.project.height();
        let index = self.selected.min(self.project.slides.len() - 1);
        let to_screen = |x: f32, y: f32| pos2(slide.left() + x * scale, slide.top() + y * scale);
        let layers: Vec<(String, Rect)> = self.project.slides[index]
            .layers
            .iter()
            .map(|l| {
                (
                    l.id.clone(),
                    Rect::from_min_max(
                        to_screen(l.x, l.y),
                        to_screen(l.x + l.width, l.y + l.height),
                    ),
                )
            })
            .collect();
        for (id, rect) in &layers {
            let selected = self.selected_layer.as_ref() == Some(id);
            ui.painter().rect_stroke(
                *rect,
                2.0,
                egui::Stroke::new(
                    if selected { 1.5 } else { 0.8 },
                    if selected {
                        color::SUN
                    } else {
                        color::INK_MUTED.gamma_multiply(0.6)
                    },
                ),
                egui::StrokeKind::Outside,
            );
            if selected {
                ui.painter().rect_filled(
                    Rect::from_center_size(rect.max, vec2(10.0, 10.0)),
                    2.0,
                    color::SUN,
                );
            }
        }
        // Topmost (last drawn) wins, as overlapping elements stack.
        for (id, rect) in layers.iter().rev() {
            let corner = Rect::from_center_size(rect.max, vec2(16.0, 16.0));
            let response = ui.interact(
                rect.union(corner),
                ui.scope_id().with(("folio-element", id)),
                egui::Sense::click_and_drag(),
            );
            if let Some(p) = response.hover_pos() {
                ui.ctx().set_cursor_icon(if corner.contains(p) {
                    egui::CursorIcon::ResizeNwSe
                } else {
                    egui::CursorIcon::Grab
                });
            }
            if response.drag_started() || response.clicked() {
                self.selected_layer = Some(id.clone());
                let origin = ui
                    .input(|i| i.pointer.press_origin())
                    .unwrap_or(rect.center());
                let layer = self.project.slides[index]
                    .layers
                    .iter()
                    .find(|l| &l.id == id)
                    .unwrap();
                self.element_drag = Some((
                    id.clone(),
                    corner.contains(origin),
                    (layer.x, layer.y, layer.width, layer.height),
                ));
            }
            if response.dragged() {
                let Some((dragged, resizing, (x, y, w, h0))) =
                    self.element_drag.clone().filter(|d| &d.0 == id)
                else {
                    continue;
                };
                let delta = response.drag_delta();
                let total = ui
                    .input(|i| i.pointer.interact_pos())
                    .zip(ui.input(|i| i.pointer.press_origin()))
                    .map(|(now, from)| (now - from) / scale)
                    .unwrap_or(delta);
                let Some(layer) = self.project.slides[index]
                    .layers
                    .iter_mut()
                    .find(|l| l.id == dragged)
                else {
                    continue;
                };
                if resizing {
                    if layer.kind == "image" {
                        let factor = 1.0
                            + if (total.x / w).abs() > (total.y / h0).abs() {
                                total.x / w
                            } else {
                                total.y / h0
                            };
                        let bounded = factor.clamp(
                            (40.0 / w).max(40.0 / h0),
                            ((1080.0 - x) / w).min((h - y) / h0),
                        );
                        (layer.width, layer.height) =
                            ((w * bounded).round(), (h0 * bounded).round());
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

    /// Electron's preview: one slide at a time, large, with arrows (and the arrow keys) to swipe through.
    fn preview_window(&mut self, ui: &mut Ui) {
        let Some(index) = self.preview_index else {
            return;
        };
        let count = self.project.slides.len();
        let index = index.min(count - 1);
        if self
            .preview_big
            .as_ref()
            .is_none_or(|(_, project, i)| *project != self.project || *i != index)
        {
            let pixmap = render::slide(&self.project, index, &mut self.pictures).0;
            let image = egui::ColorImage::from_rgba_premultiplied(
                [pixmap.width() as usize, pixmap.height() as usize],
                pixmap.data(),
            );
            self.preview_big = Some((
                ui.ctx()
                    .load_texture("folio-preview-big", image, TextureOptions::LINEAR),
                self.project.clone(),
                index,
            ));
        }
        let (left, right) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
            )
        });
        let mut go = if left {
            index.checked_sub(1)
        } else if right && index + 1 < count {
            Some(index + 1)
        } else {
            None
        };
        let height = (ui.ctx().content_rect().height() - 160.0).max(300.0);
        let modal = egui::Modal::new(ui.scope_id().with("folio-preview")).show(ui.ctx(), |ui| {
            ui.label(egui::RichText::new("Take it for a swipe.").font(font::bold(18.0)));
            if let Some((texture, ..)) = &self.preview_big {
                let size = texture.size_vec2();
                ui.add(egui::Image::new(texture).fit_to_exact_size(size * (height / size.y)));
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(index > 0, egui::Button::new("←")).clicked() {
                    go = Some(index - 1);
                }
                ui.label(format!("{} / {}", index + 1, count));
                if ui
                    .add_enabled(index + 1 < count, egui::Button::new("→"))
                    .clicked()
                {
                    go = Some(index + 1);
                }
            });
        });
        if let Some(next) = go {
            self.preview_index = Some(next);
        } else if modal.should_close() {
            self.preview_index = None;
        }
    }

    /// Electron's reference library: the original Figma layouts for a size and theme; picking one applies its layout
    /// to the slide and its size and theme to the project. The panoramic navy original starts the navy carousel.
    fn references_window(&mut self, ui: &mut Ui) {
        if !self.show_references {
            return;
        }
        let Some(folder) = references_folder() else {
            return;
        };
        let (format, theme) = self.reference_look.clone();
        let mut names: Vec<String> = REFERENCE_NAMES
            .iter()
            .map(|name| {
                let stem = format!("Size={format}, Theme={theme}, Sections={name}");
                if folder.join(format!("{stem}.svg")).is_file() {
                    format!("{stem}.svg")
                } else {
                    format!("{stem}.png")
                }
            })
            .collect();
        names.push("Untitled.png".into());
        // One new picture a frame.
        if let Some(name) = names
            .iter()
            .find(|n| !self.reference_pictures.contains_key(*n))
        {
            let texture = reference_picture(&folder.join(name)).map(|image| {
                ui.ctx()
                    .load_texture(format!("folio-ref-{name}"), image, TextureOptions::LINEAR)
            });
            self.reference_pictures.insert(name.clone(), texture);
            ui.ctx().request_repaint();
        }
        let mut picked = None;
        let modal = egui::Modal::new(ui.scope_id().with("folio-references")).show(ui.ctx(), |ui| {
            ui.set_width(640.0);
            ui.label(egui::RichText::new("Your Figma references").font(font::bold(18.0)));
            ui.label(
                egui::RichText::new("Choose a reference to apply its layout, format and theme.")
                    .color(color::INK_MUTED),
            );
            ui.horizontal(|ui| {
                for look in ["Portrait", "Square"] {
                    ui.selectable_value(&mut self.reference_look.0, look.into(), look);
                }
                ui.separator();
                for look in ["Light", "Dark"] {
                    ui.selectable_value(&mut self.reference_look.1, look.into(), look);
                }
            });
            egui::ScrollArea::vertical()
                .id_salt("folio-references-grid")
                .max_height(480.0)
                .show(ui, |ui| {
                    egui::Grid::new("folio-reference-grid")
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            for (i, name) in names.iter().enumerate() {
                                if i > 0 && i % 4 == 0 {
                                    ui.end_row();
                                }
                                ui.vertical(|ui| {
                                    let label = if i < REFERENCE_NAMES.len() {
                                        REFERENCE_NAMES[i]
                                    } else {
                                        "Navy ribbon carousel"
                                    };
                                    let clicked = match self.reference_pictures.get(name) {
                                        Some(Some(texture)) => ui
                                            .add(egui::Button::image(
                                                egui::Image::new(texture)
                                                    .fit_to_exact_size(vec2(130.0, 162.0)),
                                            ))
                                            .clicked(),
                                        Some(None) => ui
                                            .add_sized(
                                                [130.0, 162.0],
                                                egui::Button::new("(missing)"),
                                            )
                                            .clicked(),
                                        None => ui
                                            .add_sized([130.0, 162.0], egui::Button::new("…"))
                                            .clicked(),
                                    };
                                    ui.label(egui::RichText::new(label).font(font::ui(12.0)));
                                    if clicked {
                                        picked = Some(i);
                                    }
                                });
                            }
                        });
                });
        });
        if let Some(i) = picked {
            if i < REFERENCE_NAMES.len() {
                self.slide().layout = LAYOUTS[i].0.into();
                self.project.format = format.to_lowercase();
                self.project.theme = theme.to_lowercase();
                self.notice = Some(format!("{} layout applied.", LAYOUTS[i].1));
            } else {
                match self.open(samples::navy(&mut new_id)) {
                    Ok(()) => self.edited_at = Some(Instant::now()),
                    Err(error) => self.notice = Some(error),
                }
            }
            self.show_references = false;
        } else if modal.should_close() {
            self.show_references = false;
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
    fn thumbnail(
        ctx: &egui::Context,
        name: String,
        project: &Project,
        index: usize,
        pictures: &mut Pictures,
        width: u32,
    ) -> TextureHandle {
        let pixmap = render::slide(project, index, pictures).0;
        let full =
            image::RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
                .unwrap();
        let small =
            image::imageops::thumbnail(&full, width, width * pixmap.height() / pixmap.width());
        let image = egui::ColorImage::from_rgba_premultiplied(
            [small.width() as usize, small.height() as usize],
            small.as_raw(),
        );
        ctx.load_texture(name, image, TextureOptions::LINEAR)
    }

    /// Electron's layout grid: each layout drawn as a sample square slide in this project's brand; picking one sets
    /// the current slide's layout.
    fn layouts_window(&mut self, ui: &mut Ui) {
        if !self.show_layouts {
            return;
        }
        let brand = (
            self.project.accent.clone(),
            self.project.theme.clone(),
            self.project.font.clone(),
        );
        if self
            .layout_thumbs
            .as_ref()
            .is_none_or(|(drawn, _)| *drawn != brand)
        {
            let thumbs = LAYOUTS
                .iter()
                .map(|(id, name, ..)| {
                    let navy = *id == "navy-ribbon";
                    let mut slide = Slide::new(id, String::new());
                    slide.eyebrow = if navy {
                        String::new()
                    } else {
                        "YOUR STORY".into()
                    };
                    slide.title = match *id {
                        "navy-ribbon" => "What Is A\nQR Code?",
                        "quote" => "A thought worth sharing.",
                        "thank-you" => "Thank you.",
                        "introduction" => "Your next\nbig idea.",
                        _ => "Title space",
                    }
                    .into();
                    slide.body = if navy {
                        String::new()
                    } else {
                        "A little room for your ideas.".into()
                    };
                    (slide.title_size, slide.swipe) = (104.0, !navy);
                    slide.items = vec![
                        "First idea".into(),
                        "Second idea".into(),
                        "Third idea".into(),
                    ];
                    let sample = Project {
                        accent: if navy {
                            "#f3ddc1".into()
                        } else {
                            self.project.accent.clone()
                        },
                        format: "square".into(),
                        brand: "YOUR STUDIO".into(),
                        handle: "@yourstudio".into(),
                        slides: vec![slide],
                        ..self.project.clone()
                    };
                    Self::thumbnail(
                        ui.ctx(),
                        format!("folio-layout-{name}"),
                        &sample,
                        0,
                        &mut self.pictures,
                        180,
                    )
                })
                .collect();
            self.layout_thumbs = Some((brand, thumbs));
        }
        let current = self.slide().layout.clone();
        let mut picked = None;
        let modal = egui::Modal::new(ui.scope_id().with("folio-layouts")).show(ui.ctx(), |ui| {
            ui.set_width(620.0);
            ui.label(egui::RichText::new("Pick a layout").font(font::bold(18.0)));
            let thumbs = &self.layout_thumbs.as_ref().unwrap().1;
            egui::Grid::new("folio-layout-grid")
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    for (i, ((id, name, _, description), texture)) in
                        LAYOUTS.iter().zip(thumbs).enumerate()
                    {
                        ui.vertical(|ui| {
                            let image =
                                egui::Image::new(texture).fit_to_exact_size(vec2(110.0, 110.0));
                            let response = ui
                                .add(egui::Button::image(image).selected(*id == current))
                                .on_hover_text(*description);
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
        if let Some(doc) = docs.iter().find(|d| {
            self.covers
                .get(&d.id)
                .is_none_or(|(at, _)| *at != d.saved_at)
        }) {
            if let Ok(project) = storage::load_document(&self.place, &doc.id) {
                let texture = Self::thumbnail(
                    ui.ctx(),
                    format!("folio-cover-{}", doc.id),
                    &project,
                    0,
                    &mut self.pictures,
                    160,
                );
                self.covers.insert(doc.id.clone(), (doc.saved_at, texture));
            }
            ui.ctx().request_repaint();
        }
        let mut opened = None;
        let modal = egui::Modal::new(ui.scope_id().with("folio-projects")).show(ui.ctx(), |ui| {
            ui.set_width(640.0);
            ui.label(egui::RichText::new("Open a project").font(font::bold(18.0)));
            ui.label(
                egui::RichText::new("Choose a carousel by its cover to continue editing.")
                    .color(color::INK_MUTED),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search projects")
                    .desired_width(f32::INFINITY),
            );
            let query = self.search.to_lowercase();
            egui::ScrollArea::vertical()
                .id_salt("folio-gallery")
                .max_height(460.0)
                .show(ui, |ui| {
                    // Four covers a row (a wrapping layout doesn't wrap columns of widgets).
                    egui::Grid::new("folio-gallery-grid")
                        .spacing([10.0, 12.0])
                        .show(ui, |ui| {
                            for (i, doc) in docs
                                .iter()
                                .filter(|d| d.name.to_lowercase().contains(&query))
                                .enumerate()
                            {
                                if i > 0 && i % 4 == 0 {
                                    ui.end_row();
                                }
                                ui.vertical(|ui| {
                                    ui.set_width(140.0);
                                    let clicked = match self.covers.get(&doc.id) {
                                        Some((_, texture)) => ui
                                            .add(egui::Button::image(
                                                egui::Image::new(texture)
                                                    .fit_to_exact_size(vec2(130.0, 162.0)),
                                            ))
                                            .clicked(),
                                        None => ui
                                            .add_sized([130.0, 162.0], egui::Button::new("…"))
                                            .clicked(),
                                    };
                                    let name: String = doc.name.chars().take(34).collect();
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(name).font(font::ui(12.0)),
                                        )
                                        .wrap(),
                                    );
                                    ui.label(
                                        egui::RichText::new(&doc.detail)
                                            .font(font::ui(11.0))
                                            .color(color::INK_MUTED),
                                    );
                                    if clicked {
                                        opened = Some(doc.id.clone());
                                    }
                                });
                            }
                        });
                });
        });
        if let Some(id) = opened {
            if let Err(error) =
                storage::load_document(&self.place, &id).and_then(|project| self.open(project))
            {
                self.notice = Some(error);
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
        if host.focused() && !ui.ctx().egui_wants_keyboard_input() && self.preview_index.is_none() {
            let (undo, redo, left, right) = ui.input(|i| {
                (
                    i.modifiers.command && i.key_pressed(egui::Key::Z) && !i.modifiers.shift,
                    i.modifiers.command
                        && (i.key_pressed(egui::Key::Y)
                            || (i.modifiers.shift && i.key_pressed(egui::Key::Z))),
                    i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown),
                )
            });
            if undo {
                if let Some(previous) = self.undo.pop() {
                    self.redo
                        .push(std::mem::replace(&mut self.project, previous));
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
        // egui delivers clipboard images as a paste event; text fields retain normal text paste.
        let pasted = if host.focused()
            && !ui.ctx().egui_wants_keyboard_input()
            && self.preview_index.is_none()
        {
            let (image, text_paste) = ui.input(|i| {
                (
                    i.events.iter().find_map(|event| match event {
                        egui::Event::PasteImage(image) => Some(image.clone()),
                        _ => None,
                    }),
                    i.events
                        .iter()
                        .any(|event| matches!(event, egui::Event::Paste(_))),
                )
            });
            // Some clipboard sources provide text and an image; egui sends only text in that case.
            image.or_else(|| {
                text_paste
                    .then(|| {
                        let mut clipboard = arboard::Clipboard::new().ok()?;
                        let image = clipboard.get_image().ok()?;
                        Some(std::sync::Arc::new(
                            egui::ColorImage::from_rgba_unmultiplied(
                                [image.width, image.height],
                                &image.bytes,
                            ),
                        ))
                    })
                    .flatten()
            })
        } else {
            None
        };
        if let Some(image) = pasted {
            let result = (|| -> Result<(), String> {
                let (w, h) = (
                    u32::try_from(image.size[0]).map_err(|e| e.to_string())?,
                    u32::try_from(image.size[1]).map_err(|e| e.to_string())?,
                );
                let pixels: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_srgba_unmultiplied())
                    .collect();
                let rgba =
                    image::RgbaImage::from_raw(w, h, pixels).ok_or("Invalid clipboard image")?;
                let mut png = std::io::Cursor::new(Vec::new());
                image::DynamicImage::ImageRgba8(rgba)
                    .write_to(&mut png, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
                let bytes = png.into_inner();
                if bytes.len() > 15 * 1024 * 1024 {
                    return Err("Use photos under 15 MB".into());
                }
                self.add_image_data(
                    format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    ),
                    w,
                    h,
                    None,
                )
            })();
            if let Err(error) = result {
                self.notice = Some(error);
            }
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            if let Err(error) = self.save() {
                host.toast(error);
            }
        }
        let rect = ui.max_rect();
        let top = Rect::from_min_size(rect.min, vec2(rect.width(), 40.0));
        ui.scope_builder(
            UiBuilder::new().max_rect(top.shrink2(vec2(6.0, 4.0))),
            |ui| self.toolbar(ui, host),
        );
        let body = Rect::from_min_max(pos2(rect.left(), top.bottom() + 4.0), rect.max);
        let left = Rect::from_min_size(body.min, vec2(200.0, body.height()));
        let right = Rect::from_min_max(pos2(body.right() - 280.0, body.top()), body.max);
        let middle = Rect::from_min_max(
            pos2(left.right() + 6.0, body.top()),
            pos2(right.left() - 6.0, body.bottom()),
        );
        for panel in [left, right] {
            ui.painter()
                .rect_filled(panel, 8.0, Color32::from_rgb(31, 37, 48));
        }
        ui.scope_builder(UiBuilder::new().max_rect(left.shrink(8.0)), |ui| {
            self.slides(ui)
        });
        ui.scope_builder(UiBuilder::new().max_rect(right.shrink(8.0)), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("folio-details")
                .show(ui, |ui| self.details(ui));
        });
        ui.scope_builder(UiBuilder::new().max_rect(middle), |ui| self.canvas(ui));
        self.projects_window(ui);
        self.layouts_window(ui);
        self.preview_window(ui);
        self.references_window(ui);
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
        self.receive_files(files, None, host)
    }

    fn receive_at(
        &mut self,
        files: Vec<SharedFile>,
        position: egui::Pos2,
        host: &mut dyn Host,
    ) -> Receipt {
        let at = self
            .slide_rect
            .filter(|rect| rect.contains(position))
            .map(|rect| {
                (
                    (position.x - rect.left()) * 1080.0 / rect.width(),
                    (position.y - rect.top()) * self.project.height() / rect.height(),
                )
            });
        self.receive_files(files, at, host)
    }

    fn open_document(&mut self, id: &str, host: &mut dyn Host) {
        if let Err(error) =
            storage::load_document(&self.place, id).and_then(|project| self.open(project))
        {
            host.toast(error);
        }
    }

    fn save(&mut self) -> Result<(), String> {
        // Write the recovery first so a project-folder failure still leaves the current work available.
        self.edited_at = None;
        storage::save_recovery(&self.place, &self.project)?;
        storage::save(&self.place, &mut self.project, new_id)?;
        Ok(())
    }

    fn title(&self) -> String {
        self.project.name.clone()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn failed_save_does_not_switch_projects_or_overwrite_recovery() {
        use canopy_ui::{Place, SpaceInfo};
        let root = std::env::temp_dir().join(format!("folio-switch-{}", super::new_id()));
        std::fs::create_dir_all(&root).unwrap();
        let projects = root.join("not-a-directory");
        std::fs::write(&projects, b"file").unwrap();
        let place = Place {
            space: Some(SpaceInfo {
                id: "test".into(),
                name: "test".into(),
                dir: root.clone(),
            }),
            projects_dir: projects,
            data_dir: root.join("data"),
            exports_dir: root.clone(),
        };
        let mut app = super::Folio::new(place);
        app.project.name = "Current work".into();
        let mut other = app.project.clone();
        other.name = "Other".into();
        assert!(app.open(other).is_err());
        assert_eq!(app.project.name, "Current work");
        assert_eq!(
            super::storage::load_recovery(&app.place, 0).unwrap().name,
            "Current work"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn image_paste_event_adds_a_layer() {
        use canopy_ui::{CanopyApp, Host, Place, SpaceInfo};
        struct TestHost;
        impl Host for TestHost {
            fn toast(&mut self, _: String) {}
            fn exported(&mut self, _: std::path::PathBuf) {}
        }
        let root = std::env::temp_dir().join(format!("folio-paste-{}", super::new_id()));
        let place = Place {
            space: Some(SpaceInfo {
                id: "test".into(),
                name: "test".into(),
                dir: root.clone(),
            }),
            projects_dir: root.clone(),
            data_dir: root.clone(),
            exports_dir: root.clone(),
        };
        let mut app = super::Folio::new(place);
        let ctx = egui::Context::default();
        canopy_ui::install(&ctx);
        let image = egui::ColorImage::new(
            [2, 2],
            vec![egui::Color32::from_rgba_unmultiplied(200, 50, 20, 128); 4],
        );
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            events: vec![egui::Event::PasteImage(std::sync::Arc::new(image))],
            focused: true,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.ui(ui, &mut TestHost));
        output.textures_delta.clear();
        let layer = app.project.slides[0]
            .layers
            .last()
            .expect("image paste adds a layer");
        assert_eq!(layer.kind, "image");
        assert!(
            layer
                .src
                .as_deref()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        assert!(app.edited_at.is_some());
    }

    #[test]
    fn received_image_uses_slide_coordinates_and_can_be_saved() {
        use canopy_ui::{CanopyApp, Host, Place, SharedFile, SpaceInfo};
        struct TestHost;
        impl Host for TestHost {
            fn toast(&mut self, _: String) {}
            fn exported(&mut self, _: std::path::PathBuf) {}
        }
        let root = std::env::temp_dir().join(format!("folio-drop-{}", super::new_id()));
        std::fs::create_dir_all(&root).unwrap();
        let place = Place {
            space: Some(SpaceInfo {
                id: "test".into(),
                name: "test".into(),
                dir: root.clone(),
            }),
            projects_dir: root.clone(),
            data_dir: root.clone(),
            exports_dir: root.clone(),
        };
        let file = root.join("image.png");
        image::RgbaImage::from_pixel(100, 100, image::Rgba([255, 0, 0, 255]))
            .save(&file)
            .unwrap();
        let mut app = super::Folio::new(place);
        app.slide_rect = Some(egui::Rect::from_min_size(
            egui::pos2(100.0, 50.0),
            egui::vec2(540.0, 675.0),
        ));
        let receipt = app.receive_at(
            vec![SharedFile::new(&file)],
            egui::pos2(250.0, 250.0),
            &mut TestHost,
        );
        assert_eq!(receipt.accepted, 1);
        let layer = app.project.slides[0].layers.last().unwrap();
        assert_eq!((layer.x, layer.y), (300.0, 400.0));
        assert_eq!(app.undo.len(), 1);
        assert!(app.edited_at.is_some());
        app.save().unwrap();
        assert!(super::storage::list(&app.place).len() > 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ids_name_files_apart() {
        let ids: Vec<String> = (0..1000).map(|_| super::new_id()).collect();
        let prefixes: std::collections::HashSet<&str> = ids.iter().map(|id| &id[..12]).collect();
        assert_eq!(prefixes.len(), ids.len());
    }
}
