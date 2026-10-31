// The editor's main-process side: recovery, the project folder, saving, and export. The standalone app (main.cjs) and
// Canopy, which shows Folio in one of its panes, both create it. Inside Canopy the project folder is the space's
// Folio folder and exports land in the space's Files instead of a save dialog.
const { dialog } = require('electron');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const JSZip = require('jszip');
const funTechProjects = require('./assets/projects/fun-tech/manifest.json');
const funTechIds = new Set(funTechProjects.map(project => project.slug));
const IMAGE_TYPES = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', webp: 'image/webp' };
const MAX_IMAGE = 15 * 1024 * 1024;
const MAX_PROJECT = 150 * 1024 * 1024;
const channels = ['load-recovery', 'save-project', 'open-project', 'list-fun-tech-projects', 'open-fun-tech-project', 'get-project-folder', 'choose-project-folder', 'list-folder-projects', 'open-folder-project', 'save-folder-project', 'export-slides'];

function safeName(name) { return String(name || 'carousel').replace(/[<>:"/\\|?*\x00-\x1f]/g, '-').slice(0, 100) || 'carousel'; }
function validProjectFileName(name) {
  return typeof name === 'string' && name.endsWith('.folio.json') && path.basename(name) === name && !/[<>:"/\\|?*\x00-\x1f]/.test(name) && name !== '.folio.json';
}
async function folderProjects(folder) {
  const files = await fs.readdir(folder, { withFileTypes: true });
  const projects = [];
  for (const entry of files) {
    if (!entry.isFile() || !validProjectFileName(entry.name)) continue;
    try {
      const file = path.join(folder, entry.name);
      const stat = await fs.stat(file);
      if (stat.size > MAX_PROJECT) continue;
      const value = JSON.parse(await fs.readFile(file, 'utf8'));
      if (value.version !== 1 || !Array.isArray(value.slides) || !value.slides.length || value.slides.length > 20) continue;
      projects.push({ fileName: entry.name, libraryId: typeof value.libraryId === 'string' ? value.libraryId : '', name: typeof value.name === 'string' ? value.name : entry.name, slides: value.slides.length, savedAt: Number(value.savedAt) || 0, modifiedAt: stat.mtimeMs });
    } catch (_) { /* Leave unrelated or invalid JSON files untouched. */ }
  }
  return projects.sort((a, b) => a.name.localeCompare(b.name));
}
// For Canopy's sidebar, which lists a space's carousels before Folio is opened.
async function documents({ canopy }) {
  let projects;
  try { projects = await folderProjects(canopy.projectsDir); } catch (error) { if (error.code === 'ENOENT') return []; throw error; }
  return projects.map(item => ({ id: item.fileName, name: item.name, savedAt: Math.max(item.savedAt, item.modifiedAt), detail: `${item.slides} slide${item.slides === 1 ? '' : 's'}` }));
}

// ipc: ipcMain or Canopy's per-app router. window/webContents: getters for the editor. documentsDir: where the folder
// picker starts. canopy: present only inside Canopy ({space, projectsDir, filesDir, saveFile}).
function create({ ipc, dataDir, documentsDir, window, webContents, canopy = null }) {
  const recoveryPath = () => path.join(dataDir, 'last-session.json');
  const folderConfigPath = () => path.join(dataDir, 'project-folder.json');
  const editor = () => { const contents = webContents(); return contents && !contents.isDestroyed() ? contents : null; };
  async function projectFolder() {
    if (canopy) { await fs.mkdir(canopy.projectsDir, { recursive: true }); return canopy.projectsDir; }
    try {
      const config = JSON.parse(await fs.readFile(folderConfigPath(), 'utf8'));
      if (typeof config.path !== 'string' || !path.isAbsolute(config.path)) return null;
      return config.path;
    } catch (_) { return null; }
  }
  async function writeRecovery(snapshot) {
    snapshot.savedAt = Date.now();
    await fs.mkdir(dataDir, { recursive: true });
    const temporary = recoveryPath() + '.' + process.pid + '.tmp';
    await fs.writeFile(temporary, JSON.stringify(snapshot), 'utf8');
    await fs.rename(temporary, recoveryPath());
  }
  // Saves the open carousel as the recovery copy (on close, and before Canopy switches spaces).
  async function checkpoint() {
    const contents = editor();
    if (!contents) return;
    const snapshot = await contents.executeJavaScript('window.FolioSession ? window.FolioSession.snapshot() : null');
    if (snapshot) await writeRecovery(snapshot);
  }
  async function saveExport(fileName, contents) {
    if (canopy) { await canopy.saveFile({ name: fileName, data: await contents() }); return true; }
    const extension = path.extname(fileName).slice(1);
    const result = await dialog.showSaveDialog(window(), { defaultPath: fileName, filters: [{ name: extension === 'png' ? 'PNG image' : 'ZIP archive', extensions: [extension] }] });
    if (result.canceled) return false;
    await fs.writeFile(result.filePath, await contents());
    return true;
  }

  ipc.handle('load-recovery', async () => {
    try { return JSON.parse(await fs.readFile(recoveryPath(), 'utf8')); }
    catch (_) { return null; }
  });
  ipc.handle('save-project', async (_, project) => {
    const fileName = safeName(project.name) + '.folio.json';
    const result = await dialog.showSaveDialog(window(), { defaultPath: canopy ? path.join(canopy.filesDir, fileName) : fileName, filters: [{ name: 'Folio project', extensions: ['json'] }] });
    if (result.canceled) return false;
    await fs.writeFile(result.filePath, JSON.stringify(project, null, 2), 'utf8');
    return true;
  });
  ipc.handle('open-project', async () => {
    const result = await dialog.showOpenDialog(window(), { ...(canopy && { defaultPath: canopy.filesDir }), properties: ['openFile'], filters: [{ name: 'Folio project', extensions: ['json'] }] });
    if (result.canceled) return null;
    const stat = await fs.stat(result.filePaths[0]);
    if (stat.size > MAX_PROJECT) throw new Error('Project exceeds the 150 MB limit.');
    return JSON.parse(await fs.readFile(result.filePaths[0], 'utf8'));
  });
  ipc.handle('list-fun-tech-projects', () => funTechProjects.map(({ slug, name, slides }) => ({ slug, name, slides })));
  ipc.handle('open-fun-tech-project', async (_, slug) => {
    if (typeof slug !== 'string' || !funTechIds.has(slug)) throw new Error('This collection project is unavailable.');
    const file = path.join(__dirname, 'assets', 'projects', 'fun-tech', slug + '.folio.json');
    return JSON.parse(await fs.readFile(file, 'utf8'));
  });
  ipc.handle('get-project-folder', projectFolder);
  ipc.handle('choose-project-folder', async () => {
    if (canopy) return projectFolder();
    const current = await projectFolder();
    const suggested = path.join(documentsDir, 'Folio Projects');
    const availableCurrent = current && await fs.stat(current).then(stat => stat.isDirectory(), () => false);
    const defaultPath = availableCurrent ? current : await fs.stat(suggested).then(stat => stat.isDirectory() ? suggested : documentsDir).catch(() => documentsDir);
    const result = await dialog.showOpenDialog(window(), { title: 'Choose your Folio projects folder', defaultPath, properties: ['openDirectory', 'createDirectory'] });
    if (result.canceled || !result.filePaths?.[0]) return null;
    const selected = path.resolve(result.filePaths[0]);
    if (!(await fs.stat(selected)).isDirectory()) throw new Error('Choose a folder for your projects.');
    await fs.mkdir(dataDir, { recursive: true });
    const temporary = folderConfigPath() + '.' + process.pid + '.tmp';
    await fs.writeFile(temporary, JSON.stringify({ path: selected }), 'utf8');
    await fs.rename(temporary, folderConfigPath());
    return selected;
  });
  ipc.handle('list-folder-projects', async () => {
    const folder = await projectFolder();
    if (!folder) return [];
    return folderProjects(folder);
  });
  ipc.handle('open-folder-project', async (_, fileName) => {
    const folder = await projectFolder();
    if (!folder || !validProjectFileName(fileName)) throw new Error('Choose a valid project from the folder.');
    const file = path.join(folder, fileName);
    if (!(await fs.lstat(file)).isFile()) throw new Error('This project file is unavailable.');
    if ((await fs.stat(file)).size > MAX_PROJECT) throw new Error('Project exceeds 150 MB.');
    const value = JSON.parse(await fs.readFile(file, 'utf8'));
    value.folderFileName = fileName;
    return value;
  });
  ipc.handle('save-folder-project', async (_, value) => {
    const folder = await projectFolder();
    if (!folder) throw new Error('Choose a project folder first.');
    if (!value || value.version !== 1 || !Array.isArray(value.slides) || value.slides.length < 1 || value.slides.length > 20 || typeof value.name !== 'string') throw new Error('Invalid Folio project.');
    if (JSON.stringify(value).length > MAX_PROJECT) throw new Error('Project exceeds 150 MB.');
    let fileName = validProjectFileName(value.folderFileName) ? value.folderFileName : null;
    if (!fileName && typeof value.libraryId === 'string' && value.libraryId) {
      const match = (await folderProjects(folder)).find(item => item.libraryId === value.libraryId);
      if (match) fileName = match.fileName;
    }
    if (!fileName) {
      const suffix = (typeof value.libraryId === 'string' ? value.libraryId.replace(/[^a-zA-Z0-9-]/g, '').slice(0, 12) : '') || crypto.randomUUID().slice(0, 12);
      const stem = safeName(value.name).slice(0, 70);
      fileName = `${stem}--${suffix}.folio.json`;
      for (let number = 2; await fs.access(path.join(folder, fileName)).then(() => true, () => false); number++) fileName = `${stem}--${suffix}-${number}.folio.json`;
    }
    const target = path.join(folder, fileName);
    try { if (!(await fs.lstat(target)).isFile()) throw new Error('Project target is not a regular file.'); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
    const temporary = target + '.' + process.pid + '.' + crypto.randomUUID() + '.tmp';
    await fs.writeFile(temporary, JSON.stringify({ ...value, folderFileName: fileName }, null, 2), 'utf8');
    try { await fs.rename(temporary, target); }
    catch (error) { await fs.unlink(temporary).catch(() => {}); throw error; }
    return { fileName };
  });
  ipc.handle('export-slides', async (_, { name, slides }) => {
    if (!Array.isArray(slides) || slides.length < 1 || slides.length > 20 || slides.some(s => !/^data:image\/png;base64,/.test(s))) throw new Error('Invalid export.');
    const buffers = slides.map(s => Buffer.from(s.split(',')[1], 'base64'));
    if (buffers.length === 1) return saveExport(safeName(name) + '.png', async () => buffers[0]);
    return saveExport(safeName(name) + '.zip', () => {
      const zip = new JSZip();
      buffers.forEach((buffer, i) => zip.file(`${String(i + 1).padStart(2, '0')}-${safeName(name)}.png`, buffer));
      return zip.generateAsync({ type: 'nodebuffer' });
    });
  });

  // Files handed over by Canopy: pictures become movable image layers on the current slide, Folio project files are
  // imported into the project folder.
  async function receive(files) {
    const images = [], skipped = [];
    let accepted = 0;
    for (const file of files) {
      const extension = path.extname(file.path).slice(1).toLowerCase();
      try {
        if (file.name.toLowerCase().endsWith('.folio.json')) {
          if ((await fs.stat(file.path)).size > MAX_PROJECT) throw new Error('Too large');
          editor()?.send('folio:canopy-project', JSON.parse(await fs.readFile(file.path, 'utf8')));
          accepted++;
        } else if (IMAGE_TYPES[extension] && (await fs.stat(file.path)).size <= MAX_IMAGE) {
          images.push({ name: file.name, type: IMAGE_TYPES[extension], bytes: await fs.readFile(file.path) });
          accepted++;
        } else skipped.push(file.name);
      } catch (_) { skipped.push(file.name); }
    }
    if (images.length) editor()?.send('folio:canopy-images', images);
    return { accepted, skipped };
  }
  async function listDocuments() {
    const folder = await projectFolder();
    return folder ? documents({ canopy: { projectsDir: folder } }) : [];
  }
  async function openDocument(fileName) {
    if (!validProjectFileName(fileName)) throw new Error('Choose a valid project from the folder.');
    editor()?.send('folio:canopy-open-project', fileName);
  }
  async function dispose() { for (const channel of channels) ipc.removeHandler(channel); }

  return {
    preload: path.join(__dirname, 'preload.cjs'),
    entry: path.join(__dirname, 'index.html'),
    additionalArguments: canopy ? ['--canopy=' + JSON.stringify({ space: canopy.space.name })] : [],
    checkpoint, receive, listDocuments, openDocument, dispose
  };
}

module.exports = { id: 'folio', name: 'Folio', accent: 'sky', schemes: [], documents, create };
