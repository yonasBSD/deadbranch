#!/usr/bin/env node

/**
 * npm postinstall script for deadbranch
 * Downloads the appropriate pre-built binary for the current platform
 */

const { execSync, spawn } = require('child_process');
const fs = require('fs');
const https = require('https');
const path = require('path');
const os = require('os');
const zlib = require('zlib');

const REPO = 'armgabrielyan/deadbranch';
const BINARY_NAME = 'deadbranch';
const VERSION = require('../package.json').version;

/**
 * Get the target triple for the current platform
 */
function getTarget() {
  const platform = os.platform();
  const arch = os.arch();

  const targets = {
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin',
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'linux-arm64': 'aarch64-unknown-linux-gnu',
    'win32-x64': 'x86_64-pc-windows-msvc',
  };

  const key = `${platform}-${arch}`;
  const target = targets[key];

  if (!target) {
    throw new Error(`Unsupported platform: ${key}`);
  }

  return target;
}

/**
 * Get the archive extension for the current platform
 */
function getArchiveExt() {
  return os.platform() === 'win32' ? 'zip' : 'tar.gz';
}

/**
 * Download a file from a URL
 */
function download(url) {
  return new Promise((resolve, reject) => {
    const handleResponse = (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        https.get(response.headers.location, handleResponse).on('error', reject);
        return;
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode}`));
        return;
      }

      const chunks = [];
      response.on('data', (chunk) => chunks.push(chunk));
      response.on('end', () => resolve(Buffer.concat(chunks)));
      response.on('error', reject);
    };

    https.get(url, handleResponse).on('error', reject);
  });
}

/**
 * Extract tar.gz archive
 */
function extractTarGz(buffer, destDir) {
  const tarPath = path.join(os.tmpdir(), 'deadbranch.tar');

  // Decompress gzip
  const decompressed = zlib.gunzipSync(buffer);
  fs.writeFileSync(tarPath, decompressed);

  // Extract tar
  execSync(`tar -xf "${tarPath}" -C "${destDir}"`, { stdio: 'inherit' });
  fs.unlinkSync(tarPath);
}

/**
 * Extract zip archive (Windows)
 */
function extractZip(buffer, destDir) {
  const zipPath = path.join(os.tmpdir(), 'deadbranch.zip');
  fs.writeFileSync(zipPath, buffer);

  // Use PowerShell to extract
  execSync(`powershell -Command "Expand-Archive -Path '${zipPath}' -DestinationPath '${destDir}' -Force"`, { stdio: 'inherit' });
  fs.unlinkSync(zipPath);
}

async function install() {
  console.log('Installing deadbranch...');

  try {
    const target = getTarget();
    const ext = getArchiveExt();
    const archiveName = `deadbranch-${VERSION}-${target}.${ext}`;
    const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${archiveName}`;

    console.log(`Downloading ${archiveName}...`);
    const buffer = await download(url);

    const binDir = path.join(__dirname, '..', 'bin');
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    // Create temp dir for extraction
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'deadbranch-'));

    console.log('Extracting...');
    if (ext === 'zip') {
      extractZip(buffer, tmpDir);
    } else {
      extractTarGz(buffer, tmpDir);
    }

    // Copy binary to bin directory
    const binaryExt = os.platform() === 'win32' ? '.exe' : '';
    const srcBinary = path.join(tmpDir, BINARY_NAME + binaryExt);
    const destBinary = path.join(binDir, BINARY_NAME + binaryExt);

    fs.copyFileSync(srcBinary, destBinary);

    // Make executable on Unix
    if (os.platform() !== 'win32') {
      fs.chmodSync(destBinary, 0o755);
    }

    // Cleanup temp dir
    fs.rmSync(tmpDir, { recursive: true, force: true });

    console.log('deadbranch installed successfully!');
  } catch (error) {
    console.error('Failed to install deadbranch:', error.message);
    console.error('');
    console.error('You can install from source instead:');
    console.error('  cargo install deadbranch');
    process.exit(1);
  }
}

install();
