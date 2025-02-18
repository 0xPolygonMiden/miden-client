import { db, openDatabase } from "./schema.js";

async function recursivelyTransformForImport(obj) {
  if (obj && typeof obj === "object") {
    if (obj.__type === "Blob") {
      return new Blob([base64ToUint8Array(obj.data)]);
    }

    if (Array.isArray(obj)) {
      return await Promise.all(obj.map(recursivelyTransformForImport));
    }

    const entries = await Promise.all(
      Object.entries(obj).map(async ([key, value]) => [
        key,
        await recursivelyTransformForImport(value),
      ])
    );
    return Object.fromEntries(entries);
  }

  return obj; // Return unchanged if it's neither Blob, Array, nor Object
}

export async function importStore(jsonStr) {
  try {
    if (!db.isOpen) {
      await openDatabase();
    }

    let db_json = JSON.parse(jsonStr);
    if (typeof db_json === "string") {
      db_json = JSON.parse(db_json);
    }

    db.tables.forEach((t) => console.log(t.name));

    for (const tableName in db_json) {
      const table = db[tableName];
      const records = db_json[tableName];

      const transformedRecords = await Promise.all(
        records.map(recursivelyTransformForImport)
      );

      await table.bulkPut(transformedRecords);
    }
  } catch (err) {
    console.error("Failed to import store: ", err);
    throw err;
  }
}

function base64ToUint8Array(base64) {
  const binaryString = atob(base64);
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}
