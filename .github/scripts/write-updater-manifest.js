import fs from 'node:fs';
import path from 'node:path';

const platform = process.env.UPDATER_PLATFORM;
const target = process.env.RUST_TARGET;

if (!platform || !target) {
  throw new Error('UPDATER_PLATFORM and RUST_TARGET must be set');
}

const bundleRoot = path.join('src-tauri', 'target', target, 'release', 'bundle');

const listFiles = (directory) => {
  if (!fs.existsSync(directory)) {
    return [];
  }

  const files = [];

  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      files.push(...listFiles(entryPath));
      continue;
    }

    files.push(entryPath);
  }

  return files;
};

const byPreference = (candidates) => {
  const artifact = candidates.find((filePath) => fs.existsSync(`${filePath}.sig`));

  if (!artifact) {
    throw new Error(`No signed updater artifact found for ${platform}`);
  }

  return artifact;
};

const files = listFiles(bundleRoot).filter((filePath) => !filePath.endsWith('.sig'));
const artifact = platform.startsWith('darwin-')
  ? byPreference(files.filter((filePath) => filePath.endsWith('.app.tar.gz')))
  : platform === 'linux-x86_64'
    ? byPreference(files.filter((filePath) => filePath.endsWith('.AppImage')))
    : byPreference([
        ...files.filter((filePath) => filePath.endsWith('.msi')),
        ...files.filter((filePath) => filePath.endsWith('.exe'))
      ]);

const manifest = {
  platform,
  assetName: path.basename(artifact),
  signature: fs.readFileSync(`${artifact}.sig`, 'utf8').trim()
};

fs.writeFileSync(`updater-${platform}.json`, `${JSON.stringify(manifest, null, 2)}\n`);
