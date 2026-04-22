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
};

export type StartDownloadResult = {
  outputName: string;
  outputPath: string;
  mediaPlaylistUrl: string;
  outputBytes: number;
  trackCount: number;
};

export type DownloadProgressPayload = {
  status: DownloadStatus;
  completedSegments: number;
  totalSegments: number | null;
  downloadedBytes: number;
  totalBytes: number | null;
  message?: string | null;
};

export function startDownload(request: StartDownloadRequest): Promise<StartDownloadResult> {
  return invoke<StartDownloadResult>('start_download_command', { request });
}

export function listenForDownloadProgress(
  callback: (payload: DownloadProgressPayload) => void
): Promise<UnlistenFn> {
  return listen<DownloadProgressPayload>('download:progress', (event) => {
    callback(event.payload);
  });
}
