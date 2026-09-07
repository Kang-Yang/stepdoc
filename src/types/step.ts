export type Step = {
  id: string;
  eventType: string;
  description: string;
  coordinates?: number[] | null;
  imageBase64?: string | null;
  timestamp: string;
};

export type StepUpdate = {
  id: string;
  description: string;
};
