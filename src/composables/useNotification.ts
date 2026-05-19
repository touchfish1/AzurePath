import {
  sendNotification,
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";

/**
 * Send a system notification with automatic permission handling.
 * Requests permission if not yet granted.
 */
export async function sendSystemNotification(title: string, body: string) {
  let granted = await isPermissionGranted();
  if (!granted) {
    const permission = await requestPermission();
    granted = permission === "granted";
  }
  if (granted) {
    sendNotification({ title, body });
  }
}

/**
 * Composable that provides the `sendSystemNotification` function.
 */
export function useNotification() {
  return { sendNotification: sendSystemNotification };
}
