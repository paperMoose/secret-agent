#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const pkg = require("./package.json");
const VERSION = pkg.version;

const PLATFORM_MAP = {
  "darwin-x64": "secret-agent-macos-x86_64",
  "darwin-arm64": "secret-agent-macos-aarch64",
  "linux-x64": "secret-agent-linux-x86_64",
};

const binDir = path.join(__dirname, "bin");
const binPath = path.join(binDir, "secret-agent");

// Skip if binary already exists (cached node_modules)
if (fs.existsSync(binPath)) {
  process.exit(0);
}

const key = `${process.platform}-${process.arch}`;
const artifact = PLATFORM_MAP[key];

if (!artifact) {
  console.error(
    `secret-agent-cli: unsupported platform ${process.platform}-${process.arch}`
  );
  console.error(
    "Supported: darwin-x64, darwin-arm64, linux-x64"
  );
  process.exit(1);
}

const url = `https://github.com/paperMoose/secret-agent/releases/download/v${VERSION}/${artifact}.tar.gz`;

fs.mkdirSync(binDir, { recursive: true });

const tarball = path.join(binDir, `${artifact}.tar.gz`);

function download(url, dest, callback) {
  const file = fs.createWriteStream(dest);
  https
    .get(url, (res) => {
      // Follow redirects (GitHub → S3)
      if (res.statusCode === 301 || res.statusCode === 302) {
        file.close();
        fs.unlinkSync(dest);
        download(res.headers.location, dest, callback);
        return;
      }

      if (res.statusCode !== 200) {
        file.close();
        fs.unlinkSync(dest);
        callback(
          new Error(`Download failed: HTTP ${res.statusCode} from ${url}`)
        );
        return;
      }

      res.pipe(file);
      file.on("finish", () => file.close(callback));
    })
    .on("error", (err) => {
      file.close();
      if (fs.existsSync(dest)) fs.unlinkSync(dest);
      callback(err);
    });
}

console.log(`Downloading secret-agent v${VERSION} for ${key}...`);

download(url, tarball, (err) => {
  if (err) {
    // Don't fail the install — user can install via cargo instead
    console.error(`secret-agent-cli: failed to download binary: ${err.message}`);
    process.exit(0);
  }

  try {
    execSync(`tar -xzf "${tarball}" -C "${binDir}"`);
    fs.unlinkSync(tarball);
    fs.chmodSync(binPath, 0o755);
    console.log("secret-agent installed successfully.");
  } catch (e) {
    console.error(`secret-agent-cli: failed to extract binary: ${e.message}`);
    process.exit(0);
  }
});
