import { describe, it, expect, vi, beforeEach } from "vitest";

const mockIsPermissionGranted = vi.hoisted(() => vi.fn());
const mockRequestPermission = vi.hoisted(() => vi.fn());
const mockSendNotification = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: mockIsPermissionGranted,
  requestPermission: mockRequestPermission,
  sendNotification: mockSendNotification,
}));

import { useNotification, sendSystemNotification } from "@/composables/useNotification";

describe("useNotification", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("useNotification returns sendNotification function", () => {
    const { sendNotification } = useNotification();
    expect(typeof sendNotification).toBe("function");
  });

  it("sendSystemNotification sends when permission is granted", async () => {
    mockIsPermissionGranted.mockResolvedValue(true);

    await sendSystemNotification("Title", "Body");

    expect(mockSendNotification).toHaveBeenCalledWith({ title: "Title", body: "Body" });
    expect(mockRequestPermission).not.toHaveBeenCalled();
  });

  it("sendSystemNotification requests permission if not granted", async () => {
    mockIsPermissionGranted.mockResolvedValue(false);
    mockRequestPermission.mockResolvedValue("granted");

    await sendSystemNotification("Title", "Body");

    expect(mockRequestPermission).toHaveBeenCalled();
    expect(mockSendNotification).toHaveBeenCalledWith({ title: "Title", body: "Body" });
  });

  it("sendSystemNotification does not send if permission denied", async () => {
    mockIsPermissionGranted.mockResolvedValue(false);
    mockRequestPermission.mockResolvedValue("denied");

    await sendSystemNotification("Title", "Body");

    expect(mockSendNotification).not.toHaveBeenCalled();
  });
});
