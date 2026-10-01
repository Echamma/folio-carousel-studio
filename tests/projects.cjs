const { app, BrowserWindow } = require('electron');
const path = require('node:path');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');

process.env.FOLIO_TEST = '1';
app.setPath('userData', path.join(app.getPath('temp'), 'folio-projects-test-' + process.pid));
require('../main.cjs');
const timeout = setTimeout(() => { console.error('Project library test timed out'); app.exit(1); }, 60000);

app.whenReady().then(async () => {
  try {
    const win = BrowserWindow.getAllWindows()[0];
    if (win.webContents.isLoading()) await new Promise(resolve => win.webContents.once('did-finish-load', resolve));
    const first = await win.webContents.executeJavaScript(`(async () => {
      const waitFor = async check => {
        for (let i = 0; i < 120; i++) { if (check()) return; await new Promise(resolve => setTimeout(resolve, 100)); }
        throw new Error('Project library did not become ready');
      };
      await waitFor(() => db && document.querySelectorAll('#projects-list button[data-id]').length === 10);
      if (project.name !== 'Why Bluetooth Has a Viking Name') throw new Error('First carousel did not open in the editor');
      document.getElementById('open-project').click();
      await waitFor(() => document.getElementById('project-dialog').open && document.querySelectorAll('#project-picker-list button[data-id]').length === 10);
      const search = document.getElementById('project-search'); search.value = 'GPS'; search.dispatchEvent(new Event('input'));
      if (document.querySelectorAll('#project-picker-list button[data-id]').length !== 1) throw new Error('Project search did not filter the picker');
      document.querySelector('#project-picker-list button[data-id="02-gps"]').click();
      await waitFor(() => project.name === 'How GPS Finds You' && !document.getElementById('project-dialog').open);
      document.getElementById('new-project').click();
      await waitFor(() => project.name === 'Untitled carousel' && document.querySelectorAll('#projects-list button[data-id]').length === 11);
      const createdId = project.libraryId;
      const input = document.getElementById('project-name'); input.value = 'My new carousel'; input.dispatchEvent(new Event('input'));
      await saveProject();
      await openSavedProject('02-gps');
      if (project.name !== 'How GPS Finds You') throw new Error('Saved GPS carousel did not open');
      await openSavedProject(createdId);
      if (project.name !== 'My new carousel') throw new Error('New project was not saved');
      return { createdId, count: document.querySelectorAll('#projects-list button[data-id]').length };
    })()`);
    assert.equal(first.count, 11);
    const loading = new Promise(resolve => win.webContents.once('did-finish-load', resolve));
    win.webContents.reload(); await loading;
    const afterReload = await win.webContents.executeJavaScript(`(async () => {
      for (let i = 0; i < 120; i++) {
        if (db && document.querySelectorAll('#projects-list button[data-id]').length === 11) return { name: project.name, count: document.querySelectorAll('#projects-list button[data-id]').length };
        await new Promise(resolve => setTimeout(resolve, 100));
      }
      throw new Error('Saved project library did not reload');
    })()`);
    assert.equal(afterReload.name, 'My new carousel');
    assert.equal(afterReload.count, 11);
    await fs.mkdir(path.join(__dirname, '..', 'test-output'), { recursive: true });
    await fs.writeFile(path.join(__dirname, '..', 'test-output', 'projects-ui.png'), (await win.webContents.capturePage()).toPNG());
    console.log('PASS: Ten carousels load in Folio; New, Save, Open, and reload preserve projects');
    clearTimeout(timeout); app.exit(0);
  } catch (error) { console.error(error); clearTimeout(timeout); app.exit(1); }
});
