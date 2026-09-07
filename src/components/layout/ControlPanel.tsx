import { useState } from "react";

import { IconExport, IconRecord, IconSettings, IconStop, IconTrash } from "@/components/icons";
import { SettingsPanel } from "@/components/layout/SettingsPanel";
import { formatHotkey } from "@/lib/settings/hotkey";
import type { RecordingOptions } from "@/types/recording";

type ControlPanelProps = {
  recording: boolean;
  busy: boolean;
  savingGif: boolean;
  canExportMedia: boolean;
  hasSteps: boolean;
  stepCount: number;
  screenshotCount: number;
  recordingOptions: RecordingOptions;
  hotkey: string | null;
  onStart: () => void;
  onStop: () => void;
  onExportWord: () => void;
  onExportGif: () => void;
  onClear: () => void;
  onRecordingOptionsChange: (options: RecordingOptions) => void;
  onHotkeyCommit: (hotkey: string) => Promise<void>;
  onHotkeyClear: () => Promise<void>;
};

export function ControlPanel({
  recording,
  busy,
  savingGif,
  canExportMedia,
  hasSteps,
  stepCount,
  screenshotCount,
  recordingOptions,
  hotkey,
  onStart,
  onStop,
  onExportWord,
  onExportGif,
  onClear,
  onRecordingOptionsChange,
  onHotkeyCommit,
  onHotkeyClear,
}: ControlPanelProps) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const hotkeyHint = hotkey ? `快捷键：${formatHotkey(hotkey)}` : "可在设置中开启快捷键";

  return (
    <>
      <aside className="control-panel">
        <div className="control-section">
          <span className="control-label">录制</span>
          <div className="control-buttons">
            <button
              className="btn primary block"
              onClick={onStart}
              disabled={recording || busy}
              title={hotkeyHint}
            >
              <IconRecord />
              {recording ? "录制中…" : "开始录制"}
            </button>
            <button
              className="btn danger block"
              onClick={onStop}
              disabled={!recording || busy}
              title={hotkeyHint}
            >
              <IconStop />
              停止录制
            </button>
          </div>
        </div>

        <div className="control-divider" aria-hidden />

        <div className="control-section">
          <span className="control-label">导出</span>
          <div className="control-buttons">
            <button className="btn block" onClick={onExportGif} disabled={!canExportMedia || busy}>
              <IconExport />
              {savingGif ? "生成中…" : "GIF 动图"}
            </button>
            <button className="btn block" onClick={onExportWord} disabled={!hasSteps || busy}>
              <IconExport />
              Word 文档
            </button>
            <button className="btn ghost block" onClick={onClear} disabled={!hasSteps || recording}>
              <IconTrash />
              清空
            </button>
          </div>
        </div>

        <div className="control-footer">
          <div className="control-stats">
            <span>{stepCount} 步</span>
            <span className="control-stats__sep" aria-hidden>
              ·
            </span>
            <span>{screenshotCount} 张截图</span>
          </div>
          <button
            type="button"
            className="btn icon-btn"
            onClick={() => setSettingsOpen(true)}
            title="设置"
            aria-label="打开设置"
          >
            <IconSettings />
          </button>
        </div>
      </aside>

      <SettingsPanel
        open={settingsOpen}
        recording={recording}
        recordingOptions={recordingOptions}
        hotkey={hotkey}
        onClose={() => setSettingsOpen(false)}
        onRecordingOptionsChange={onRecordingOptionsChange}
        onHotkeyCommit={onHotkeyCommit}
        onHotkeyClear={onHotkeyClear}
      />
    </>
  );
}
