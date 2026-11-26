import { useCallback, useEffect, useState } from "react";
import { api, type AutoStatus } from "./api";

const initial: AutoStatus = { phase: "idle", steps: [], connected: null, systemProxy: false };

/** وضعیت اتصال از Rust؛ هنگام تست تندتر تازه می‌شود */
export function useAuto() {
  const [status, setStatus] = useState<AutoStatus>(initial);
  const refresh = useCallback(() => api.autoStatus().then(setStatus).catch(() => {}), []);
  const fast = status.phase === "testing";
  useEffect(() => {
    refresh();
    const t = window.setInterval(refresh, fast ? 700 : 2500);
    return () => window.clearInterval(t);
  }, [fast, refresh]);
  return { status, refresh };
}
