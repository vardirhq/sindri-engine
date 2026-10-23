/**
 * The browser's PNG writer, standing in for `src/png.ts` in the web build.
 *
 * `src/png.ts` leans on `node:zlib` and Node's `Buffer`, and a page has neither.
 * This writes the same file layout -- 8-bit RGBA, filter 0 on every row, one
 * IDAT -- through a small DEFLATE of its own: greedy LZ77 over a hash chain and
 * one fixed-Huffman block. It compresses a sprite sheet, which is long runs of
 * transparent and flat colors, to within a modest factor of zlib's level 9, and
 * it is deterministic, so a bake in a page writes the same bytes every time.
 *
 * Only `encodePng` is here. The decoder exists for the regression tests, which
 * run under Node against `src/png.ts`, and the web build never imports it.
 */

import { type RgbaImage } from '../src/image.ts';

const SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

export function encodePng(image: RgbaImage): Uint8Array {
  const { width, height } = image;
  const stride = width * 4;
  const raw = new Uint8Array((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    // The filter byte stays 0 (None); see `src/png.ts` for why no predictor.
    raw.set(image.data.subarray(y * stride, (y + 1) * stride), y * (stride + 1) + 1);
  }

  const header = new Uint8Array(13);
  const view = new DataView(header.buffer);
  view.setUint32(0, width);
  view.setUint32(4, height);
  header[8] = 8; // bit depth
  header[9] = 6; // color type: truecolor with alpha

  const parts = [
    Uint8Array.from(SIGNATURE),
    chunk('IHDR', header),
    chunk('IDAT', zlib(raw)),
    chunk('IEND', new Uint8Array(0)),
  ];
  const out = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

function chunk(type: string, data: Uint8Array): Uint8Array {
  const out = new Uint8Array(data.length + 12);
  const view = new DataView(out.buffer);
  view.setUint32(0, data.length);
  for (let i = 0; i < 4; i++) out[4 + i] = type.charCodeAt(i);
  out.set(data, 8);
  view.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)));
  return out;
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

/** The CRC PNG chunks and zip entries both carry. */
export function crc32(data: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of data) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function adler32(data: Uint8Array): number {
  let a = 1;
  let b = 0;
  for (const byte of data) {
    a = (a + byte) % 65521;
    b = (b + a) % 65521;
  }
  return ((b << 16) | a) >>> 0;
}

/** Match length codes 257..285: the shortest length each covers, and its extra bits. */
const LENGTH_BASE = [
  3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163,
  195, 227, 258,
];
const LENGTH_EXTRA = [
  0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/** Distance codes 0..29, the same way. */
const DISTANCE_BASE = [
  1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049,
  3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA = [
  0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

const WINDOW = 32768;
const MAX_MATCH = 258;
const MIN_MATCH = 3;
const MAX_CHAIN = 48;

/** Bits go out least significant first; Huffman codes go out most significant first. */
class BitWriter {
  bytes: number[] = [];
  private buffer = 0;
  private count = 0;

  bits(value: number, length: number): void {
    for (let i = 0; i < length; i++) {
      this.buffer |= ((value >>> i) & 1) << this.count;
      if (++this.count === 8) this.flushByte();
    }
  }

  code(code: number, length: number): void {
    for (let i = length - 1; i >= 0; i--) {
      this.buffer |= ((code >>> i) & 1) << this.count;
      if (++this.count === 8) this.flushByte();
    }
  }

  finish(): number[] {
    if (this.count > 0) this.flushByte();
    return this.bytes;
  }

  private flushByte(): void {
    this.bytes.push(this.buffer);
    this.buffer = 0;
    this.count = 0;
  }
}

/** The fixed literal/length code from RFC 1951 section 3.2.6. */
function literal(out: BitWriter, symbol: number): void {
  if (symbol < 144) out.code(0x30 + symbol, 8);
  else if (symbol < 256) out.code(0x190 + symbol - 144, 9);
  else if (symbol < 280) out.code(symbol - 256, 7);
  else out.code(0xc0 + symbol - 280, 8);
}

function indexOfBase(bases: number[], value: number): number {
  let index = bases.length - 1;
  while (bases[index] > value) index--;
  return index;
}

function zlib(data: Uint8Array): Uint8Array {
  const out = new BitWriter();
  out.bytes.push(0x78, 0x9c);
  out.bits(1, 1); // the only block is the final one
  out.bits(1, 2); // fixed Huffman codes

  const head = new Int32Array(1 << 15).fill(-1);
  const previous = new Int32Array(data.length).fill(-1);
  const hash = (at: number) => ((data[at] << 10) ^ (data[at + 1] << 5) ^ data[at + 2]) & 0x7fff;
  const insert = (at: number) => {
    if (at + MIN_MATCH > data.length) return;
    const key = hash(at);
    previous[at] = head[key];
    head[key] = at;
  };

  let at = 0;
  while (at < data.length) {
    let bestLength = 0;
    let bestDistance = 0;
    if (at + MIN_MATCH <= data.length) {
      const limit = Math.min(MAX_MATCH, data.length - at);
      let candidate = head[hash(at)];
      for (let chain = 0; candidate >= 0 && chain < MAX_CHAIN; chain++) {
        const distance = at - candidate;
        if (distance > WINDOW) break;
        let length = 0;
        while (length < limit && data[candidate + length] === data[at + length]) length++;
        if (length > bestLength) {
          bestLength = length;
          bestDistance = distance;
          if (length === limit) break;
        }
        candidate = previous[candidate];
      }
    }

    if (bestLength >= MIN_MATCH) {
      const lengthCode = indexOfBase(LENGTH_BASE, bestLength);
      literal(out, 257 + lengthCode);
      out.bits(bestLength - LENGTH_BASE[lengthCode], LENGTH_EXTRA[lengthCode]);
      const distanceCode = indexOfBase(DISTANCE_BASE, bestDistance);
      out.code(distanceCode, 5);
      out.bits(bestDistance - DISTANCE_BASE[distanceCode], DISTANCE_EXTRA[distanceCode]);
      for (let i = 0; i < bestLength; i++) insert(at + i);
      at += bestLength;
    } else {
      literal(out, data[at]);
      insert(at);
      at++;
    }
  }
  literal(out, 256); // end of block

  const bytes = out.finish();
  const checksum = adler32(data);
  bytes.push(checksum >>> 24, (checksum >>> 16) & 0xff, (checksum >>> 8) & 0xff, checksum & 0xff);
  return Uint8Array.from(bytes);
}
