import { invoke } from "@tauri-apps/api/core";

export interface UiPreferences {
  close_to_tray: boolean;
}

export async function getUiPreferences(): Promise<UiPreferences> {
  return invoke<UiPreferences>("get_ui_preferences");
}

export async function setUiPreferences(prefs: UiPreferences): Promise<void> {
  await invoke("set_ui_preferences", { prefs });
}
