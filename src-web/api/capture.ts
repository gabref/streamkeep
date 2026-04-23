import { addPluginListener, type PluginListener } from '@tauri-apps/api/core';

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

export function listenForCaptureRequest(
  callback: (payload: CaptureRequestPayload) => void
): Promise<PluginListener> {
  return addPluginListener<CaptureRequestPayload>(
    PLUGIN_NAME,
    'capture:request-seen',
    callback
  );
}

export function listenForMasterDetected(
  callback: (payload: CaptureRequestPayload) => void
): Promise<PluginListener> {
  return addPluginListener<CaptureRequestPayload>(
    PLUGIN_NAME,
    'capture:master-detected',
    callback
  );
}

export function listenForCaptureDownloadRequested(
  callback: (payload: CaptureDownloadRequestPayload) => void
): Promise<PluginListener> {
  return addPluginListener<CaptureDownloadRequestPayload>(
    PLUGIN_NAME,
    'capture:download-requested',
    callback
  );
}
