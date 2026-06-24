import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const uiRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = join(uiRoot, "..");
const outDir = join(uiRoot, "public", "legal");

mkdirSync(outDir, { recursive: true });
copyFileSync(
  join(repoRoot, "installer", "THIRD_PARTY_NOTICES.txt"),
  join(outDir, "THIRD_PARTY_NOTICES.txt"),
);

console.log("Synced THIRD_PARTY_NOTICES.txt to public/legal/");
