import fs from 'node:fs';

const configPath = 'src-tauri/tauri.conf.json';
const pubkey = process.env.TAURI_UPDATER_PUBKEY;
const repository = process.env.GITHUB_REPOSITORY;
const version = process.env.RELEASE_VERSION;

if (!pubkey) {
  throw new Error('TAURI_UPDATER_PUBKEY must be configured as a GitHub variable or secret');
}

if (!repository || !version) {
  throw new Error('GITHUB_REPOSITORY and RELEASE_VERSION must be set');
}

const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

config.version = version;
config.bundle = config.bundle || {};
config.bundle.createUpdaterArtifacts = true;
config.plugins = config.plugins || {};
config.plugins.updater = {
  pubkey,
  endpoints: [`https://github.com/${repository}/releases/latest/download/latest.json`]
};

fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
