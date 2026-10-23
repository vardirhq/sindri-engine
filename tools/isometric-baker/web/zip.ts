/**
 * A zip archive of a bake's files, so a page can hand over the whole asset
 * folder in one download.
 *
 * Entries are stored rather than compressed: the PNG is already deflated and the
 * two JSON documents are small. The timestamp is fixed (1 January 1980, the
 * zip epoch) so the same bake is the same archive.
 */

import { crc32 } from './png.ts';

export interface ZipEntry {
  /** Path inside the archive, with forward slashes and no leading slash. */
  path: string;
  contents: Uint8Array;
}

const DOS_DATE = (0 << 9) | (1 << 5) | 1;

export function zip(entries: ZipEntry[]): Uint8Array {
  const names = entries.map((entry) => new TextEncoder().encode(entry.path));
  const centralSize = entries.reduce((total, _, index) => total + 46 + names[index].length, 0);
  const localSize = entries.reduce(
    (total, entry, index) => total + 30 + names[index].length + entry.contents.length,
    0,
  );
  const out = new Uint8Array(localSize + centralSize + 22);
  const view = new DataView(out.buffer);

  let at = 0;
  const offsets: number[] = [];
  entries.forEach((entry, index) => {
    offsets.push(at);
    view.setUint32(at, 0x04034b50, true);
    view.setUint16(at + 4, 20, true); // version needed
    view.setUint16(at + 6, 0x0800, true); // names are UTF-8
    view.setUint16(at + 8, 0, true); // stored
    view.setUint16(at + 10, 0, true); // time
    view.setUint16(at + 12, DOS_DATE, true);
    view.setUint32(at + 14, crc32(entry.contents), true);
    view.setUint32(at + 18, entry.contents.length, true);
    view.setUint32(at + 22, entry.contents.length, true);
    view.setUint16(at + 26, names[index].length, true);
    view.setUint16(at + 28, 0, true); // no extra field
    out.set(names[index], at + 30);
    out.set(entry.contents, at + 30 + names[index].length);
    at += 30 + names[index].length + entry.contents.length;
  });

  const centralStart = at;
  entries.forEach((entry, index) => {
    view.setUint32(at, 0x02014b50, true);
    view.setUint16(at + 4, 20, true); // made by
    view.setUint16(at + 6, 20, true); // version needed
    view.setUint16(at + 8, 0x0800, true);
    view.setUint16(at + 10, 0, true);
    view.setUint16(at + 12, 0, true);
    view.setUint16(at + 14, DOS_DATE, true);
    view.setUint32(at + 16, crc32(entry.contents), true);
    view.setUint32(at + 20, entry.contents.length, true);
    view.setUint32(at + 24, entry.contents.length, true);
    view.setUint16(at + 28, names[index].length, true);
    // Extra field, comment, disk number, internal and external attributes: zero.
    view.setUint32(at + 42, offsets[index], true);
    out.set(names[index], at + 46);
    at += 46 + names[index].length;
  });

  view.setUint32(at, 0x06054b50, true);
  view.setUint16(at + 8, entries.length, true);
  view.setUint16(at + 10, entries.length, true);
  view.setUint32(at + 12, centralSize, true);
  view.setUint32(at + 16, centralStart, true);
  return out;
}
