export async function testRpc(endpoint) {
    console.log(`Calling ${endpoint}`);
    const response = await fetch(endpoint);
    if (!response.ok) {
        console.log('Failed to call RPC endpoint');
    }
    // Assuming you might do something with the response, but no return is needed
}