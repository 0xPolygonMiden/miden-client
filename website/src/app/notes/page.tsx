
'use client'
import { useWasm } from '@/components/hooks/useWasm';
import init, * as wasm from '@wasm';

init();

export default function Notes() {
  const wasm = useWasm();
  wasm?.greet2()
  
  return (
    <div>This is the notes page.</div>
  )
}

class MyJSClass {
  doSomething() {
    console.log("This is a JS class method");
  }
}