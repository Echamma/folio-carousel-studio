const { contextBridge, ipcRenderer } = require('electron');
// Inside Canopy the hub passes --canopy={"space":"…"}; standalone Folio has no such argument.
const canopyArgument = process.argv.find(arg => arg.startsWith('--canopy='));
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
  exportSlides: data => ipcRenderer.invoke('export-slides', data),
  canopy: canopyArgument ? JSON.parse(canopyArgument.slice('--canopy='.length)) : null,
  onCanopyImages: callback => ipcRenderer.on('folio:canopy-images', (_, images) => callback(images)),
  onCanopyProject: callback => ipcRenderer.on('folio:canopy-project', (_, value) => callback(value)),
  onCanopyOpenProject: callback => ipcRenderer.on('folio:canopy-open-project', (_, fileName) => callback(fileName))
});
