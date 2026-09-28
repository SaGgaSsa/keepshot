import { invoke } from "@tauri-apps/api/core";

/** Forwards a user-visible error to the backend log (terminal and keepshot.log). */
export function reportError(source: string, error: unknown): string {
  const message = error instanceof Error ? `${error.message}\n${error.stack ?? ""}`.trim() : String(error);
  void invoke("log_client_error", { source, message }).catch(() => {});
  return error instanceof Error ? error.message : String(error);
}
