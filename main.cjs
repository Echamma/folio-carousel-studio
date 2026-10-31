const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const fsSync = require('node:fs');
const path = require('node:path');
const folioHost = require('./app-host.cjs');
if (process.env.FOLIO_TEST !== '1') {
  const dataPath = path.join(app.getPath('appData'), 'folio-carousel-studio');
  fsSync.mkdirSync(dataPath, { recursive: true });
  app.setPath('userData', dataPath);
}
let window;
let closing = false;
let allowClose = false;
const host = folioHost.create({ ipc: ipcMain, dataDir: app.getPath('userData'), documentsDir: app.getPath('documents'), window: () => window, webContents: () => window?.webContents });
app.whenReady().then(() => {
  window = new BrowserWindow({ width: 1500, height: 980, minWidth: 1100, minHeight: 760, show: process.env.FOLIO_TEST !== '1', backgroundColor: '#f6f5f1', title: 'Folio — Carousel Studio', autoHideMenuBar: true, webPreferences: { preload: host.preload, contextIsolation: true, nodeIntegration: false, sandbox: true } });
  window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  window.webContents.on('will-navigate', (event) => event.preventDefault());
  window.on('close', async event => {
    if (allowClose) return;
    event.preventDefault();
    if (closing) return;
    closing = true;
    try {
      let timer;
      await Promise.race([
        host.checkpoint(),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('The editor did not respond.')), 5000); })
      ]).finally(() => clearTimeout(timer));
      allowClose = true;
      window.close();
    } catch (error) {
      const result = await dialog.showMessageBox(window, { type: 'warning', title: 'Save before closing', message: 'Your latest changes could not be saved automatically.', detail: 'Choose Cancel to return to the editor and use Save project.\n\n' + error.message, buttons: ['Cancel', 'Close without saving'], defaultId: 0, cancelId: 0 });
      if (result.response === 1) { allowClose = true; window.close(); }
    } finally { closing = false; }
  });
  window.loadFile(host.entry);
});
app.on('window-all-closed', () => app.quit());
