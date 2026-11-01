//! Folio's project folder, as Electron Folio keeps it (`app-host.cjs`): `<name>--<id>.folio.json` files, pretty
//! JSON, written atomically, each knowing its own file name (`folderFileName`). Inside a Canopy space the folder
//! is the space's `Folio` folder; on its own it's `Documents\Folio Projects`.

use crate::model::Project;
use canopy_ui::{Document, Place};
use serde_json::Value;
use std::path::Path;
use std::time::UNIX_EPOCH;

const MAX_PROJECT: u64 = 150 * 1024 * 1024;

/// A name made safe for a file, as Electron's `safeName` does.
pub fn safe_name(name: &str) -> String {
    let safe: String = name.chars().map(|c| if "<>:\"/\\|?*".contains(c) || (c as u32) < 0x20 { '-' } else { c }).take(100).collect();
    if safe.is_empty() { "carousel".into() } else { safe }
}

fn valid_file_name(name: &str) -> bool {
    name.ends_with(".folio.json") && name != ".folio.json" && !name.chars().any(|c| "<>:\"/\\|?*".contains(c) || (c as u32) < 0x20)
}

pub fn list(place: &Place) -> Vec<Document> {
    let Ok(entries) = std::fs::read_dir(&place.projects_dir) else { return Vec::new() };
    let mut documents: Vec<Document> = entries.flatten().filter_map(|entry| {
        let name = entry.file_name().to_str()?.to_owned();
        let meta = entry.metadata().ok()?;
        if !meta.is_file() || !valid_file_name(&name) || meta.len() > MAX_PROJECT {
            return None;
        }
        let value: Value = serde_json::from_slice(&std::fs::read(entry.path()).ok()?).ok()?;
        let slides = value["slides"].as_array().map_or(0, Vec::len);
        if value["version"] != 1 || !(1..=20).contains(&slides) {
            return None;
        }
        let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
        let saved = value["savedAt"].as_f64().unwrap_or(0.0) as u64;
        Some(Document {
            id: name.clone(),
            name: value["name"].as_str().unwrap_or(&name).to_owned(),
            saved_at: saved.max(modified),
            detail: format!("{slides} slide{}", if slides == 1 { "" } else { "s" }),
        })
    }).collect();
    documents.sort_by(|a, b| a.name.cmp(&b.name));
    documents
}

pub fn load(path: &Path) -> Result<Project, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_PROJECT {
        return Err("Project exceeds 150 MB.".into());
    }
    Project::read(&std::fs::read(path).map_err(|e| e.to_string())?)
}

pub fn load_document(place: &Place, file_name: &str) -> Result<Project, String> {
    if !valid_file_name(file_name) {
        return Err("Choose a valid project from the folder.".into());
    }
    let mut project = load(&place.projects_dir.join(file_name))?;
    project.extra.insert("folderFileName".into(), file_name.into());
    Ok(project)
}

/// A separate recovery copy, compatible with Electron's last-session.json.
pub fn save_recovery(place: &Place, project: &Project) -> Result<(), String> {
    std::fs::create_dir_all(&place.data_dir).map_err(|e| e.to_string())?;
    let mut snapshot = project.clone();
    let now = std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    snapshot.extra.insert("savedAt".into(), now.into());
    let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_PROJECT { return Err("Project exceeds 150 MB.".into()); }
    let target = place.data_dir.join("last-session.json");
    let temporary = place.data_dir.join(format!("last-session.{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, &target).map_err(|e| { let _ = std::fs::remove_file(&temporary); e.to_string() })
}

/// A recovery copy is only used when it is newer than the last project file.
pub fn load_recovery(place: &Place, latest_saved: u64) -> Option<Project> {
    let project = load(&place.data_dir.join("last-session.json")).ok()?;
    let saved = project.extra.get("savedAt").and_then(Value::as_u64).unwrap_or(0);
    (saved > latest_saved).then_some(project)
}

/// Saves the project in its file (a new one, named after it, the first time) and returns the file name.
pub fn save(place: &Place, project: &mut Project, id: impl FnOnce() -> String) -> Result<String, String> {
    let folder = &place.projects_dir;
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let file_name = match project.extra.get("folderFileName").and_then(Value::as_str).filter(|n| valid_file_name(n)) {
        Some(name) => name.to_owned(),
        None => {
            // Electron names a library project after its libraryId, anything else after a fresh id.
            let library = project.extra.get("libraryId").and_then(Value::as_str).unwrap_or("").to_owned();
            let source = if library.is_empty() { id() } else { library };
            let suffix: String = source.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(12).collect();
            let stem: String = safe_name(&project.name).chars().take(70).collect();
            let mut name = format!("{stem}--{suffix}.folio.json");
            let mut number = 2;
            while folder.join(&name).exists() {
                name = format!("{stem}--{suffix}-{number}.folio.json");
                number += 1;
            }
            name
        }
    };
    project.extra.insert("folderFileName".into(), file_name.clone().into());
    let json = serde_json::to_string_pretty(project).map_err(|e| e.to_string())?;
    let target = folder.join(&file_name);
    let temporary = target.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, json).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, &target).map_err(|e| {
        let _ = std::fs::remove_file(&temporary);
        e.to_string()
    })?;
    Ok(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_survives_an_unwritable_project_folder_and_respects_age() {
        let root = std::env::temp_dir().join(format!("folio-recovery-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let projects = root.join("not-a-directory");
        std::fs::write(&projects, b"file").unwrap();
        let place = Place { space: None, projects_dir: projects, data_dir: root.join("data"), exports_dir: root.join("Files") };
        let mut project = Project::starter(&mut || "id".into());
        project.name = "Recover me".into();
        save_recovery(&place, &project).unwrap();
        assert!(save(&place, &mut project, || "id".into()).is_err());
        assert_eq!(load_recovery(&place, 0).unwrap().name, "Recover me");
        assert!(load_recovery(&place, u64::MAX).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projects_are_saved_and_listed_like_electron_folio() {
        let root = std::env::temp_dir().join(format!("folio-native-storage-{}", std::process::id()));
        let place = Place { space: None, projects_dir: root.join("Folio"), data_dir: root.join("data"), exports_dir: root.join("Files") };
        let mut project = Project::starter(&mut || "id".into());
        project.name = "A: b?".into();
        let name = save(&place, &mut project, || "1234567890abcdef".into()).unwrap();
        assert_eq!(name, "A- b---1234567890ab.folio.json");
        assert_eq!(save(&place, &mut project, || unreachable!()).unwrap(), name, "the same file next time");
        let listed = list(&place);
        assert_eq!((listed.len(), listed[0].id.as_str(), listed[0].name.as_str(), listed[0].detail.as_str()), (1, name.as_str(), "A: b?", "5 slides"));
        let opened = load_document(&place, &name).unwrap();
        assert_eq!(opened.slides.len(), 5);
        std::fs::remove_dir_all(root).unwrap();
    }
}
