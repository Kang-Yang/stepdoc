import { useCallback, useEffect, useState } from "react";

import { useAsyncAction } from "@/hooks/useAsyncAction";
import { formatElapsed } from "@/lib/format/elapsed";
import {
  getRecordingStatus,
  pauseRecording,
  resumeRecording,
  stopRecording,
} from "@/lib/tauri/commands";
import type { RecordingStatus } from "@/types/recording";

import "@/styles/recording-bar.css";

function IconPause() {
  return (
    <svg width="15" height="15" viewBox="0 0 15 15" fill="none" aria-hidden>
      <rect x="3.5" y="2.5" width="2.8" height="10" rx="0.9" fill="currentColor" />
      <rect x="8.7" y="2.5" width="2.8" height="10" rx="0.9" fill="currentColor" />
    </svg>
  );
}

function IconPlay() {
  return (
    <svg width="15" height="15" viewBox="0 0 15 15" fill="none" aria-hidden>
      <path d="M4.8 2.6 12.2 7.5 4.8 12.4V2.6Z" fill="currentColor" />
    </svg>
  );
}

function IconStop() {
  return (
    <svg width="15" height="15" viewBox="0 0 15 15" fill="none" aria-hidden>
      <rect x="3.8" y="3.8" width="7.4" height="7.4" rx="1.4" fill="currentColor" />
    </svg>
  );
}

export default function RecordingBar() {
  const [status, setStatus] = useState<RecordingStatus>({
    recording: true,
    paused: false,
    elapsedMs: 0,
    stepCount: 0,
  });
  const { busy, error, run } = useAsyncAction();

  useEffect(() => {
    document.documentElement.classList.add("recording-bar-mode");

    let active = true;
    const refresh = async () => {
      try {
        const next = await getRecordingStatus();
        if (!active) return;
        setStatus(next);
      } catch (cause) {
        if (active) {
          console.error(cause);
        }
      }
    };

    refresh();
    const timer = window.setInterval(refresh, 200);
    return () => {
      active = false;
      window.clearInterval(timer);
      document.documentElement.classList.remove("recording-bar-mode");
    };
  }, []);

  const togglePause = useCallback(
    () =>
      run(async () => {
        if (status.paused) {
          await resumeRecording();
        } else {
          await pauseRecording();
        }
      }),
    [run, status.paused],
  );

  const stop = useCallback(
    () =>
      run(async () => {
        await stopRecording();
      }),
    [run],
  );

  return (
    <div className="recording-bar-shell">
      <div className={`recording-bar ${status.paused ? "is-paused" : ""}`}>
        <div className="recording-bar__drag" data-tauri-drag-region>
          <div className="recording-bar__indicator" data-tauri-drag-region>
            <span className={`recording-bar__dot ${status.paused ? "is-paused" : ""}`} aria-hidden />
          </div>
          <div className="recording-bar__info" data-tauri-drag-region>
            <strong className="recording-bar__time">{formatElapsed(status.elapsedMs)}</strong>
            <span className="recording-bar__meta">
              <strong>{status.paused ? "已暂停" : "REC"}</strong>
              {" · "}
              {status.stepCount} 步
            </span>
          </div>
        </div>

        <div className="recording-bar__divider" aria-hidden />

        <div className="recording-bar__actions">
          <button
            type="button"
            className={`recording-bar__btn recording-bar__btn--pause ${status.paused ? "is-active" : ""}`}
            onClick={togglePause}
            disabled={busy}
            title={status.paused ? "继续录制" : "暂停录制"}
            aria-label={status.paused ? "继续录制" : "暂停录制"}
          >
            {status.paused ? <IconPlay /> : <IconPause />}
          </button>
          <button
            type="button"
            className="recording-bar__btn recording-bar__btn--stop"
            onClick={stop}
            disabled={busy}
            title="结束录制"
            aria-label="结束录制"
          >
            <IconStop />
            {busy ? "处理中…" : "结束"}
          </button>
        </div>
      </div>

      {error ? <div className="recording-bar__error">{error}</div> : null}
    </div>
  );
}
