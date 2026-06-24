import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const uiRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const licensesPath = join(uiRoot, "src", "generated", "npm-licenses.json");

const raw = readFileSync(licensesPath, "utf8");
const entries = JSON.parse(raw);

function relativize(value) {
  if (typeof value !== "string" || value.length === 0) {
    return value;
  }
  const normalized = value.replace(/\\/g, "/");
  const uiRootNorm = uiRoot.replace(/\\/g, "/");
  if (normalized === uiRootNorm) {
    return ".";
  }
  if (normalized.startsWith(`${uiRootNorm}/`)) {
    return relative(uiRoot, value).replace(/\\/g, "/");
  }
  return value;
}

for (const entry of Object.values(entries)) {
  if (entry.path) {
    entry.path = relativize(entry.path);
  }
  if (entry.licenseFile) {
    entry.licenseFile = relativize(entry.licenseFile);
  }
}

writeFileSync(licensesPath, `${JSON.stringify(entries, null, 2)}\n`, "utf8");
console.log("Relativized paths in src/generated/npm-licenses.json");
