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

export async function forceImportStore(jsonStr) {
  try {
    if (!db.isOpen) {
      await openDatabase();
    }

    let dbJson = JSON.parse(jsonStr);
    if (typeof dbJson === "string") {
      dbJson = JSON.parse(dbJson);
    }

    const jsonTableNames = Object.keys(dbJson);
    const dbTableNames = db.tables.map((t) => t.name);

    if (jsonTableNames.length === 0) {
      throw new Error("No tables found in the provided JSON.");
    }

    // Wrap everything in a transaction
    await db.transaction(
      "rw",
      ...dbTableNames.map((name) => db.table(name)),
      async () => {
        // Clear all tables in the database
        await Promise.all(db.tables.map((t) => t.clear()));

        // Import data from JSON into matching tables
        for (const tableName of jsonTableNames) {
          const table = db.table(tableName);

          if (!dbTableNames.includes(tableName)) {
            console.warn(
              `Table "${tableName}" does not exist in the database schema. Skipping.`
            );
            continue; // Skip tables not in the Dexie schema
          }

          const records = dbJson[tableName];

          const transformedRecords = await Promise.all(
            records.map(recursivelyTransformForImport)
          );

          await table.bulkPut(transformedRecords);
        }
      }
    );

    console.log("Store imported successfully.");
  } catch (err) {
    console.error("Failed to import store: ", err.toString());
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
