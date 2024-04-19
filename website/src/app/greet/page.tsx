'use client'

import init, { greet } from 'wasm';

export default function Greet() {

  async function greetFromWasm() {
    await init();
    greet();
  }

  return (
    <div className="flex min-h-screen flex-col items-center justify-between p-24">
      <div className="flex flex-row items-start">
        <div className="mr-3">This is the greeting page.</div>
        <button onClick={() => greetFromWasm()}>Greet</button>
      </div>
    </div>
  )
}