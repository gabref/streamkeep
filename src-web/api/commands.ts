import { invoke } from '@tauri-apps/api/core';

export type HealthSnapshot = {
  appName: string;
  appVersion: string;
  targetPlatform: string;
};

export async function getHealth(): Promise<HealthSnapshot> {
  if (!hasTauriRuntime()) {
    return {
      appName: 'Streamkeep',
      appVersion: '0.1.0',
      targetPlatform: 'web preview',
    };
  }

  return invoke<HealthSnapshot>('get_health_command');
}

function hasTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

