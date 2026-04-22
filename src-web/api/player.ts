import { invoke } from '@tauri-apps/api/core';

export interface PlayerState {
  supported: boolean;
  visible: boolean;
  loading: boolean;
  url: string | null;
  title: string | null;
  canGoBack: boolean;
  canGoForward: boolean;
}

export function openPlayer(url?: string): Promise<PlayerState> {
  return invoke<PlayerState>('open_player_command', { url });
}

export function getPlayerState(): Promise<PlayerState> {
  return invoke<PlayerState>('get_player_state_command');
}

export function playerGoBack(): Promise<PlayerState> {
  return invoke<PlayerState>('player_go_back_command');
}

export function playerGoForward(): Promise<PlayerState> {
  return invoke<PlayerState>('player_go_forward_command');
}

export function playerReload(): Promise<PlayerState> {
  return invoke<PlayerState>('player_reload_command');
}

export function playerLoadUrl(url: string): Promise<PlayerState> {
  return invoke<PlayerState>('player_load_url_command', { url });
}
