import type { Step } from "@/types/step";

export function stepImageSrc(step: Step): string {
  return `data:image/jpeg;base64,${step.imageBase64}`;
}

export function stepTypeLabel(eventType: string): string {
  switch (eventType) {
    case "left-click":
      return "点击";
    case "right-click":
      return "右键";
    case "keyboard-input":
      return "输入";
    default:
      return "操作";
  }
}

function parseStepTimestamp(timestamp: string): Date {
  if (/^\d+$/.test(timestamp)) {
    return new Date(Number(timestamp));
  }
  if (/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(timestamp)) {
    return new Date(timestamp.replace(" ", "T"));
  }
  return new Date(timestamp);
}

export function formatStepTime(timestamp: string): string {
  const date = parseStepTimestamp(timestamp);
  if (Number.isNaN(date.getTime())) return timestamp;
  return date.toLocaleTimeString();
}

export function stepTimeDateTime(timestamp: string): string {
  const date = parseStepTimestamp(timestamp);
  if (Number.isNaN(date.getTime())) return timestamp;
  return date.toISOString();
}
