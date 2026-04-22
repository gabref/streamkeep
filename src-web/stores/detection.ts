import { defineStore } from 'pinia';
import type { CaptureRequestPayload } from '@/api/capture';
import {
  buildMp4FileName,
  createDetectedStream,
  type ConfirmedDownloadIntent,
  type DetectedStream,
} from '@/domain/detection';

export const useDetectionStore = defineStore('detection', {
  state: () => {
    return {
      pendingStream: null as DetectedStream | null,
      latestStream: null as DetectedStream | null,
      confirmedIntent: null as ConfirmedDownloadIntent | null,
    };
  },
  actions: {
    registerDetectedPayload(payload: CaptureRequestPayload): DetectedStream {
      const stream = createDetectedStream(payload);
      this.latestStream = stream;
      this.pendingStream = stream;
      return stream;
    },
    dismissPending(): void {
      this.pendingStream = null;
    },
    confirmDownload(fileNameStem: string, qualityId: string): ConfirmedDownloadIntent | null {
      if (!this.pendingStream) {
        return null;
      }

      const intent = {
        stream: this.pendingStream,
        fileName: buildMp4FileName(fileNameStem),
        qualityId,
        confirmedAt: new Date().toISOString(),
      };
      this.confirmedIntent = intent;
      this.pendingStream = null;
      return intent;
    },
  },
});
