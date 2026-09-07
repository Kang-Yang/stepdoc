import type { ReactNode } from "react";

import type { OcrProgress } from "@/types/recording";

type StepsPanelProps = {
  stepCount: number;
  finalizing: boolean;
  ocrProgress: OcrProgress;
  children: ReactNode;
};

export function StepsPanel({ stepCount, finalizing, ocrProgress, children }: StepsPanelProps) {
  const { completed, total, activeStep } = ocrProgress;
  const currentStep = activeStep ?? (completed < total ? completed + 1 : total);
  const progress = total > 0 ? Math.round((completed / total) * 100) : 0;

  return (
    <section className="steps-panel">
      <header className="steps-panel__header">
        <h2 className="steps-panel__title">录制步骤</h2>
        <span className="steps-panel__count">{stepCount} 步</span>
      </header>
      <div className="steps-panel__body">
        {finalizing ? (
          <div className="steps-panel__processing" role="status">
            <span className="steps-panel__spinner" aria-hidden />
            <div className="steps-panel__processing-content">
              <div className="steps-panel__processing-copy">
                <strong>
                  {total > 0 ? `正在识别第 ${currentStep} / ${total} 步` : "正在整理截图"}
                </strong>
                <span>
                  {total > 0
                    ? `已完成 ${completed} / ${total} 步，识别结果会自动更新。`
                    : "正在收集最后一次点击的截图，请稍候。"}
                </span>
              </div>
              <div
                className="steps-panel__progress"
                role="progressbar"
                aria-label="OCR 识别进度"
                aria-valuemin={0}
                aria-valuemax={total || 1}
                aria-valuenow={completed}
              >
                <span style={{ width: `${progress}%` }} />
              </div>
            </div>
          </div>
        ) : null}
        {children}
      </div>
    </section>
  );
}
