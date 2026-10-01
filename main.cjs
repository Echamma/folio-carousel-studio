const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const fs = require('node:fs/promises');
const path = require('node:path');
const JSZip = require('jszip');
const funTechProjects = require('./assets/projects/fun-tech/manifest.json');
const funTechIds = new Set(funTechProjects.map(project => project.slug));
let window;
let closing = false;
let allowClose = false;
const recoveryPath = () => path.join(app.getPath('userData'), 'last-session.json');
function safeName(name) { return String(name || 'carousel').replace(/[<>:"/\\|?*\x00-\x1f]/g, '-').slice(0, 100) || 'carousel'; }
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
