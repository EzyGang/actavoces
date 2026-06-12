import fs from 'node:fs';
import path from 'node:path';

const [assetsRoot, uploadRoot, tag, version] = process.argv.slice(2);
const repository = process.env.GITHUB_REPOSITORY;

if (!assetsRoot || !uploadRoot || !tag || !version || !repository) {
  throw new Error(
    'Usage: prepare-release-assets.js <assets-root> <upload-root> <tag> <version>'
  );
}

const platformTargets = {
  'darwin-aarch64': {
    assetSuffix: 'aarch64',
    target: 'aarch64-apple-darwin'
  },
  'darwin-x86_64': {
    assetSuffix: 'x64',
    target: 'x86_64-apple-darwin'
  },
  'linux-x86_64': {
    assetSuffix: 'amd64',
    target: 'x86_64-unknown-linux-gnu'
  },
  'windows-x86_64': {
    assetSuffix: 'x64',
    target: 'x86_64-pc-windows-msvc'
  }
};

const releaseExtensions = [
  '.AppImage',
  '.deb',
  '.dmg',
  '.exe',
  '.msi',
  '.rpm',
  '.app.tar.gz'
];

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

const isReleaseAsset = (filePath) => {
  const basename = path.basename(filePath);
  const unsignedName = basename.endsWith('.sig') ? basename.slice(0, -4) : basename;

  return releaseExtensions.some((extension) => unsignedName.endsWith(extension));
};

const targetForPath = (filePath) =>
  Object.values(platformTargets).find(({ target }) => filePath.includes(target));

const uploadAssetName = (filePath) => {
  const basename = path.basename(filePath);
  const target = targetForPath(filePath);

  if (
    target &&
    (basename === 'ActaVoces.app.tar.gz' || basename === 'ActaVoces.app.tar.gz.sig')
  ) {
    const suffix = basename.endsWith('.sig') ? '.sig' : '';

    return `ActaVoces_${version}_${target.assetSuffix}.app.tar.gz${suffix}`;
  }

  return basename;
};

const downloadAliasName = (filePath) => {
  const basename = path.basename(filePath);
  const isSignature = basename.endsWith('.sig');
  const unsignedName = isSignature ? basename.slice(0, -4) : basename;
  const target = targetForPath(filePath);

  if (!target) {
    return null;
  }

  const signatureSuffix = isSignature ? '.sig' : '';

  if (target.target === 'x86_64-pc-windows-msvc') {
    if (unsignedName.endsWith('-setup.exe')) {
      return `ActaVoces-windows-x64-setup.exe${signatureSuffix}`;
    }

    if (unsignedName.endsWith('.msi')) {
      return `ActaVoces-windows-x64.msi${signatureSuffix}`;
    }
  }

  if (target.target === 'aarch64-apple-darwin' && unsignedName.endsWith('.dmg')) {
    return `ActaVoces-macos-aarch64.dmg${signatureSuffix}`;
  }

  if (target.target === 'x86_64-apple-darwin' && unsignedName.endsWith('.dmg')) {
    return `ActaVoces-macos-x64.dmg${signatureSuffix}`;
  }

  if (target.target === 'x86_64-unknown-linux-gnu') {
    if (unsignedName.endsWith('.AppImage')) {
      return `ActaVoces-linux-x64.AppImage${signatureSuffix}`;
    }

    if (unsignedName.endsWith('.deb')) {
      return `ActaVoces-linux-x64.deb${signatureSuffix}`;
    }

    if (unsignedName.endsWith('.rpm')) {
      return `ActaVoces-linux-x64.rpm${signatureSuffix}`;
    }
  }

  return null;
};

const copyAsset = (copied, sourcePath, assetName) => {
  const existingPath = copied.get(assetName);

  if (existingPath) {
    throw new Error(
      `Release asset name collision for ${assetName}: ${existingPath} and ${sourcePath}`
    );
  }

  copied.set(assetName, sourcePath);
  fs.copyFileSync(sourcePath, path.join(uploadRoot, assetName));
};

const copyReleaseAssets = (files) => {
  fs.rmSync(uploadRoot, { force: true, recursive: true });
  fs.mkdirSync(uploadRoot, { recursive: true });

  const copied = new Map();

  for (const filePath of files.filter(isReleaseAsset)) {
    const assetName = uploadAssetName(filePath);
    const aliasName = downloadAliasName(filePath);

    copyAsset(copied, filePath, assetName);
    if (aliasName && aliasName !== assetName) {
      copyAsset(copied, filePath, aliasName);
    }
  }

  return copied;
};

const updaterAssetName = (manifest, files) => {
  const platform = platformTargets[manifest.platform];

  if (!platform) {
    throw new Error(`Unknown updater platform: ${manifest.platform}`);
  }

  const candidates = files.filter(
    (filePath) =>
      filePath.includes(platform.target) && path.basename(filePath) === manifest.assetName
  );

  if (candidates.length !== 1) {
    throw new Error(
      `Expected one updater asset for ${manifest.platform} named ${manifest.assetName}, found ${candidates.length}`
    );
  }

  return uploadAssetName(candidates[0]);
};

const files = listFiles(assetsRoot);
const copied = copyReleaseAssets(files);
const updaterManifests = files
  .filter((filePath) => path.basename(filePath).startsWith('updater-'))
  .map(readJson)
  .sort((left, right) => left.platform.localeCompare(right.platform));

if (updaterManifests.length === 0) {
  throw new Error('No updater manifests found in downloaded artifacts');
}

const expectedPlatforms = Object.keys(platformTargets).sort();
const updaterPlatforms = updaterManifests.map((manifest) => manifest.platform).sort();

if (JSON.stringify(updaterPlatforms) !== JSON.stringify(expectedPlatforms)) {
  throw new Error(
    `Expected updater manifests for ${expectedPlatforms.join(', ')}, found ${updaterPlatforms.join(', ')}`
  );
}

const platforms = {};

for (const manifest of updaterManifests) {
  const assetName = updaterAssetName(manifest, files);

  if (!copied.has(assetName) || !copied.has(`${assetName}.sig`)) {
    throw new Error(`Missing release asset or signature for updater asset: ${assetName}`);
  }

  platforms[manifest.platform] = {
    signature: manifest.signature,
    url: releaseAssetUrl(assetName)
  };
}

const latest = {
  version,
  notes: `Release ${tag}`,
  pub_date: new Date().toISOString(),
  platforms
};

fs.writeFileSync(path.join(uploadRoot, 'latest.json'), `${JSON.stringify(latest, null, 2)}\n`);
