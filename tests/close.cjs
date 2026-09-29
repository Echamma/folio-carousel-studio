const { app, BrowserWindow } = require('electron');
const fs = require('node:fs/promises');
const path = require('node:path');
const assert = require('node:assert/strict');
const profile = path.join(__dirname, '..', 'test-output', 'close-profile');
app.setPath('userData', profile);
process.env.FOLIO_TEST = '1';
require('../main.cjs');
const timeout = setTimeout(() => { console.error('Close regression timed out'); app.exit(1); }, 20000);
app.whenReady().then(async () => {
  try {
    app.removeAllListeners('window-all-closed');
    const win = BrowserWindow.getAllWindows()[0];
    if (win.webContents.isLoading()) await new Promise(resolve => win.webContents.once('did-finish-load', resolve));
    await win.webContents.executeJavaScript(`(async()=>{while(!db)await new Promise(r=>setTimeout(r,20)); syncControls(); $('title').value='Saved on immediate close'; $('title').dispatchEvent(new Event('input'));})()`);
    const closed = new Promise(resolve => win.once('closed', resolve));
    win.close(); await closed;
    assert(win.isDestroyed());
    const recovery = JSON.parse(await fs.readFile(path.join(profile, 'last-session.json'), 'utf8'));
    assert.equal(recovery.slides[0].title, 'Saved on immediate close');
    assert(recovery.savedAt > 0);
    const reopened = new BrowserWindow({ show: false, webPreferences: { preload: path.join(__dirname, '..', 'preload.cjs'), contextIsolation: true, sandbox: true, nodeIntegration: false } });
    await reopened.loadFile(path.join(__dirname, '..', 'index.html'));
    const title = await reopened.webContents.executeJavaScript(`(async()=>{while(!db || !document.getElementById('title').value)await new Promise(r=>setTimeout(r,20));return document.getElementById('title').value;})()`);
    assert.equal(title, 'Saved on immediate close');
    console.log('PASS: Close immediately after editing exits, saves a recovery file, and restores latest edits on reopening.');
    clearTimeout(timeout); app.exit(0);
  } catch (error) { console.error(error); clearTimeout(timeout); app.exit(1); }
});
