import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { message } from "@tauri-apps/plugin-dialog";

import { ErrorBanner } from "@/components/common/ErrorBanner";
import { ControlPanel } from "@/components/layout/ControlPanel";
import { StepsPanel } from "@/components/layout/StepsPanel";
import { TopBar } from "@/components/layout/TopBar";
import { EmptyState } from "@/components/steps/EmptyState";
import { StepList } from "@/components/steps/StepList";
import { useAsyncAction } from "@/hooks/useAsyncAction";
import { useRecordingSession } from "@/hooks/useRecordingSession";
import { loadHotkey, saveHotkey } from "@/lib/settings/hotkey";
import { loadRecordingOptions, saveRecordingOptions } from "@/lib/settings/recording";
import {
  clearSteps,
  DEFAULT_TUTORIAL_TITLE,
  exportTutorial,
  setRecordingHotkey,
  startRecording,
  stopRecording,
  type ExportCommand,
} from "@/lib/tauri/commands";
import { TAURI_EVENTS } from "@/lib/tauri/events";
import type { RecordingOptions } from "@/types/recording";

export default function MainApp() {
  const {
    recording,
    recordingPaused,
    finalizing,
    ocrProgress,
    elapsedMs,
    steps,
    setSteps,
    setRecording,
    setRecordingPaused,
    setElapsedMs,
  } = useRecordingSession();
  const { busy, error, run, setBusy, setError } = useAsyncAction();

  const [savingGif, setSavingGif] = useState(false);
  const [recordingOptions, setRecordingOptions] = useState<RecordingOptions>(() =>
    loadRecordingOptions(),
  );
  const [hotkey, setHotkey] = useState<string | null>(() => loadHotkey());

  const screenshotCount = steps.filter((step) => step.imageBase64).length;
  const canExportMedia = screenshotCount > 0;

  const beginRecording = () =>
    run(async () => {
      setSteps([]);
      setElapsedMs(0);
      await startRecording(recordingOptions);
    });

  const handleStart = async () => {
    if (steps.length > 0) {
      const result = await message(
        `当前已有 ${steps.length} 个步骤。开始新录制后，这些内容将被替换。\n\n建议先保存教程，或确认放弃后再继续。`,
        {
          title: "开始新录制",
          kind: "warning",
          buttons: {
            yes: "保存并继续",
            no: "放弃并开始",
            cancel: "取消",
          },
        },
      );

      if (result === "取消") return;

      if (result === "保存并继续") {
        setBusy(true);
        setError(null);
        try {
          const path = await exportTutorial("export_gif", DEFAULT_TUTORIAL_TITLE, steps);
          if (!path) return;
        } catch (cause) {
          setError(String(cause));
          return;
        } finally {
          setBusy(false);
        }
      } else if (result !== "放弃并开始") {
        return;
      }
    }

    beginRecording();
  };

  const handleStop = () =>
    run(async () => {
      const finalSteps = await stopRecording();
      setSteps(finalSteps);
      setRecording(false);
      setRecordingPaused(false);
      setElapsedMs(0);
    });

  const handleClear = () =>
    run(async () => {
      await clearSteps();
      setSteps([]);
    });

  const saveTutorialFile = async (command: ExportCommand) => {
    await exportTutorial(command, DEFAULT_TUTORIAL_TITLE, steps);
  };

  const handleExportWord = () => run(() => saveTutorialFile("export_word"));
  const handleExportGif = () =>
    run(async () => {
      setSavingGif(true);
      try {
        await saveTutorialFile("export_gif");
      } finally {
        setSavingGif(false);
      }
    });

  const updateStepDescription = (id: string, description: string) => {
    setSteps((prev) => prev.map((step) => (step.id === id ? { ...step, description } : step)));
  };

  const removeStep = (id: string) => {
    setSteps((prev) => prev.filter((step) => step.id !== id));
  };

  const updateRecordingOptions = (next: RecordingOptions) => {
    setRecordingOptions(next);
    saveRecordingOptions(next);
  };

  const updateHotkey = useCallback((value: string | null) => {
    setHotkey(value);
    saveHotkey(value);
  }, []);

  const commitHotkey = useCallback(
    async (value: string) => {
      await setRecordingHotkey(value);
      updateHotkey(value);
    },
    [updateHotkey],
  );

  const clearHotkey = useCallback(async () => {
    await setRecordingHotkey(null);
    updateHotkey(null);
  }, [updateHotkey]);

  // 启动时恢复上次保存的快捷键；被其他程序占用时只记录日志，不阻塞主流程。
  // 后续变更由设置面板在注册成功后才写入状态，因此这里只在挂载时执行一次。
  const initialHotkeyRef = useRef(hotkey);
  useEffect(() => {
    void setRecordingHotkey(initialHotkeyRef.current).catch((cause) => {
      console.error("注册全局快捷键失败", cause);
    });
  }, []);

  // 全局快捷键按下时由 Rust 发来事件；通过 ref 转发到最新的处理函数，避免监听器反复重挂。
  const hotkeyToggleRef = useRef<() => void>(() => undefined);

  hotkeyToggleRef.current = () => {
    if (busy || finalizing) return;
    if (recording) {
      handleStop();
    } else {
      void handleStart();
    }
  };

  useEffect(() => {
    const unlisten = listen(TAURI_EVENTS.hotkeyRecordingToggle, () => hotkeyToggleRef.current());
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  return (
    <main className="app">
      <div className="app-shell">
        <TopBar recording={recording} recordingPaused={recordingPaused} elapsedMs={elapsedMs} />

        {error ? <ErrorBanner message={error} /> : null}

        <section className="workspace">
          <ControlPanel
            recording={recording}
            busy={busy || finalizing}
            savingGif={savingGif}
            canExportMedia={canExportMedia}
            hasSteps={steps.length > 0}
            stepCount={steps.length}
            screenshotCount={screenshotCount}
            recordingOptions={recordingOptions}
            hotkey={hotkey}
            onStart={handleStart}
            onStop={handleStop}
            onExportWord={handleExportWord}
            onExportGif={handleExportGif}
            onClear={handleClear}
            onRecordingOptionsChange={updateRecordingOptions}
            onHotkeyCommit={commitHotkey}
            onHotkeyClear={clearHotkey}
          />

          <StepsPanel
            stepCount={steps.length}
            finalizing={finalizing}
            ocrProgress={ocrProgress}
          >
            {steps.length === 0 ? (
              <EmptyState />
            ) : (
              <StepList
                steps={steps}
                recording={recording || finalizing || busy}
                onDescriptionChange={updateStepDescription}
                onRemove={removeStep}
              />
            )}
          </StepsPanel>
        </section>
      </div>
    </main>
  );
}
