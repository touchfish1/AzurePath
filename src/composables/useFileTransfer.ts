import { type Ref } from "vue";
import {
  onFileRequest,
  onFileProgress,
  onFileComplete,
  onFileError,
  type FileTransfer,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

export interface IncomingRequest {
  fileId: string;
  filename: string;
  size: number;
  from: string;
}

/**
 * Shared composable for listening to file transfer events.
 * Replaces duplicate listener registration patterns in chat and files pages.
 */
export function useFileTransferListeners(
  transfers: Ref<FileTransfer[]>,
  incomingRequest: Ref<IncomingRequest | null>,
) {
  let unlistenRequest: UnlistenFn | null = null;
  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  async function setup() {
    // Clean up any existing listeners first
    teardown();

    unlistenRequest = await onFileRequest((req) => {
      incomingRequest.value = req;
      transfers.value.unshift({
        id: req.fileId,
        filename: req.filename,
        path: null,
        size: req.size,
        received: 0,
        status: "pending",
        peer_id: req.from,
        is_incoming: true,
        created_at: new Date().toISOString(),
      });
    });

    unlistenProgress = await onFileProgress((p) => {
      const t = transfers.value.find((x) => x.id === p.fileId);
      if (t) {
        t.received = p.received;
        t.size = p.total;
        if (t.status !== "pending") t.status = "transferring";
      }
    });

    unlistenComplete = await onFileComplete((p) => {
      const t = transfers.value.find((x) => x.id === p.fileId);
      if (t) {
        t.status = "completed";
        t.received = t.size;
        t.path = p.path;
        t.download_url = p.downloadUrl;
      }
    });

    unlistenError = await onFileError((p) => {
      const t = transfers.value.find((x) => x.id === p.fileId);
      if (t) t.status = "failed";
    });
  }

  function teardown() {
    unlistenRequest?.();
    unlistenProgress?.();
    unlistenComplete?.();
    unlistenError?.();
    unlistenRequest = null;
    unlistenProgress = null;
    unlistenComplete = null;
    unlistenError = null;
  }

  return { setup, teardown };
}
