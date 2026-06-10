import fs from 'node:fs';
import path from 'node:path';

// Builds the static updater JSON consumed by the Tauri updater plugin:
// https://v2.tauri.app/plugin/updater/#static-json-file

const [assetsRoot, tag, version] = process.argv.slice(2);
const repository = process.env.GITHUB_REPOSITORY;

if (!assetsRoot || !tag || !version || !repository) {
  throw new Error('Usage: create-updater-latest.js <assets-root> <tag> <version>');
}

const readJson = (filePath) => JSON.parse(fs.readFileSync(filePath, 'utf8'));

const listFiles = (directory) => {
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

const releaseAssetUrl = (assetName) =>
  `https://github.com/${repository}/releases/download/${tag}/${assetName}`;

const updaterManifests = listFiles(assetsRoot)
  .filter((filePath) => path.basename(filePath).startsWith('updater-'))
  .map(readJson);

if (updaterManifests.length === 0) {
  throw new Error('No updater manifests found in downloaded artifacts');
}

const platforms = {};

for (const manifest of updaterManifests) {
  platforms[manifest.platform] = {
    signature: manifest.signature,
    url: releaseAssetUrl(manifest.assetName)
  };
}

const latest = {
  version,
  notes: `Release ${tag}`,
  pub_date: new Date().toISOString(),
  platforms
};

fs.writeFileSync(path.join(assetsRoot, 'latest.json'), `${JSON.stringify(latest, null, 2)}\n`);
