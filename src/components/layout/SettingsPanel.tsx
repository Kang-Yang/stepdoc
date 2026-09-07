import { IconClose } from "@/components/icons";
import { AboutSection } from "@/components/layout/AboutSection";
import { HotkeyField } from "@/components/layout/HotkeyField";
import { openOcrDebugFolder } from "@/lib/tauri/commands";
import type { RecordingOptions } from "@/types/recording";

type SettingsPanelProps = {
  open: boolean;
  recording: boolean;
  recordingOptions: RecordingOptions;
  hotkey: string | null;
  hotkeyError: string | null;
  onClose: () => void;
  onRecordingOptionsChange: (options: RecordingOptions) => void;
  onHotkeyCommit: (hotkey: string) => Promise<void>;
  onHotkeyClear: () => Promise<void>;
};

export function SettingsPanel({
  open,
  recording,
  recordingOptions,
  hotkey,
  hotkeyError,
  onClose,
  onRecordingOptionsChange,
  onHotkeyCommit,
  onHotkeyClear,
}: SettingsPanelProps) {
  if (!open) return null;

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div
        className="settings-panel"
        role="dialog"
        aria-labelledby="settings-title"
        aria-modal="true"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="settings-panel__header">
          <h2 id="settings-title" className="settings-panel__title">
            设置
          </h2>
          <button type="button" className="btn icon-btn" onClick={onClose} aria-label="关闭设置">
            <IconClose />
          </button>
        </header>

        <div className="settings-panel__body">
          <section className="settings-section">
            <h3 className="settings-section__title">快捷键</h3>
            <HotkeyField
              hotkey={hotkey}
              registrationError={hotkeyError}
              onCommit={onHotkeyCommit}
              onClear={onHotkeyClear}
            />
          </section>

          <section className="settings-section">
            <h3 className="settings-section__title">OCR 调试</h3>
            <label className="settings-option">
              <input
                type="checkbox"
                checked={recordingOptions.ocrDebug}
                disabled={recording}
                onChange={(event) =>
                  onRecordingOptionsChange({
                    ...recordingOptions,
                    ocrDebug: event.target.checked,
                  })
                }
              />
              <span className="settings-option__text">
                <strong>保存 OCR 调试明细</strong>
                <small>录制点击时，把裁剪图、预处理图和识别候选词写入临时目录</small>
              </span>
            </label>
            <button
              type="button"
              className="btn block settings-open-debug"
              onClick={() => void openOcrDebugFolder().catch((error) => alert(String(error)))}
            >
              打开 OCR 调试目录
            </button>
          </section>

          <section className="settings-section">
            <h3 className="settings-section__title">关于</h3>
            <AboutSection />
          </section>
        </div>
      </div>
    </div>
  );
}
