const { app, BrowserWindow, dialog } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

const root = path.resolve(process.argv[2]);
const phase = process.argv[3];
const folder = path.join(root, 'projects');
const profile = path.join(root, 'profile');
fs.mkdirSync(profile, { recursive: true });
app.setPath('userData', profile);
process.env.FOLIO_TEST = '1';

let prompts = 0;
dialog.showOpenDialog = async () => {
  prompts++;
  if (phase !== 'select') throw new Error('The folder chooser reopened after restart.');
  return { canceled: false, filePaths: [folder] };
};
require('../main.cjs');

const timeout = setTimeout(() => { console.error('Folder restart test timed out'); app.exit(1); }, 45000);
app.whenReady().then(async () => {
  try {
    const win = BrowserWindow.getAllWindows()[0];
    if (win.webContents.isLoading()) await new Promise(resolve => win.webContents.once('did-finish-load', resolve));
    await win.webContents.executeJavaScript('window.FolioReady');
    if (phase === 'select') {
      const chosen = await win.webContents.executeJavaScript('desktop.chooseProjectFolder()');
      assert.equal(chosen, folder);
      assert.equal(prompts, 1);
    } else {
      const result = await win.webContents.executeJavaScript(`(async () => {
        const remembered = await desktop.getProjectFolder();
        document.getElementById('open-project').click();
        for (let i = 0; i < 100 && !document.getElementById('project-dialog').open; i++) await new Promise(resolve => setTimeout(resolve, 100));
        return { remembered, inMemory: projectFolder, label: document.getElementById('projects-folder-path').textContent, pickerOpen: document.getElementById('project-dialog').open, names: [...document.querySelectorAll('#project-picker-list .project-tile-info strong')].map(node => node.textContent) };
      })()`);
      assert.equal(result.remembered, folder);
      assert.equal(result.inMemory, folder);
      assert.equal(result.label, folder);
      assert.equal(result.pickerOpen, true);
      assert.equal(prompts, 0);
      if (phase === 'restored') assert(result.names.includes('How GPS Finds You'));
      else assert.equal(result.names.length, 0);
    }
    console.log(`PASS: ${phase}`);
    clearTimeout(timeout); app.exit(0);
  } catch (error) { console.error(error); clearTimeout(timeout); app.exit(1); }
});
