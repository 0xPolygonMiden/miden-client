'use client'

import { useEffect } from 'react';
import init, * as wasm from 'wasm';

export default function Notes() {
  useEffect(() => {
    async function greet2() {
      await init();
      wasm.greet2();
    }
    greet2();
  })
  
  return (
    <div className="flex min-h-screen flex-col items-center justify-between p-24">This is the notes page</div>
  )
}