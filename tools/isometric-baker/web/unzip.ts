/**
 * Reading a zip archive, so a model that comes zipped (a Sketchfab download is a
 * `.gltf`, its `.bin` and a `textures/` folder) can be opened as it arrived.
 *
 * No dependency: an archive is a table of entries at its end, and each entry's
 * bytes are stored or deflated, which the browser's own `DecompressionStream`
 * undoes. Only what a model download needs is read: stored and deflated entries,
 * no encryption, no ZIP64. Everything else is refused with a message, not
 * guessed at, and sizes are bounded so an archive cannot be a bomb.
 */

import { crc32 } from './png.ts';

export interface UnzippedEntry {
  /** The entry's path inside the archive, with forward slashes. */
  path: string;
  contents: Uint8Array;
}

/** The most one entry, and one archive, may expand to. */
const MAX_ENTRY = 256 * 1024 * 1024;
const MAX_TOTAL = 1024 * 1024 * 1024;

async function inflate(data: Uint8Array, size: number, path: string): Promise<Uint8Array> {
  const stream = new Blob([data as BlobPart]).stream().pipeThrough(new DecompressionStream('deflate-raw'));
  const reader = stream.getReader();
  const out = new Uint8Array(size);
  let filled = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    if (filled + value.length > size) {
      await reader.cancel();
      throw new Error(`${path} is larger than the archive says it is`);
    }
    out.set(value, filled);
    filled += value.length;
  }
  if (filled !== size) throw new Error(`${path} is smaller than the archive says it is`);
  return out;
}

/** Every file in a zip archive; folders are left out. */
export async function unzip(bytes: Uint8Array): Promise<UnzippedEntry[]> {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

  let end = -1;
  for (let at = bytes.length - 22; at >= Math.max(0, bytes.length - 22 - 0xffff); at--) {
    if (view.getUint32(at, true) === 0x06054b50) {
      end = at;
      break;
    }
  }
  if (end < 0) throw new Error('not a zip archive');

  const count = view.getUint16(end + 10, true);
  const directory = view.getUint32(end + 16, true);
  if (count === 0xffff || directory === 0xffffffff) throw new Error('ZIP64 archives are not supported');

  const entries: UnzippedEntry[] = [];
  let total = 0;
  let at = directory;
  for (let index = 0; index < count; index++) {
    if (at + 46 > bytes.length || view.getUint32(at, true) !== 0x02014b50) throw new Error('the zip archive is damaged');
    const flags = view.getUint16(at + 8, true);
    const method = view.getUint16(at + 10, true);
    const crc = view.getUint32(at + 16, true);
    const compressed = view.getUint32(at + 20, true);
    const size = view.getUint32(at + 24, true);
    const nameLength = view.getUint16(at + 28, true);
    const extraLength = view.getUint16(at + 30, true);
    const commentLength = view.getUint16(at + 32, true);
    const local = view.getUint32(at + 42, true);
    const path = new TextDecoder().decode(bytes.subarray(at + 46, at + 46 + nameLength)).replaceAll('\\', '/');
    at += 46 + nameLength + extraLength + commentLength;

    if (path.endsWith('/')) continue;
    if (flags & 1) throw new Error(`${path} is encrypted, which is not supported`);
    if (method !== 0 && method !== 8) throw new Error(`${path} uses a compression method this reader does not have`);
    if (size > MAX_ENTRY || (total += size) > MAX_TOTAL) throw new Error('the zip archive expands to more than this page will open');
    if (local + 30 > bytes.length || view.getUint32(local, true) !== 0x04034b50) throw new Error('the zip archive is damaged');

    const start = local + 30 + view.getUint16(local + 26, true) + view.getUint16(local + 28, true);
    if (start + compressed > bytes.length) throw new Error('the zip archive is damaged');
    const stored = bytes.subarray(start, start + compressed);
    const contents = method === 0 ? stored.slice() : await inflate(stored, size, path);
    if (contents.length !== size || crc32(contents) !== crc) throw new Error(`${path} does not match its checksum: the zip archive is damaged`);
    entries.push({ path, contents });
  }
  return entries;
}
