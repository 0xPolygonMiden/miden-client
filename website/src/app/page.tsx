'use client'
import { useEffect } from "react";
import { db } from "../lib/db";

export default function Home() {
  useEffect(() => {
    db.open()
  }, [])

  return (
    <div className="flex min-h-screen flex-col items-center justify-between p-24">
      <p>This website will serve as a testing site and demo of the Miden WASM</p>
    </div>
  );
}
