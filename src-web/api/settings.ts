import { invoke } from '@tauri-apps/api/core';

export type DownloadSettings = {
  outputDirectory: string;
  defaultOutputDirectory: string;
  customOutputDirectory: boolean;
};

export function getDownloadSettings(): Promise<DownloadSettings> {
  return invoke<DownloadSettings>('get_download_settings_command');
}

export function setDownloadDirectory(outputDirectory: string): Promise<DownloadSettings> {
  return invoke<DownloadSettings>('set_download_directory_command', { outputDirectory });
}

export function resetDownloadDirectory(): Promise<DownloadSettings> {
  return invoke<DownloadSettings>('reset_download_directory_command');
}
