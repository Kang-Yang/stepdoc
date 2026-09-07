import { useState } from "react";

import { StepCard } from "@/components/steps/StepCard";
import { StepImageLightbox } from "@/components/steps/StepImageLightbox";
import type { Step } from "@/types/step";

type StepListProps = {
  steps: Step[];
  recording: boolean;
  onDescriptionChange: (id: string, description: string) => void;
  onRemove: (id: string) => void;
};

export function StepList({ steps, recording, onDescriptionChange, onRemove }: StepListProps) {
  const [preview, setPreview] = useState<{ step: Step; index: number } | null>(null);

  return (
    <>
      <div className="steps-list">
        {steps.map((step, index) => (
          <StepCard
            key={step.id}
            step={step}
            index={index}
            total={steps.length}
            recording={recording}
            onDescriptionChange={onDescriptionChange}
            onRemove={onRemove}
            onImagePreview={() => setPreview({ step, index })}
          />
        ))}
      </div>

      {preview ? (
        <StepImageLightbox
          step={preview.step}
          stepIndex={preview.index}
          onClose={() => setPreview(null)}
        />
      ) : null}
    </>
  );
}
