import { greets } from './schema.js';

export async function insertGreeting(greeting) {
    try {
        // Prepare the data object to insert
        const data = {
            greeting: greeting
        };

        // Perform the insert using Dexie
        await greets.add(data);
        return `Successfully inserted greeting: ${greeting}`;
    } catch (error) {
        console.error(`Error inserting greeting: ${greeting}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}