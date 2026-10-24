// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SettingsPage } from "@/routes/settings-page";
import * as ipc from "@/lib/ipc";

vi.mock("@/lib/ipc", () => ({
  getFlowHistorySummary: vi.fn(),
  getSystemHealth: vi.fn(),
  updateRetentionPolicy: vi.fn(),
  pruneFlowHistory: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe("SettingsPage", () => {
  it("renders history settings and system health diagnostic tiles", async () => {
    vi.mocked(ipc.getFlowHistorySummary).mockResolvedValue({
      storedFlows: 1250,
      oldestSeen: "2026-08-01T10:00:00.000Z",
      newestSeen: "2026-08-01T12:00:00.000Z",
      retentionHours: 24,
      maxRows: 100000,
    });
    vi.mocked(ipc.getSystemHealth).mockResolvedValue({
      isElevated: true,
      databaseReady: true,
      databasePath: "cognton.sqlite3",
      logDirectory: "C:\\AppData\\Cognton\\logs",
      npcapAvailable: true,
      activeInterfacesCount: 2,
      activeFlowsCount: 15,
      activeRulesCount: 3,
      enforcementStatus: {
        available: true,
        enforcing: true,
        observing: true,
        enabledRules: 3,
        blockRules: 2,
        blockedPackets: 42,
        socketEvents: 100,
        trackedEndpoints: 8,
        error: null,
      },
    });

    render(<SettingsPage />);

    await waitFor(() => {
      expect(screen.getByText("Flow history & retention")).toBeTruthy();
      expect(screen.getByText("System health & diagnostics")).toBeTruthy();
      expect(screen.getByText("1,250")).toBeTruthy();
      expect(screen.getByText("Administrator")).toBeTruthy();
      expect(screen.getByText("Enforcing")).toBeTruthy();
    });
  });

  it("allows updating retention window and max rows", async () => {
    vi.mocked(ipc.getFlowHistorySummary).mockResolvedValue({
      storedFlows: 1250,
      oldestSeen: null,
      newestSeen: null,
      retentionHours: 24,
      maxRows: 100000,
    });
    vi.mocked(ipc.getSystemHealth).mockResolvedValue({
      isElevated: true,
      databaseReady: true,
      databasePath: "cognton.sqlite3",
      logDirectory: "logs",
      npcapAvailable: true,
      activeInterfacesCount: 1,
      activeFlowsCount: 0,
      activeRulesCount: 0,
      enforcementStatus: {
        available: true,
        enforcing: false,
        observing: true,
        enabledRules: 0,
        blockRules: 0,
        blockedPackets: 0,
        socketEvents: 0,
        trackedEndpoints: 0,
        error: null,
      },
    });
    vi.mocked(ipc.updateRetentionPolicy).mockResolvedValue({
      storedFlows: 1250,
      oldestSeen: null,
      newestSeen: null,
      retentionHours: 48,
      maxRows: 500000,
    });

    render(<SettingsPage />);

    await waitFor(() => {
      expect(screen.getByLabelText("Retention window (hours)")).toBeTruthy();
    });

    const hoursInput = screen.getByLabelText("Retention window (hours)");
    fireEvent.change(hoursInput, { target: { value: "48" } });

    const maxRowsInput = screen.getByLabelText("Maximum history rows");
    fireEvent.change(maxRowsInput, { target: { value: "500000" } });

    const saveButton = screen.getByRole("button", { name: "Save retention settings" });
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(ipc.updateRetentionPolicy).toHaveBeenCalledWith(48, 500000);
    });
  });

  it("handles manual history pruning", async () => {
    vi.mocked(ipc.getFlowHistorySummary).mockResolvedValue({
      storedFlows: 1250,
      oldestSeen: null,
      newestSeen: null,
      retentionHours: 24,
      maxRows: 100000,
    });
    vi.mocked(ipc.getSystemHealth).mockResolvedValue({
      isElevated: true,
      databaseReady: true,
      databasePath: "cognton.sqlite3",
      logDirectory: "logs",
      npcapAvailable: true,
      activeInterfacesCount: 1,
      activeFlowsCount: 0,
      activeRulesCount: 0,
      enforcementStatus: {
        available: true,
        enforcing: false,
        observing: true,
        enabledRules: 0,
        blockRules: 0,
        blockedPackets: 0,
        socketEvents: 0,
        trackedEndpoints: 0,
        error: null,
      },
    });
    vi.mocked(ipc.pruneFlowHistory).mockResolvedValue({
      removedCount: 120,
      summary: {
        storedFlows: 1130,
        oldestSeen: null,
        newestSeen: null,
        retentionHours: 24,
        maxRows: 100000,
      },
    });

    render(<SettingsPage />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Prune expired history now" })).toBeTruthy();
    });

    const pruneButton = screen.getByRole("button", { name: "Prune expired history now" });
    fireEvent.click(pruneButton);

    await waitFor(() => {
      expect(ipc.pruneFlowHistory).toHaveBeenCalled();
    });
  });
});
