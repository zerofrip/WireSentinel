#!/usr/bin/env node
/** Minimal RGB -> RGBA PNG converter (no deps). Tauri icons must be RGBA. */
import { readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { deflateSync, inflateSync } from 'node:zlib';
import { createHash } from 'node:crypto';

const iconsDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'icons');
const files = ['32x32.png', '128x128.png', '128x128@2x.png'];

function crc(buf) {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = table[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const t = Buffer.from(type, 'ascii');
  const crcBuf = Buffer.concat([t, data]);
  const c = Buffer.alloc(4);
  c.writeUInt32BE(crc(crcBuf));
  return Buffer.concat([len, t, data, c]);
}

function toRgba(path) {
  const buf = readFileSync(path);
  if (buf.toString('ascii', 1, 4) !== 'PNG') throw new Error(`${path}: not PNG`);
  let pos = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  let idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos);
    const type = buf.toString('ascii', pos + 4, pos + 8);
    const data = buf.subarray(pos + 8, pos + 8 + len);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
    } else if (type === 'IDAT') {
      idat.push(data);
    } else if (type === 'IEND') break;
    pos += 12 + len;
  }
  if (colorType === 6) {
    console.log(`${path}: already RGBA`);
    return;
  }
  if (colorType !== 2 || bitDepth !== 8) {
    throw new Error(`${path}: unsupported PNG (colorType=${colorType} bitDepth=${bitDepth})`);
  }
  const raw = inflateSync(Buffer.concat(idat));
  const bpp = 3;
  const stride = width * bpp + 1;
  const outRaw = Buffer.alloc(height * (width * 4 + 1));
  for (let y = 0; y < height; y++) {
    const filter = raw[y * stride];
    outRaw[y * (width * 4 + 1)] = filter;
    for (let x = 0; x < width; x++) {
      const si = y * stride + 1 + x * bpp;
      const di = y * (width * 4 + 1) + 1 + x * 4;
      outRaw[di] = raw[si];
      outRaw[di + 1] = raw[si + 1];
      outRaw[di + 2] = raw[si + 2];
      outRaw[di + 3] = 255;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6; // RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  const out = Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(outRaw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
  writeFileSync(path, out);
  console.log(`${path}: converted to RGBA ${width}x${height}`);
}

for (const name of files) {
  toRgba(join(iconsDir, name));
}
