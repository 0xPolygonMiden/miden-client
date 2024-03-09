export async function testRpc(endpoint) {
    console.log(`Calling ${endpoint}`);
    const response = await fetch(endpoint);
    if (!response.ok) {
        throw new Error(`HTTP error! Status: ${response.status}`);
    }
    // Assuming you might do something with the response, but no return is needed
}