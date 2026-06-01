import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

const root = fileURLToPath(new URL("..", import.meta.url));
const outPath = join(root, "src-tauri", "icons", "icon.png");

function crc32(buffer) {
  let crc = 0xffffffff;

  for (const byte of buffer) {
    crc ^= byte;

    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
    }
  }

  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type, "ascii");
  const length = Buffer.alloc(4);
  const crc = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])));
  return Buffer.concat([length, typeBuffer, data, crc]);
}

function png(width, height, drawPixel) {
  const raw = Buffer.alloc((width * 4 + 1) * height);

  for (let y = 0; y < height; y += 1) {
    const row = y * (width * 4 + 1);
    raw[row] = 0;

    for (let x = 0; x < width; x += 1) {
      const offset = row + 1 + x * 4;
      const [r, g, b, a] = drawPixel(x, y);
      raw[offset] = r;
      raw[offset + 1] = g;
      raw[offset + 2] = b;
      raw[offset + 3] = a;
    }
  }

  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;
  header[10] = 0;
  header[11] = 0;
  header[12] = 0;

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const image = png(64, 64, (x, y) => {
  const dx = x - 32;
  const dy = y - 32;
  const inside = dx * dx + dy * dy <= 30 * 30;

  if (!inside) {
    return [0, 0, 0, 0];
  }

  const isTapStem = x >= 28 && x <= 35 && y >= 16 && y <= 48;
  const isTapTop = x >= 18 && x <= 46 && y >= 16 && y <= 22;
  const isTapBase = x >= 20 && x <= 44 && y >= 41 && y <= 47;

  if (isTapStem || isTapTop || isTapBase) {
    return [248, 250, 252, 255];
  }

  return [71, 85, 105, 255];
});

mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, image);
