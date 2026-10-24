import type { Rule, RuleAction, RuleDirection, RuleDraft, RuleProtocol } from "@/types/ipc";

/** Limits mirrored from `ipc/dto/rule.rs`. Rust re-validates every field — these exist so
 *  the user sees the problem in the form instead of as a rejected command. */
export const MAX_NAME_LENGTH = 120;
export const MAX_PROCESS_PATH_LENGTH = 512;
export const MAX_SID_LENGTH = 184;
export const MAX_PRIORITY = 100_000;
export const DEFAULT_PRIORITY = 100;

/** Every field as the form holds it: strings, so a half-typed port is representable. */
export type RuleFormValues = {
  name: string;
  action: RuleAction;
  direction: RuleDirection;
  protocol: RuleProtocol;
  remoteIp: string;
  remotePort: string;
  localPort: string;
  processPath: string;
  userSid: string;
  enabled: boolean;
  priority: string;
};

export type RuleFormField = keyof RuleFormValues;
export type RuleFormErrors = Partial<Record<RuleFormField, string>>;

export const EMPTY_RULE_FORM: RuleFormValues = {
  name: "",
  action: "block",
  direction: "both",
  protocol: "any",
  remoteIp: "",
  remotePort: "",
  localPort: "",
  processPath: "",
  userSid: "",
  enabled: true,
  priority: String(DEFAULT_PRIORITY),
};

/** Turns a draft (from the server, or prefilled from a flow) into form values. */
export function formValuesFromDraft(draft: RuleDraft): RuleFormValues {
  return {
    name: draft.name,
    action: draft.action,
    direction: draft.direction,
    protocol: draft.protocol,
    remoteIp: draft.remoteIp ?? "",
    remotePort: draft.remotePort === null ? "" : String(draft.remotePort),
    localPort: draft.localPort === null ? "" : String(draft.localPort),
    processPath: draft.processPath ?? "",
    userSid: draft.userSid ?? "",
    enabled: draft.enabled,
    priority: String(draft.priority),
  };
}

export function formValuesFromRule(rule: Rule): RuleFormValues {
  return formValuesFromDraft(rule);
}

/** Validated form values as the IPC layer wants them, or `null` when anything is wrong. */
export function draftFromFormValues(values: RuleFormValues): RuleDraft | null {
  if (Object.keys(validateRuleForm(values)).length > 0) return null;
  return {
    name: values.name.trim(),
    action: values.action,
    direction: values.direction,
    protocol: values.protocol,
    remoteIp: blankToNull(values.remoteIp),
    remotePort: portOrNull(values.remotePort),
    localPort: portOrNull(values.localPort),
    processPath: blankToNull(values.processPath),
    userSid: blankToNull(values.userSid),
    enabled: values.enabled,
    priority: Number.parseInt(values.priority, 10),
  };
}

/** Field-keyed messages, empty when the form is submittable. Deliberately lenient about
 *  address syntax: it catches shapes that are obviously wrong and leaves the final word
 *  to the Rust matcher, whose error is surfaced on the form. */
export function validateRuleForm(values: RuleFormValues): RuleFormErrors {
  const errors: RuleFormErrors = {};

  const name = values.name.trim();
  if (name === "") errors.name = "A rule needs a name.";
  else if (name.length > MAX_NAME_LENGTH)
    errors.name = `At most ${MAX_NAME_LENGTH} characters.`;

  const remote = values.remoteIp.trim();
  if (remote !== "" && !looksLikeAddressOrCidr(remote))
    errors.remoteIp = "Use an address or CIDR block, e.g. 93.184.216.34 or 10.0.0.0/8.";

  for (const field of ["remotePort", "localPort"] as const) {
    const raw = values[field].trim();
    if (raw === "") continue;
    const port = Number(raw);
    if (!Number.isInteger(port) || port < 1 || port > 65_535)
      errors[field] = "Ports run from 1 to 65535.";
  }

  if (values.processPath.trim().length > MAX_PROCESS_PATH_LENGTH)
    errors.processPath = `At most ${MAX_PROCESS_PATH_LENGTH} characters.`;

  const sid = values.userSid.trim();
  if (sid !== "" && (!/^S-1-[\dA-Za-z-]+$/i.test(sid) || sid.length > MAX_SID_LENGTH))
    errors.userSid = "A user SID looks like S-1-5-21-…";

  // Unlike the criteria fields, an empty priority is not "any" — every rule has a place
  // in the ordering, so a blank box is a mistake rather than a wildcard.
  const rawPriority = values.priority.trim();
  const priority = Number(rawPriority);
  if (rawPriority === "" || !Number.isInteger(priority) || priority < 0 || priority > MAX_PRIORITY)
    errors.priority = `A whole number from 0 to ${MAX_PRIORITY}.`;

  return errors;
}

/** One-line description of what a rule matches, for the table's Criteria column. */
export function summarizeCriteria(rule: Rule): string {
  const parts: string[] = [];
  if (rule.protocol !== "any") parts.push(rule.protocol.toUpperCase());
  if (rule.remoteIp !== null)
    parts.push(rule.remotePort === null ? rule.remoteIp : `${rule.remoteIp}:${rule.remotePort}`);
  else if (rule.remotePort !== null) parts.push(`remote port ${rule.remotePort}`);
  if (rule.localPort !== null) parts.push(`local port ${rule.localPort}`);
  if (rule.processPath !== null) parts.push(fileName(rule.processPath));
  if (rule.userSid !== null) parts.push(rule.userSid);
  // A rule with no criteria matches everything — say so rather than showing a blank cell.
  return parts.length === 0 ? "Any traffic" : parts.join(" · ");
}

/** Trailing component of a Windows path; bare executable names pass through unchanged. */
export function fileName(path: string): string {
  const cut = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return cut === -1 ? path : path.slice(cut + 1);
}

function blankToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

function portOrNull(value: string): number | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : Number.parseInt(trimmed, 10);
}

/** Shape check only: IPv4 dotted quad or something with a colon (IPv6), plus an optional
 *  prefix length in range for that family. */
function looksLikeAddressOrCidr(value: string): boolean {
  const [address, prefix, ...rest] = value.split("/");
  if (rest.length > 0 || address === undefined || address === "") return false;

  const ipv6 = address.includes(":");
  if (ipv6) {
    if (!/^[\dA-Fa-f:.]+$/.test(address)) return false;
  } else if (!/^\d{1,3}(\.\d{1,3}){3}$/.test(address)) {
    return false;
  } else if (address.split(".").some((octet) => Number(octet) > 255)) {
    return false;
  }

  if (prefix === undefined) return true;
  const bits = Number(prefix);
  return Number.isInteger(bits) && bits >= 0 && bits <= (ipv6 ? 128 : 32);
}
