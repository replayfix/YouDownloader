import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const target = process.env.CARGO_TARGET_DIR
  ? resolve(root, process.env.CARGO_TARGET_DIR)
  : join(root, "src-tauri", "target");
const exe = join(target, "release", "jc-youdownloader.exe");
if (!existsSync(exe)) throw new Error("Compile the Windows release first.");
const dest = join(root, "portable");

if (process.platform === "win32") {
  const release = "https://github.com/invzfnc/spotify-downloader/releases/download/v1.4.0/spotify-dl-1.4.0-win.zip";
  const expectedHash = "a2c9ed0eec17059e10ec9010e68e84dfb2f26534ce0ec9485c8f8fb7ecbc38f5";
  const response = await fetch(release, { signal: AbortSignal.timeout(120_000) });
  if (!response.ok) throw new Error(`spotify-dl download failed (${response.status})`);
  const zip = Buffer.from(await response.arrayBuffer());
  if (createHash("sha256").update(zip).digest("hex") !== expectedHash) {
    throw new Error("spotify-dl archive checksum mismatch");
  }
  const temp = mkdtempSync(join(tmpdir(), "youdownloader-spotify-"));
  try {
    const archive = join(temp, "spotify-dl.zip");
    const extracted = join(temp, "spotify-dl");
    writeFileSync(archive, zip);
    mkdirSync(extracted);
    execFileSync("tar", ["-xf", archive, "-C", extracted], { stdio: "inherit" });
    if (!existsSync(join(extracted, "spotify-dl.exe")) || !existsSync(join(extracted, "_internal"))) {
      throw new Error("Unexpected spotify-dl release layout");
    }
    writeFileSync(join(extracted, "LICENSE"), "MIT License\r\n\r\nCopyright (c) 2025 Cha\r\n\r\nPermission is hereby granted, free of charge, to any person obtaining a copy\r\nof this software and associated documentation files (the \"Software\"), to deal\r\nin the Software without restriction, including without limitation the rights\r\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\r\ncopies of the Software, and to permit persons to whom the Software is\r\nfurnished to do so, subject to the following conditions:\r\n\r\nThe above copyright notice and this permission notice shall be included in all\r\ncopies or substantial portions of the Software.\r\n\r\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\r\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\r\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\r\nAUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\r\nLIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\r\nOUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\r\nSOFTWARE.");
    cpSync(extracted, join(dest, "binaries", "spotify-dl"), { recursive: true });
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

mkdirSync(dest, { recursive: true });
cpSync(exe, join(dest, "JC_YouDownloader.exe"));
cpSync(join(root, "src-tauri", "binaries"), join(dest, "binaries"), { recursive: true });
writeFileSync(join(dest, "portable.flag"), "");
const config = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
writeFileSync(join(dest, ".portable-version.json"), JSON.stringify({ version: config.version }));
console.log(`Portable folder: ${dest}`);
