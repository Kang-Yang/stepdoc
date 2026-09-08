import { invoke } from "@tauri-apps/api/core";

import type { RecordingStatus } from "@/types/recording";
import type { RecordingOptions } from "@/types/recording";
import type { Step } from "@/types/step";

export type ExportCommand = "export_word" | "export_gif";

export const DEFAULT_TUTORIAL_TITLE = "操作教程";

export async function startRecording(options?: RecordingOptions): Promise<void> {
  await invoke("start_recording", { options });
}

export async function stopRecording(): Promise<Step[]> {
  return invoke<Step[]>("stop_recording");
}

export async function pauseRecording(): Promise<void> {
  await invoke("pause_recording");
}

export async function resumeRecording(): Promise<void> {
  await invoke("resume_recording");
}

export async function clearSteps(): Promise<void> {
  await invoke("clear_steps");
}

export async function getRecordingStatus(): Promise<RecordingStatus> {
  return invoke<RecordingStatus>("get_recording_status");
}

export async function setRecordingHotkey(hotkey: string | null): Promise<void> {
  await invoke("set_recording_hotkey", { shortcut: hotkey });
}

export async function exportTutorial(
  command: ExportCommand,
  title: string,
  steps: Step[],
): Promise<string | null> {
  return invoke<string | null>(command, { title, steps });
}

export async function openOcrDebugFolder(): Promise<void> {
  await invoke("open_ocr_debug_folder");
}
