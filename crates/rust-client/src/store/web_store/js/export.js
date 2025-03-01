import { db } from "./schema.js";

async function recursivelyTransformForExport(obj) {
  if (obj instanceof Blob) {
    const blobBuffer = await obj.arrayBuffer();
    return {
      __type: "Blob",
      data: uint8ArrayToBase64(new Uint8Array(blobBuffer)),
    };
  }

  if (Array.isArray(obj)) {
    return await Promise.all(obj.map(recursivelyTransformForExport));
  }

  if (obj && typeof obj === "object") {
    const entries = await Promise.all(
      Object.entries(obj).map(async ([key, value]) => [
        key,
        await recursivelyTransformForExport(value),
      ])
    );
    return Object.fromEntries(entries);
  }

  return obj;
}

export async function exportStore() {
  const dbJson = {};
  for (const table of db.tables) {
    const records = await table.toArray();

    dbJson[table.name] = await Promise.all(
      records.map(recursivelyTransformForExport)
    );
  }

  const stringified = JSON.stringify(dbJson);
  return stringified;
}

function uint8ArrayToBase64(uint8Array) {
  return btoa(String.fromCharCode(...uint8Array));
}
