export async function testRpc(endpoint) {
    try {
        console.log(`Calling ${endpoint}`);
        await fetch(endpoint);
    } catch (error) {
        console.log('Failed to call RPC endpoint');
    }
}