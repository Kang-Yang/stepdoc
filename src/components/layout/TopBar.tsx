import { formatElapsed } from "@/lib/format/elapsed";

type TopBarProps = {
  recording: boolean;
  recordingPaused: boolean;
  elapsedMs: number;
};

export function TopBar({ recording, recordingPaused, elapsedMs }: TopBarProps) {
  return (
    <header className="topbar">
      <div className="brand">
        <img src="/assets/logo.svg" alt="StepDoc" className="brand-logo" height={32} />
      </div>
      <div className={`status ${recording ? "is-recording" : ""} ${recordingPaused ? "is-paused" : ""}`}>
        <span className="dot" />
        {recording
          ? recordingPaused
            ? `已暂停 ${formatElapsed(elapsedMs)}`
            : `录制中 ${formatElapsed(elapsedMs)}`
          : "未录制"}
      </div>
    </header>
  );
}
