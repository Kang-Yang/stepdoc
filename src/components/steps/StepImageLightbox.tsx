import { useEffect } from "react";

import { stepImageSrc } from "@/lib/format/step";
import type { Step } from "@/types/step";

type StepImageLightboxProps = {
  step: Step;
  stepIndex: number;
  onClose: () => void;
};

export function StepImageLightbox({ step, stepIndex, onClose }: StepImageLightboxProps) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", onKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose]);

  return (
    <div
      className="step-lightbox"
      role="dialog"
      aria-modal="true"
      aria-label={`步骤 ${stepIndex + 1} 截图预览`}
      onClick={onClose}
    >
      <div className="step-lightbox__panel" onClick={(event) => event.stopPropagation()}>
        <header className="step-lightbox__header">
          <div>
            <p className="step-lightbox__title">步骤 {stepIndex + 1}</p>
            <p className="step-lightbox__desc">{step.description}</p>
          </div>
          <button type="button" className="step-lightbox__close" onClick={onClose} aria-label="关闭预览">
            关闭
          </button>
        </header>
        <div className="step-lightbox__image-wrap">
          <img
            src={stepImageSrc(step)}
            alt={step.description}
            className="step-lightbox__image"
            draggable={false}
            onDoubleClick={(event) => event.preventDefault()}
          />
        </div>
        <p className="step-lightbox__hint">已为最大预览，按 Esc 或点击空白处关闭</p>
      </div>
    </div>
  );
}
