import { addPluginListener } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type CaptureRequestType = 'master' | 'playlist' | 'segment';
export type CaptureConfidence = 'strong' | 'candidate';

export interface CaptureRequestPayload {
  url: string;
  requestUrl: string;
  masterUrl: string | null;
  pageUrl: string | null;
  referer: string | null;
  userAgent: string | null;
  cookies: string | null;
  pageTitle: string | null;
  documentTitle: string | null;
  openGraphTitle: string | null;
  headingTitle: string | null;
  titleSuggestion: string | null;
  detectedAt: string;
  source: 'webview' | 'service-worker' | 'manual';
  requestType: CaptureRequestType;
  confidence: CaptureConfidence;
}

export interface CaptureDownloadRequestPayload extends CaptureRequestPayload {
  requestedFileNameStem: string;
}

const PLUGIN_NAME = 'streamkeep-capture';

type CaptureListener = {
  unregister: () => Promise<void>;
};

export function listenForCaptureRequest(
  callback: (payload: CaptureRequestPayload) => void
): Promise<CaptureListener> {
  return listenForCaptureEvent('capture:request-seen', callback);
}

export function listenForMasterDetected(
  callback: (payload: CaptureRequestPayload) => void
): Promise<CaptureListener> {
  return listenForCaptureEvent('capture:master-detected', callback);
}

export function listenForCaptureDownloadRequested(
  callback: (payload: CaptureDownloadRequestPayload) => void
): Promise<CaptureListener> {
  return listenForCaptureEvent('capture:download-requested', callback);
}

async function listenForCaptureEvent<T>(
  eventName: string,
  callback: (payload: T) => void
): Promise<CaptureListener> {
  if (/Android/i.test(globalThis.navigator.userAgent)) {
    return addPluginListener<T>(PLUGIN_NAME, eventName, callback);
  }

  const unlisten: UnlistenFn = await listen<T>(eventName, (event) => {
    callback(event.payload);
  });
  return {
    unregister: async () => {
      unlisten();
    },
  };
}
