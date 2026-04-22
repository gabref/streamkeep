import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type DownloadStatus =
  | 'queued'
  | 'preparing'
  | 'downloading'
  | 'remuxing'
  | 'done'
  | 'failed'
  | 'cancelled';

export type StartDownloadRequest = {
  masterUrl: string;
  mediaPlaylistUrl?: string | null;
  referer?: string | null;
  userAgent?: string | null;
  cookies?: string | null;
  outputName: string;
  title?: string | null;
  pageUrl?: string | null;
  qualityLabel?: string | null;
};

export type StartDownloadResult = {
  jobId: string;
  outputName: string;
  outputPath: string;
  outputUri: string;
  mediaPlaylistUrl: string;
  outputBytes: number;
  trackCount: number;
};

export type DownloadProgressPayload = {
  jobId: string;
  status: DownloadStatus;
  completedSegments: number;
  totalSegments: number | null;
  downloadedBytes: number;
  totalBytes: number | null;
  message?: string | null;
};

export type DownloadJobRecord = {
  id: string;
  title: string;
  outputName: string;
  pageUrl: string;
  masterUrl: string;
  mediaPlaylistUrl?: string | null;
  quality: string;
  status: DownloadStatus;
  progress: number;
  createdAt: string;
  updatedAt: string;
  outputPath?: string | null;
  outputUri?: string | null;
  outputBytes?: number | null;
  errorMessage?: string | null;
};

export function startDownload(request: StartDownloadRequest): Promise<StartDownloadResult> {
  return invoke<StartDownloadResult>('start_download_command', { request });
}

export function listDownloadHistory(): Promise<DownloadJobRecord[]> {
  return invoke<DownloadJobRecord[]>('list_download_history_command');
}

export function openDownload(contentUri: string): Promise<void> {
  return invoke('open_download_command', { contentUri });
}

export function listenForDownloadProgress(
  callback: (payload: DownloadProgressPayload) => void
): Promise<UnlistenFn> {
  return listen<DownloadProgressPayload>('download:progress', (event) => {
    callback(event.payload);
  });
}

export function listenForDownloadHistoryUpdates(
  callback: (payload: DownloadJobRecord) => void
): Promise<UnlistenFn> {
  return listen<DownloadJobRecord>('download:history-updated', (event) => {
    callback(event.payload);
  });
}
