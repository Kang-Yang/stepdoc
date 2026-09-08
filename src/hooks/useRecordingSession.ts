import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { getRecordingStatus } from "@/lib/tauri/commands";
import { TAURI_EVENTS } from "@/lib/tauri/events";
import type { OcrProgress } from "@/types/recording";
import type { Step, StepUpdate } from "@/types/step";

const EMPTY_OCR_PROGRESS: OcrProgress = {
  completed: 0,
  total: 0,
  activeStep: null,
};

async function registerListener<T>(
  active: () => boolean,
  unlisteners: Array<() => void>,
  event: string,
  handler: (payload: T) => void,
) {
  const unlisten = await listen<T>(event, (event) => handler(event.payload));
  if (!active()) {
    unlisten();
    return;
  }
  unlisteners.push(unlisten);
}

export function useRecordingSession() {
  const [recording, setRecording] = useState(false);
  const [recordingPaused, setRecordingPaused] = useState(false);
  const [finalizing, setFinalizing] = useState(false);
  const [ocrProgress, setOcrProgress] = useState<OcrProgress>(EMPTY_OCR_PROGRESS);
  const [elapsedMs, setElapsedMs] = useState(0);
  const [steps, setSteps] = useState<Step[]>([]);

  useEffect(() => {
    let active = true;
    const unlisteners: Array<() => void> = [];
    const isActive = () => active;

    void (async () => {
      await registerListener<StepUpdate>(
        isActive,
        unlisteners,
        TAURI_EVENTS.recordingStepUpdated,
        (payload) => {
          setSteps((prev) =>
            prev.map((step) =>
              step.id === payload.id ? { ...step, description: payload.description } : step,
            ),
          );
        },
      );

      await registerListener<Step>(isActive, unlisteners, TAURI_EVENTS.recordingStep, (payload) => {
        setSteps((prev) => [...prev, payload]);
      });

      await registerListener<OcrProgress>(
        isActive,
        unlisteners,
        TAURI_EVENTS.ocrProgress,
        setOcrProgress,
      );

      await registerListener(isActive, unlisteners, TAURI_EVENTS.recordingStarted, () => {
        setRecording(true);
        setRecordingPaused(false);
        setFinalizing(false);
        setOcrProgress(EMPTY_OCR_PROGRESS);
        setSteps([]);
        setElapsedMs(0);
      });

      await registerListener(isActive, unlisteners, TAURI_EVENTS.recordingPaused, () => {
        setRecordingPaused(true);
      });

      await registerListener(isActive, unlisteners, TAURI_EVENTS.recordingResumed, () => {
        setRecordingPaused(false);
      });

      await registerListener<OcrProgress>(
        isActive,
        unlisteners,
        TAURI_EVENTS.recordingFinalizing,
        (payload) => {
          setRecording(false);
          setRecordingPaused(false);
          setFinalizing(true);
          setOcrProgress(payload);
          setElapsedMs(0);
        },
      );

      await registerListener<Step[]>(isActive, unlisteners, TAURI_EVENTS.recordingStopped, (payload) => {
        setSteps(payload);
        setRecording(false);
        setRecordingPaused(false);
        setFinalizing(false);
        setOcrProgress(EMPTY_OCR_PROGRESS);
        setElapsedMs(0);
      });
    })();

    return () => {
      active = false;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!recording) return;

    let active = true;
    const refresh = async () => {
      try {
        const status = await getRecordingStatus();
        if (!active) return;
        setElapsedMs(status.elapsedMs);
        setRecordingPaused(status.paused);
      } catch (cause) {
        // 只有窗口最小化时才静默忽略（此时无人在看界面）；其余失败如实记录，
        // 避免掩盖真正的后端异常。
        if (!(await getCurrentWindow().isMinimized())) {
          console.error("获取录制状态失败:", cause);
        }
      }
    };

    refresh();
    const timer = window.setInterval(refresh, 500);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [recording]);

  return {
    recording,
    recordingPaused,
    finalizing,
    ocrProgress,
    elapsedMs,
    steps,
    setSteps,
    setRecording,
    setRecordingPaused,
    setElapsedMs,
  };
}
