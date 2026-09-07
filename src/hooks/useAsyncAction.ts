import { useCallback, useState } from "react";

export function useAsyncAction() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async (scope: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await scope();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  }, []);

  return { busy, error, run, setError, setBusy };
}
