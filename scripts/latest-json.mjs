import { access, readFile, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const configPath = path.join(root, "src-tauri", "tauri.conf.json");
const config = JSON.parse(await readFile(configPath, "utf8"));
const version = config.version;
if (typeof version !== "string" || !version) {
  throw new Error(`Could not read a version from ${path.relative(root, configPath)}.`);
}

const bundleDir = path.join(root, "src-tauri", "target", "release", "bundle");
const setupName = `KeepShot_${version}_x64-setup.exe`;
const setupPath = path.join(bundleDir, "nsis", setupName);
const signaturePath = `${setupPath}.sig`;
await requireFile(setupPath, "Signed NSIS installer");
await requireFile(signaturePath, "Updater signature");

const notesPath = process.argv[2];
const notes = notesPath
  ? await readFile(path.resolve(process.cwd(), notesPath), "utf8")
  : "";
const signature = (await readFile(signaturePath, "utf8")).trim();
if (!signature) throw new Error(`The signature file is empty: ${signaturePath}`);

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/SaGgaSsa/keepshot/releases/download/v${version}/${setupName}`,
    },
  },
};

const outputPath = path.join(bundleDir, "latest.json");
await mkdir(bundleDir, { recursive: true });
await writeFile(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
console.log(`Wrote ${path.relative(root, outputPath)}`);

async function requireFile(filePath, label) {
  try {
    await access(filePath);
  } catch {
    throw new Error(`${label} is missing: ${filePath}`);
  }
}
