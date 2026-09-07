import { useState, type SyntheticEvent } from "react";

import {
  formatStepTime,
  stepImageSrc,
  stepTimeDateTime,
  stepTypeLabel,
} from "@/lib/format/step";
import { IconEdit } from "@/components/icons";
import type { Step } from "@/types/step";

function imageHasZoomHeadroom(image: HTMLImageElement) {
  return (
    image.naturalWidth > image.clientWidth + 2 ||
    image.naturalHeight > image.clientHeight + 2
  );
}

type StepCardProps = {
  step: Step;
  index: number;
  total: number;
  recording: boolean;
  onDescriptionChange: (id: string, description: string) => void;
  onRemove: (id: string) => void;
  onImagePreview: () => void;
};

export function StepCard({
  step,
  index,
  total,
  recording,
  onDescriptionChange,
  onRemove,
  onImagePreview,
}: StepCardProps) {
  const [canZoom, setCanZoom] = useState(true);

  const handleImageLoad = (event: SyntheticEvent<HTMLImageElement>) => {
    setCanZoom(imageHasZoomHeadroom(event.currentTarget));
  };

  return (
    <article className="step-card">
      <div className="step-rail">
        <div className="step-index">{index + 1}</div>
        {index < total - 1 ? <div className="step-line" aria-hidden /> : null}
      </div>
      <div className="step-body">
        <div className="step-meta">
          <div className="step-title">
            <div className={`step-desc-field ${recording ? "is-disabled" : ""}`}>
              <input
                className="step-desc-input"
                value={step.description}
                onChange={(event) => onDescriptionChange(step.id, event.target.value)}
                disabled={recording}
                placeholder="点击编辑步骤说明"
                aria-label={`步骤 ${index + 1} 说明`}
              />
              <span className="step-desc-field__icon" aria-hidden>
                <IconEdit />
              </span>
            </div>
            <span className="step-badge">{stepTypeLabel(step.eventType)}</span>
          </div>
          <div className="step-actions">
            <time dateTime={stepTimeDateTime(step.timestamp)}>{formatStepTime(step.timestamp)}</time>
            <button
              type="button"
              className="step-delete"
              onClick={() => onRemove(step.id)}
              disabled={recording}
              title="删除此步骤"
            >
              删除
            </button>
          </div>
        </div>
        {step.imageBase64 ? (
          canZoom ? (
            <button
              type="button"
              className="step-image-trigger"
              onClick={onImagePreview}
              title="点击放大查看"
              aria-label={`放大查看步骤 ${index + 1} 截图`}
            >
              <img
                src={stepImageSrc(step)}
                alt={step.description}
                onLoad={handleImageLoad}
                draggable={false}
              />
              <span className="step-image-trigger__hint">点击放大</span>
            </button>
          ) : (
            <div className="step-image-wrap">
              <img
                src={stepImageSrc(step)}
                alt={step.description}
                onLoad={handleImageLoad}
                draggable={false}
              />
            </div>
          )
        ) : (
          <div className="no-image">无截图</div>
        )}
      </div>
    </article>
  );
}
