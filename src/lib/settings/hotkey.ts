export const DEFAULT_HOTKEY = "Ctrl+Alt+KeyR";

const STORAGE_KEY = "stepdoc.hotkey";

// 存储格式：修饰键（Ctrl / Alt / Shift / Super）+ W3C 按键码（KeyboardEvent.code），
// 例如 "Ctrl+Alt+KeyR"；该格式与 Rust 侧 global-hotkey 的解析规则直接对应。
export function loadHotkey(): string | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === null) return DEFAULT_HOTKEY;
    const parsed = JSON.parse(raw) as unknown;
    return typeof parsed === "string" && parsed ? parsed : null;
  } catch {
    return DEFAULT_HOTKEY;
  }
}

export function saveHotkey(hotkey: string | null): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(hotkey));
}

const MODIFIER_LABELS: Record<string, string> = {
  Ctrl: "Ctrl",
  Alt: "Alt",
  Shift: "Shift",
  Super: "Win",
};

const KEY_LABELS: Record<string, string> = {
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Space: "空格",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  PrintScreen: "PrtSc",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Home: "Home",
  End: "End",
  Insert: "Insert",
  Delete: "Delete",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  NumpadAdd: "Num+",
  NumpadSubtract: "Num-",
  NumpadMultiply: "Num*",
  NumpadDivide: "Num/",
  NumpadDecimal: "Num.",
  NumpadEnter: "NumEnter",
};

export function formatHotkey(hotkey: string): string {
  const tokens = hotkey
    .split("+")
    .map((token) => token.trim())
    .filter(Boolean);
  const key = tokens.pop();
  if (!key) return hotkey;

  const parts = tokens.map((token) => MODIFIER_LABELS[token] ?? token);
  parts.push(KEY_LABELS[key] ?? key.replace(/^Key|^Digit/, ""));
  return parts.join(" + ");
}

export function validateHotkey(hotkey: string): string | null {
  const tokens = hotkey
    .split("+")
    .map((token) => token.trim())
    .filter(Boolean);
  if (tokens.length === 0) {
    return "请按下包含修饰键（Ctrl / Alt / Shift / Win）的组合键，或使用 F1~F12 功能键";
  }

  const key = tokens[tokens.length - 1];
  const hasModifier = tokens.length > 1;
  const isFunctionKey = /^F\d{1,2}$/.test(key);
  if (!hasModifier && !isFunctionKey) {
    return "需要至少一个修饰键（Ctrl / Alt / Shift / Win），或使用 F1~F12 功能键";
  }
  return null;
}
