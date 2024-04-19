import { useEffect, useState } from "react";

export const useWasm = () => {
    const [wasm, setWasm] = useState<any>(null);

    useEffect(() => {
        if (wasm === null) {
            import("wasm").then((module) => {
              module.default()
              return setWasm(module)
            });
        }
    }, []); // eslint-disable-line react-hooks/exhaustive-deps
    return wasm;
};