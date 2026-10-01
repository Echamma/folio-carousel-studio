const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('desktop', {
  loadRecovery: () => ipcRenderer.invoke('load-recovery'),
  saveProject: project => ipcRenderer.invoke('save-project', project),
  openProject: () => ipcRenderer.invoke('open-project'),
  listFunTechProjects: () => ipcRenderer.invoke('list-fun-tech-projects'),
  openFunTechProject: slug => ipcRenderer.invoke('open-fun-tech-project', slug),
  getProjectFolder: () => ipcRenderer.invoke('get-project-folder'),
  chooseProjectFolder: () => ipcRenderer.invoke('choose-project-folder'),
  listFolderProjects: () => ipcRenderer.invoke('list-folder-projects'),
  openFolderProject: fileName => ipcRenderer.invoke('open-folder-project', fileName),
  saveFolderProject: project => ipcRenderer.invoke('save-folder-project', project),
  exportSlides: data => ipcRenderer.invoke('export-slides', data)
});
