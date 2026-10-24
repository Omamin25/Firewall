import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  CaptureInterface,
  CaptureStatus,
  EnforcementStatus,
  Flow,
  FlowHistorySummary,
  PacketDetail,
  PruneResult,
  Rule,
  RuleAction,
  RuleDraft,
  RuleScope,
  StreamMessage,
  SystemHealth,
} from "@/types/ipc";

/** Checks whether the native Tauri desktop webview runtime is attached. */
export function isTauriAvailable(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriAvailable()) {
    return Promise.reject(
      "Running in web browser mode — Cognton requires the native Tauri desktop shell to access Windows packet drivers. Launch the native desktop application window.",
    );
  }
  return invoke<T>(cmd, args);
}

export function getAppStatus(): Promise<AppStatus> {
  return safeInvoke<AppStatus>("demo_health_check");
}

export function runDatabaseSmokeTest(): Promise<string> {
  return safeInvoke<string>("database_smoke_test");
}

export function listCaptureInterfaces(): Promise<CaptureInterface[]> {
  return safeInvoke<CaptureInterface[]>("list_capture_interfaces");
}

/** Starts capture. An empty/omitted selection lets Rust pick every recommended adapter. */
export function startCapture(interfaceIds?: string[]): Promise<CaptureStatus> {
  return safeInvoke<CaptureStatus>("start_capture", { interfaceIds: interfaceIds ?? [] });
}

export function stopCapture(): Promise<CaptureStatus> {
  return safeInvoke<CaptureStatus>("stop_capture");
}

export function getPacketDetail(packetId: number): Promise<PacketDetail> {
  return safeInvoke<PacketDetail>("get_packet_detail", { packetId });
}

/** Every live flow. The stream only carries diffs, so the view seeds itself from here. */
export function listFlows(): Promise<Flow[]> {
  return safeInvoke<Flow[]>("list_flows");
}

export function getFlowHistorySummary(): Promise<FlowHistorySummary> {
  return safeInvoke<FlowHistorySummary>("get_flow_history_summary");
}

/** Registers a single batched channel for packet + stats pushes (~15 Hz). */
export async function subscribePacketStream(
  onMessage: (message: StreamMessage) => void,
): Promise<void> {
  if (!isTauriAvailable()) {
    return Promise.reject(
      "Running in web browser mode — Cognton requires the native Tauri desktop shell to access Windows packet drivers.",
    );
  }
  const channel = new Channel<StreamMessage>();
  channel.onmessage = onMessage;
  await safeInvoke("subscribe_packet_stream", { channel });
}

export function listRules(): Promise<Rule[]> {
  return safeInvoke<Rule[]>("list_rules");
}

/** Rust validates every field and rejects the draft with a message for the form. */
export function createRule(draft: RuleDraft): Promise<Rule> {
  return safeInvoke<Rule>("create_rule", { draft });
}

export function updateRule(ruleId: number, draft: RuleDraft): Promise<Rule> {
  return safeInvoke<Rule>("update_rule", { ruleId, draft });
}

/** Enable/disable without touching criteria — takes effect before the promise resolves. */
export function toggleRule(ruleId: number, enabled: boolean): Promise<Rule> {
  return safeInvoke<Rule>("toggle_rule", { ruleId, enabled });
}

export function deleteRule(ruleId: number): Promise<void> {
  return safeInvoke<void>("delete_rule", { ruleId });
}

/** Criteria prefilled from a live connection, for the create-rule form. No write. */
export function draftRuleFromFlow(
  flowId: number,
  scope: RuleScope,
  action: RuleAction,
): Promise<RuleDraft> {
  return safeInvoke<RuleDraft>("draft_rule_from_flow", { flowId, scope, action });
}

/** Same, from a packet in the inspector. Rust maps the packet back to its flow. */
export function draftRuleFromPacket(
  packetId: number,
  scope: RuleScope,
  action: RuleAction,
): Promise<RuleDraft> {
  return safeInvoke<RuleDraft>("draft_rule_from_packet", { packetId, scope, action });
}

/** Creates the rule a context menu would build, in one step. */
export function createRuleFromFlow(
  flowId: number,
  scope: RuleScope,
  action: RuleAction,
): Promise<Rule> {
  return safeInvoke<Rule>("create_rule_from_flow", { flowId, scope, action });
}

export function getEnforcementStatus(): Promise<EnforcementStatus> {
  return safeInvoke<EnforcementStatus>("get_enforcement_status");
}

export function updateRetentionPolicy(
  retentionHours: number,
  maxRows: number,
): Promise<FlowHistorySummary> {
  return safeInvoke<FlowHistorySummary>("update_retention_policy", { retentionHours, maxRows });
}

export function pruneFlowHistory(): Promise<PruneResult> {
  return safeInvoke<PruneResult>("prune_flow_history");
}

export function getSystemHealth(): Promise<SystemHealth> {
  return safeInvoke<SystemHealth>("get_system_health");
}
