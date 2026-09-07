import { useEffect, useState } from "react";

import { DEFAULT_HOTKEY, formatHotkey, validateHotkey } from "@/lib/settings/hotkey";

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
]);

type HotkeyFieldProps = {
  hotkey: string | null;
  /** 注册并保存新快捷键；注册失败（如被其他程序占用）时抛出错误。 */
  onCommit: (hotkey: string) => Promise<void>;
  /** 注销并清除快捷键。 */
  onClear: () => Promise<void>;
};

export function HotkeyField({ hotkey, onCommit, onClear }: HotkeyFieldProps) {
  const [capturing, setCapturing] = useState(false);
  const [partialModifiers, setPartialModifiers] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!capturing) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.code === "Escape") {
        stopCapturing();
        return;
      }

      const modifiers: string[] = [];
      if (event.ctrlKey) modifiers.push("Ctrl");
      if (event.altKey) modifiers.push("Alt");
      if (event.shiftKey) modifiers.push("Shift");
      if (event.metaKey) modifiers.push("Super");

      if (MODIFIER_CODES.has(event.code)) {
        setPartialModifiers(modifiers);
        return;
      }

      const candidate = [...modifiers, event.code].join("+");
      const invalid = validateHotkey(candidate);
      if (invalid) {
        setError(invalid);
        setPartialModifiers(modifiers);
        return;
      }

      setError(null);
      void onCommit(candidate).catch((cause) => setError(String(cause)));
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [capturing, onCommit]);

  const stopCapturing = () => {
    setCapturing(false);
    setPartialModifiers([]);
    setError(null);
  };

  const startCapturing = () => {
    setCapturing(true);
    setPartialModifiers([]);
    setError(null);
  };

  const clear = () => {
    stopCapturing();
    void onClear().catch((cause) => setError(String(cause)));
  };

  const restoreDefault = () => {
    stopCapturing();
    void onCommit(DEFAULT_HOTKEY).catch((cause) => setError(String(cause)));
  };

  return (
    <div className={`settings-hotkey ${capturing ? "is-capturing" : ""}`}>
      <div className="settings-hotkey__head">
        <span className="settings-hotkey__name">开始 / 结束录制</span>
        {capturing ? (
          <span className="settings-hotkey__capture">
            {partialModifiers.length > 0
              ? `${partialModifiers.join(" + ")} + …`
              : "请按下新的快捷键，Esc 取消"}
          </span>
        ) : (
          <span className={`settings-hotkey__value ${hotkey ? "" : "is-empty"}`}>
            {hotkey ? formatHotkey(hotkey) : "未设置"}
          </span>
        )}
      </div>
      <div className="settings-hotkey__foot">
        <span className="settings-hotkey__hint">在任意应用中按下即可开始或结束录制</span>
        <div className="settings-hotkey__actions">
          {capturing ? (
            <button type="button" className="btn btn--sm" onClick={stopCapturing}>
              取消
            </button>
          ) : (
            <button type="button" className="btn btn--sm" onClick={startCapturing}>
              修改
            </button>
          )}
          {hotkey ? (
            <button type="button" className="btn btn--sm" onClick={clear}>
              清除
            </button>
          ) : (
            <button type="button" className="btn btn--sm" onClick={restoreDefault}>
              恢复默认
            </button>
          )}
        </div>
      </div>
      {error ? <p className="settings-hotkey__error">{error}</p> : null}
    </div>
  );
}
