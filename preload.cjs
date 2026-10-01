const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('desktop', {
  loadRecovery: () => ipcRenderer.invoke('load-recovery'),
  saveProject: project => ipcRenderer.invoke('save-project', project),
  openProject: () => ipcRenderer.invoke('open-project'),
  listFunTechProjects: () => ipcRenderer.invoke('list-fun-tech-projects'),
  openFunTechProject: slug => ipcRenderer.invoke('open-fun-tech-project', slug),
  exportSlides: data => ipcRenderer.invoke('export-slides', data)
});
