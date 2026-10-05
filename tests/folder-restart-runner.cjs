const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const electron = require('electron');

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'folio-folder-restart-'));
const folder = path.join(root, 'projects');
const hidden = path.join(root, 'projects-unavailable');
fs.mkdirSync(folder);
fs.copyFileSync(path.join(__dirname, '..', 'assets', 'projects', 'fun-tech', '02-gps.folio.json'), path.join(folder, '02-gps.folio.json'));

function phase(name) {
  const env = { ...process.env, FOLIO_TEST: '1' };
  delete env.ELECTRON_RUN_AS_NODE;
  const result = spawnSync(electron, [path.join(__dirname, 'folder-restart.cjs'), root, name], { env, encoding: 'utf8', timeout: 60000 });
  if (result.error || result.status !== 0) throw new Error(`${name}: ${result.error || result.stderr || result.stdout}`);
  process.stdout.write(result.stdout);
}

try {
  phase('select');
  fs.renameSync(folder, hidden);
  phase('unavailable');
  fs.renameSync(hidden, folder);
  phase('restored');
  console.log('PASS: Folder choice survives process restarts, temporary unavailability, and restoration.');
} finally {
  if (path.resolve(root).startsWith(path.resolve(os.tmpdir()) + path.sep)) fs.rmSync(root, { recursive: true, force: true });
}
