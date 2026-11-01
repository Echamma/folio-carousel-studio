//! Ready-made projects, as Electron Folio has them: the Navy ribbon carousel (`navyProject` in `app.js`) and the ten
//! Fun Tech Facts carousels (`assets/projects/fun-tech`), which a standalone Folio adds to its project folder the
//! first time it opens one (inside a Canopy space they stay out, as in Electron).

use crate::model::{Project, Slide};
use crate::storage;
use canopy_ui::Place;
use serde_json::Map;

const FUN_TECH: [(&str, &str); 10] = [
    ("01-bluetooth", include_str!("../../assets/projects/fun-tech/01-bluetooth.folio.json")),
    ("02-gps", include_str!("../../assets/projects/fun-tech/02-gps.folio.json")),
    ("03-dns", include_str!("../../assets/projects/fun-tech/03-dns.folio.json")),
    ("04-cookies", include_str!("../../assets/projects/fun-tech/04-cookies.folio.json")),
    ("05-browser-cache", include_str!("../../assets/projects/fun-tech/05-browser-cache.folio.json")),
    ("06-https", include_str!("../../assets/projects/fun-tech/06-https.folio.json")),
    ("07-cloud", include_str!("../../assets/projects/fun-tech/07-cloud.folio.json")),
    ("08-pixels", include_str!("../../assets/projects/fun-tech/08-pixels.folio.json")),
    ("09-nfc", include_str!("../../assets/projects/fun-tech/09-nfc.folio.json")),
    ("10-binary", include_str!("../../assets/projects/fun-tech/10-binary.folio.json")),
];

/// Electron Folio's "QR Code — Navy Ribbon": eight square slides on the navy ribbon layout.
pub fn navy(new_id: &mut impl FnMut() -> String) -> Project {
    let copy = [
        ("What Is A\nQR Code?", ""),
        ("CONNECT:", "It is a physical copy-paste button for your camera."),
        ("Free", "Anyone can generate a custom code for free."),
        ("Universal", "Originally invented in 1994 to track car parts, they now seamlessly connect you to Wi-Fi networks, digital menus, and contactless payments."),
        ("Sale:", "Share your early-bird offer for artists, small shops, and restaurants."),
        ("Sale:", "Add your launch promotion here. Give your audience a clear reason to get started."),
        ("Sale:", "Add your subscription offer, price, and terms here."),
        ("Fill out the\nform in our\nBio", ""),
    ];
    let slides = copy.iter().map(|(title, body)| Slide {
        eyebrow: String::new(),
        title: (*title).into(),
        body: (*body).into(),
        title_size: if body.is_empty() { 98.0 } else { 64.0 },
        swipe: false,
        ..Slide::new("navy-ribbon", new_id())
    }).collect();
    Project {
        version: 1, name: "QR Code — Navy Ribbon".into(), format: "square".into(), theme: "dark".into(), accent: "#f3ddc1".into(),
        brand: "YOUR STUDIO".into(), handle: "@yourstudio".into(), font: "Arial".into(), direction: None, slides, extra: Map::new(),
    }
}

/// Adds the Fun Tech carousels to a standalone Folio's project folder once (a marker in its data folder remembers
/// it, so deleted samples stay deleted). Returns how many were added.
pub fn seed(place: &Place) -> usize {
    let marker = place.data_dir.join("fun-tech-added");
    if place.space.is_some() || marker.exists() {
        return 0;
    }
    let existing: Vec<String> = storage::list(place).into_iter().filter_map(|doc| storage::load_document(place, &doc.id).ok())
        .filter_map(|p| p.extra.get("libraryId").and_then(|v| v.as_str()).map(str::to_owned)).collect();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    let mut added = 0;
    for (slug, json) in FUN_TECH {
        if existing.iter().any(|id| id == slug) {
            continue;
        }
        let Ok(mut project) = Project::read(json.as_bytes()) else { continue };
        project.extra.insert("libraryId".into(), slug.into());
        project.extra.insert("savedAt".into(), now.into());
        if storage::save(place, &mut project, || slug.to_owned()).is_ok() {
            added += 1;
        }
    }
    let _ = std::fs::create_dir_all(&place.data_dir).and_then(|_| std::fs::write(&marker, b"1"));
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_valid_and_added_once_outside_spaces() {
        let mut n = 0;
        assert!(navy(&mut || { n += 1; n.to_string() }).validate().is_ok());
        let root = std::env::temp_dir().join(format!("folio-native-samples-{}", std::process::id()));
        let place = Place { space: None, projects_dir: root.join("Folio Projects"), data_dir: root.join("data"), exports_dir: root.join("Exports") };
        assert_eq!(seed(&place), 10);
        assert_eq!(seed(&place), 0, "only the first time");
        let listed = storage::list(&place);
        assert_eq!(listed.len(), 10);
        assert!(listed.iter().any(|d| d.id == "Why Bluetooth Has a Viking Name--01-bluetooth.folio.json"), "{:?}", listed.iter().map(|d| &d.id).collect::<Vec<_>>());
        std::fs::remove_dir_all(root).unwrap();
    }
}
