export type RecordingStatus = {
  recording: boolean;
  paused: boolean;
  elapsedMs: number;
  stepCount: number;
};

export type OcrProgress = {
  completed: number;
  total: number;
  activeStep: number | null;
};

export type RecordingOptions = {
  excludeRecordingBar: boolean;
  ocrDebug: boolean;
};
