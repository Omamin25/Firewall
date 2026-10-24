import { useCallback, useEffect } from "react";
import {
  listCaptureInterfaces,
  listFlows,
  startCapture,
  stopCapture,
  subscribePacketStream,
} from "@/lib/ipc";
import { useFlowStore } from "@/store/use-flow-store";
import { useTrafficStore } from "@/store/use-traffic-store";

/** Owns the capture lifecycle: registers the stream channel once, enumerates adapters,
 *  and exposes start/stop. Kept in one hook so views stay pure presentation. */
export function useCaptureController(): {
  start: () => Promise<void>;
  stop: () => Promise<void>;
  refreshInterfaces: () => Promise<void>;
} {
  const apply = useTrafficStore((state) => state.apply);
  const applyFlows = useFlowStore((state) => state.apply);
  const subscribed = useTrafficStore((state) => state.subscribed);
  const markSubscribed = useTrafficStore((state) => state.markSubscribed);
  const setCaptureState = useTrafficStore((state) => state.setCaptureState);
  const setError = useTrafficStore((state) => state.setError);
  const setInterfaces = useTrafficStore((state) => state.setInterfaces);
  const seeded = useFlowStore((state) => state.seeded);
  const markSeeded = useFlowStore((state) => state.markSeeded);
  const replaceAll = useFlowStore((state) => state.replaceAll);
  const setFlowError = useFlowStore((state) => state.setError);

  // One channel per app session; batches are fanned out from Rust. Each store keeps the
  // message variants it owns and ignores the rest.
  useEffect(() => {
    if (subscribed) return;
    markSubscribed();
    subscribePacketStream((message) => {
      apply(message);
      applyFlows(message);
    }).catch((error: unknown) => {
      setError(errorMessage(error));
      setCaptureState("error");
    });
  }, [apply, applyFlows, markSubscribed, setCaptureState, setError, subscribed]);

  // The stream only carries diffs, so the live table is seeded from a snapshot once per
  // session — whichever view mounts first.
  useEffect(() => {
    if (seeded) return;
    markSeeded();
    listFlows()
      .then(replaceAll)
      .catch((error: unknown) => setFlowError(errorMessage(error)));
  }, [seeded, markSeeded, replaceAll, setFlowError]);

  const refreshInterfaces = useCallback(async () => {
    try {
      setInterfaces(await listCaptureInterfaces());
    } catch (error: unknown) {
      setError(errorMessage(error));
      setCaptureState("error");
    }
  }, [setCaptureState, setError, setInterfaces]);

  const start = useCallback(async () => {
    setError(null);
    setCaptureState("starting");
    try {
      const status = await startCapture();
      setInterfaces(status.interfaces);
      setCaptureState(status.running ? "running" : "idle");
      if (status.error) setError(status.error);
    } catch (error: unknown) {
      setError(errorMessage(error));
      setCaptureState("error");
    }
  }, [setCaptureState, setError, setInterfaces]);

  const stop = useCallback(async () => {
    try {
      await stopCapture();
      setCaptureState("idle");
    } catch (error: unknown) {
      setError(errorMessage(error));
      setCaptureState("error");
    }
  }, [setCaptureState, setError]);

  return { start, stop, refreshInterfaces };
}

/** Tauri command rejections arrive as plain strings; anything else is unexpected. */
function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "An unexpected error occurred while talking to the capture engine.";
}
