const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const JSZip = require('jszip');
const funTechProjects = require('./assets/projects/fun-tech/manifest.json');
const funTechIds = new Set(funTechProjects.map(project => project.slug));
let window;
let closing = false;
let allowClose = false;
const recoveryPath = () => path.join(app.getPath('userData'), 'last-session.json');
const folderConfigPath = () => path.join(app.getPath('userData'), 'project-folder.json');
function safeName(name) { return String(name || 'carousel').replace(/[<>:"/\\|?*\x00-\x1f]/g, '-').slice(0, 100) || 'carousel'; }
function validProjectFileName(name) {
  return typeof name === 'string' && name.endsWith('.folio.json') && path.basename(name) === name && !/[<>:"/\\|?*\x00-\x1f]/.test(name) && name !== '.folio.json';
}
async function projectFolder() {
  try {
    const config = JSON.parse(await fs.readFile(folderConfigPath(), 'utf8'));
    if (typeof config.path !== 'string' || !path.isAbsolute(config.path)) return null;
    if (!(await fs.stat(config.path)).isDirectory()) return null;
    return config.path;
  } catch (_) { return null; }
}
async function folderProjects(folder) {
  const files = await fs.readdir(folder, { withFileTypes: true });
  const projects = [];
  for (const entry of files) {
    if (!entry.isFile() || !validProjectFileName(entry.name)) continue;
    try {
      const file = path.join(folder, entry.name);
      const stat = await fs.stat(file);
      if (stat.size > 150 * 1024 * 1024) continue;
      const value = JSON.parse(await fs.readFile(file, 'utf8'));
      if (value.version !== 1 || !Array.isArray(value.slides) || !value.slides.length || value.slides.length > 20) continue;
      projects.push({ fileName: entry.name, libraryId: typeof value.libraryId === 'string' ? value.libraryId : '', name: typeof value.name === 'string' ? value.name : entry.name, slides: value.slides.length, savedAt: Number(value.savedAt) || 0, modifiedAt: stat.mtimeMs });
    } catch (_) { /* Leave unrelated or invalid JSON files untouched. */ }
  }
  return projects.sort((a, b) => a.name.localeCompare(b.name));
}
app.whenReady().then(() => {
  window = new BrowserWindow({ width: 1500, height: 980, minWidth: 1100, minHeight: 760, show: process.env.FOLIO_TEST !== '1', backgroundColor: '#f6f5f1', title: 'Folio — Carousel Studio', autoHideMenuBar: true, webPreferences: { preload: path.join(__dirname, 'preload.cjs'), contextIsolation: true, nodeIntegration: false, sandbox: true } });
  window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  window.webContents.on('will-navigate', (event) => event.preventDefault());
  window.on('close', async event => {
    if (allowClose) return;
    event.preventDefault();
    if (closing) return;
    closing = true;
    try {
      let timer;
      const snapshot = await Promise.race([
        window.webContents.executeJavaScript('window.FolioSession ? window.FolioSession.snapshot() : null'),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('The editor did not respond.')), 5000); })
      ]).finally(() => clearTimeout(timer));
      if (snapshot) {
        snapshot.savedAt = Date.now();
        const temporary = recoveryPath() + '.' + process.pid + '.tmp';
        await fs.writeFile(temporary, JSON.stringify(snapshot), 'utf8');
        await fs.rename(temporary, recoveryPath());
      }
      allowClose = true;
      window.close();
    } catch (error) {
      const result = await dialog.showMessageBox(window, { type: 'warning', title: 'Save before closing', message: 'Your latest changes could not be saved automatically.', detail: 'Choose Cancel to return to the editor and use Save project.\n\n' + error.message, buttons: ['Cancel', 'Close without saving'], defaultId: 0, cancelId: 0 });
      if (result.response === 1) { allowClose = true; window.close(); }
    } finally { closing = false; }
  });
  window.loadFile(path.join(__dirname, 'index.html'));
});
app.on('window-all-closed', () => app.quit());
if (process.argv.includes('--smoke-test')) {
  process.env.FOLIO_TEST = '1';
  app.setPath('userData', path.join(app.getPath('temp'), 'folio-carousel-studio-smoke-' + process.pid));
  const timeout = setTimeout(() => { console.error('Packaged Folio startup timed out'); app.exit(1); }, 20000);
  app.whenReady().then(() => window.webContents.once('did-finish-load', async () => {
    try {
      const ok = await window.webContents.executeJavaScript('(() => { const snapshot = window.FolioSession && window.FolioSession.snapshot(); return snapshot && snapshot.slides.length > 0; })()');
      if (!ok) throw new Error('Packaged editor did not initialize');
      console.log('PASS: Packaged Folio desktop app starts and loads its editor');
      clearTimeout(timeout); app.exit(0);
    } catch (error) { console.error(error); clearTimeout(timeout); app.exit(1); }
  }));
}
ipcMain.handle('load-recovery', async () => {
  try { return JSON.parse(await fs.readFile(recoveryPath(), 'utf8')); }
  catch (_) { return null; }
});
ipcMain.handle('save-project', async (_, project) => {
  const result = await dialog.showSaveDialog(window, { defaultPath: safeName(project.name) + '.folio.json', filters: [{ name: 'Folio project', extensions: ['json'] }] });
  if (result.canceled) return false;
  await fs.writeFile(result.filePath, JSON.stringify(project, null, 2), 'utf8');
  return true;
});
ipcMain.handle('open-project', async () => {
  const result = await dialog.showOpenDialog(window, { properties: ['openFile'], filters: [{ name: 'Folio project', extensions: ['json'] }] });
  if (result.canceled) return null;
  const stat = await fs.stat(result.filePaths[0]);
  if (stat.size > 150 * 1024 * 1024) throw new Error('Project exceeds the 150 MB limit.');
  return JSON.parse(await fs.readFile(result.filePaths[0], 'utf8'));
});
ipcMain.handle('list-fun-tech-projects', () => funTechProjects.map(({ slug, name, slides }) => ({ slug, name, slides })));
ipcMain.handle('open-fun-tech-project', async (_, slug) => {
  if (typeof slug !== 'string' || !funTechIds.has(slug)) throw new Error('This collection project is unavailable.');
  const file = path.join(__dirname, 'assets', 'projects', 'fun-tech', slug + '.folio.json');
  return JSON.parse(await fs.readFile(file, 'utf8'));
});
ipcMain.handle('get-project-folder', projectFolder);
ipcMain.handle('choose-project-folder', async () => {
  const current = await projectFolder();
  const suggested = path.join(app.getPath('documents'), 'Folio Projects');
  const defaultPath = current || await fs.stat(suggested).then(stat => stat.isDirectory() ? suggested : app.getPath('documents')).catch(() => app.getPath('documents'));
  const result = await dialog.showOpenDialog(window, { title: 'Choose your Folio projects folder', defaultPath, properties: ['openDirectory', 'createDirectory'] });
  if (result.canceled || !result.filePaths?.[0]) return null;
  const selected = path.resolve(result.filePaths[0]);
  if (!(await fs.stat(selected)).isDirectory()) throw new Error('Choose a folder for your projects.');
  await fs.mkdir(app.getPath('userData'), { recursive: true });
  const temporary = folderConfigPath() + '.' + process.pid + '.tmp';
  await fs.writeFile(temporary, JSON.stringify({ path: selected }), 'utf8');
  await fs.rename(temporary, folderConfigPath());
  return selected;
});
ipcMain.handle('list-folder-projects', async () => {
  const folder = await projectFolder();
  if (!folder) return [];
  return folderProjects(folder);
});
ipcMain.handle('open-folder-project', async (_, fileName) => {
  const folder = await projectFolder();
  if (!folder || !validProjectFileName(fileName)) throw new Error('Choose a valid project from the folder.');
  const file = path.join(folder, fileName);
  if (!(await fs.lstat(file)).isFile()) throw new Error('This project file is unavailable.');
  if ((await fs.stat(file)).size > 150 * 1024 * 1024) throw new Error('Project exceeds 150 MB.');
  const value = JSON.parse(await fs.readFile(file, 'utf8'));
  value.folderFileName = fileName;
  return value;
});
ipcMain.handle('save-folder-project', async (_, value) => {
  const folder = await projectFolder();
  if (!folder) throw new Error('Choose a project folder first.');
  if (!value || value.version !== 1 || !Array.isArray(value.slides) || value.slides.length < 1 || value.slides.length > 20 || typeof value.name !== 'string') throw new Error('Invalid Folio project.');
  if (JSON.stringify(value).length > 150 * 1024 * 1024) throw new Error('Project exceeds 150 MB.');
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
ipcMain.handle('export-slides', async (_, { name, slides }) => {
  if (!Array.isArray(slides) || slides.length < 1 || slides.length > 20 || slides.some(s => !/^data:image\/png;base64,/.test(s))) throw new Error('Invalid export.');
  const single = slides.length === 1;
  const result = await dialog.showSaveDialog(window, { defaultPath: safeName(name) + (single ? '.png' : '.zip'), filters: [{ name: single ? 'PNG image' : 'ZIP archive', extensions: [single ? 'png' : 'zip'] }] });
  if (result.canceled) return false;
  const buffers = slides.map(s => Buffer.from(s.split(',')[1], 'base64'));
  if (single) await fs.writeFile(result.filePath, buffers[0]);
  else {
    const zip = new JSZip();
    buffers.forEach((buffer, i) => zip.file(`${String(i + 1).padStart(2, '0')}-${safeName(name)}.png`, buffer));
    await fs.writeFile(result.filePath, await zip.generateAsync({ type: 'nodebuffer' }));
  }
  return true;
});
