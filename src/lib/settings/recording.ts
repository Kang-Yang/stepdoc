import type { RecordingOptions } from "@/types/recording";

const STORAGE_KEY = "stepdoc.recording-options";

const DEFAULT_OPTIONS: RecordingOptions = {
  excludeRecordingBar: true,
  ocrDebug: false,
};

export function loadRecordingOptions(): RecordingOptions {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_OPTIONS;
    const parsed = JSON.parse(raw) as Partial<RecordingOptions>;
    return {
      excludeRecordingBar: parsed.excludeRecordingBar ?? DEFAULT_OPTIONS.excludeRecordingBar,
      ocrDebug: parsed.ocrDebug ?? DEFAULT_OPTIONS.ocrDebug,
    };
  } catch {
    return DEFAULT_OPTIONS;
  }
}

export function saveRecordingOptions(options: RecordingOptions): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(options));
}
