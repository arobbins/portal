import { readFile } from "node:fs/promises";
import { inflateRawSync } from "node:zlib";

const END_OF_DIRECTORY = 0x06054b50;
const DIRECTORY_ENTRY = 0x02014b50;
const STORED = 0;
const DEFLATED = 8;

function endOfDirectory(zip: Buffer): number {
  for (let at = zip.length - 22; at >= 0; at--) {
    if (zip.readUInt32LE(at) === END_OF_DIRECTORY) return at;
  }
  throw new Error("not a ZIP file");
}

function entryData(zip: Buffer, method: number, localHeader: number, size: number): Buffer {
  const nameLength = zip.readUInt16LE(localHeader + 26);
  const extraLength = zip.readUInt16LE(localHeader + 28);
  const start = localHeader + 30 + nameLength + extraLength;
  const raw = zip.subarray(start, start + size);
  if (method === STORED) return raw;
  if (method === DEFLATED) return inflateRawSync(raw);
  throw new Error(`unsupported ZIP compression method ${method}`);
}

export async function readZip(path: string): Promise<Map<string, string>> {
  const zip = await readFile(path);
  const end = endOfDirectory(zip);
  const count = zip.readUInt16LE(end + 10);
  let at = zip.readUInt32LE(end + 16);
  const files = new Map<string, string>();
  for (let index = 0; index < count; index++) {
    if (zip.readUInt32LE(at) !== DIRECTORY_ENTRY) throw new Error("corrupt ZIP directory");
    const method = zip.readUInt16LE(at + 10);
    const size = zip.readUInt32LE(at + 20);
    const nameLength = zip.readUInt16LE(at + 28);
    const extraLength = zip.readUInt16LE(at + 30);
    const commentLength = zip.readUInt16LE(at + 32);
    const localHeader = zip.readUInt32LE(at + 42);
    const name = zip.toString("utf8", at + 46, at + 46 + nameLength);
    files.set(name, entryData(zip, method, localHeader, size).toString("utf8"));
    at += 46 + nameLength + extraLength + commentLength;
  }
  return files;
}
