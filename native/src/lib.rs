//! Folio's native carousel editor. Projects are the same `.folio.json` files Electron Folio saves.

pub mod app;
pub mod designs;
pub mod model;
pub mod render;
pub mod samples;
pub mod storage;
pub mod text;

use canopy_ui::{Accent, AppDescriptor, CanopyApp, Place, SharedFile};

fn create(place: Place, _ctx: &egui::Context) -> Box<dyn CanopyApp> {
    Box::new(app::Folio::new(place))
}

pub static DESCRIPTOR: AppDescriptor = AppDescriptor {
    id: "folio",
    name: "Folio",
    description: "Make carousels",
    accent: Accent::Sky,
    initial: "Fl",
    folder: "Folio",
    icon_png: &[],
    accepts: |file: &SharedFile| {
        let name = file.name.to_lowercase();
        [".folio.json", ".png", ".jpg", ".jpeg", ".webp"]
            .iter()
            .any(|ext| name.ends_with(ext))
    },
    documents: storage::list,
    create,
};
