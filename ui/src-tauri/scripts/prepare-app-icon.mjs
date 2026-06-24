#!/usr/bin/env node
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const sharp = require('sharp');

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const src = process.argv[2];
const out = join(root, 'app-icon.png');

if (!src) {
  console.error('Usage: node prepare-app-icon.mjs <source-image>');
  process.exit(1);
}

await sharp(src).ensureAlpha().png().toFile(out);
console.log(`Wrote ${out}`);
