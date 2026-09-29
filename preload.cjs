const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('desktop', {
  loadRecovery: () => ipcRenderer.invoke('load-recovery'),
  saveProject: project => ipcRenderer.invoke('save-project', project),
  openProject: () => ipcRenderer.invoke('open-project'),
  exportSlides: data => ipcRenderer.invoke('export-slides', data)
});
