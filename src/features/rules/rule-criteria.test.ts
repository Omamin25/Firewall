import { describe, expect, it } from "vitest";
import {
  DEFAULT_PRIORITY,
  EMPTY_RULE_FORM,
  draftFromFormValues,
  fileName,
  formValuesFromDraft,
  summarizeCriteria,
  validateRuleForm,
  type RuleFormValues,
} from "@/features/rules/rule-criteria";
import type { Rule, RuleDraft } from "@/types/ipc";

function values(patch: Partial<RuleFormValues> = {}): RuleFormValues {
  return { ...EMPTY_RULE_FORM, name: "Block telemetry", ...patch };
}

function rule(patch: Partial<Rule> = {}): Rule {
  return {
    id: 1,
    name: "Block telemetry",
    action: "block",
    direction: "out",
    protocol: "tcp",
    remoteIp: null,
    remotePort: null,
    localPort: null,
    processPath: null,
    userSid: null,
    enabled: true,
    priority: DEFAULT_PRIORITY,
    hitCount: 0,
    createdAt: "2026-08-01T10:00:00Z",
    updatedAt: "2026-08-01T10:00:00Z",
    ...patch,
  };
}

describe("validateRuleForm", () => {
  it("accepts a minimal named rule", () => {
    expect(validateRuleForm(values())).toEqual({});
  });

  it("requires a name", () => {
    expect(validateRuleForm(values({ name: "   " })).name).toBeDefined();
  });

  it("accepts addresses and CIDR blocks for both families", () => {
    for (const remoteIp of ["93.184.216.34", "10.0.0.0/8", "2606:4700::1", "2606:4700::/32"]) {
      expect(validateRuleForm(values({ remoteIp })).remoteIp).toBeUndefined();
    }
  });

  it("rejects malformed addresses and out-of-range prefixes", () => {
    for (const remoteIp of ["93.184.216", "300.1.1.1", "10.0.0.0/33", "10.0.0.0/8/8", "hostname"]) {
      expect(validateRuleForm(values({ remoteIp })).remoteIp).toBeDefined();
    }
  });

  it("bounds ports to the wire range", () => {
    expect(validateRuleForm(values({ remotePort: "443" })).remotePort).toBeUndefined();
    expect(validateRuleForm(values({ remotePort: "0" })).remotePort).toBeDefined();
    expect(validateRuleForm(values({ remotePort: "65536" })).remotePort).toBeDefined();
    expect(validateRuleForm(values({ localPort: "half" })).localPort).toBeDefined();
  });

  it("bounds the priority to the ordering space", () => {
    expect(validateRuleForm(values({ priority: "0" })).priority).toBeUndefined();
    expect(validateRuleForm(values({ priority: "-1" })).priority).toBeDefined();
    expect(validateRuleForm(values({ priority: "100001" })).priority).toBeDefined();
    expect(validateRuleForm(values({ priority: "" })).priority).toBeDefined();
  });

  it("only accepts values shaped like a SID", () => {
    expect(validateRuleForm(values({ userSid: "S-1-5-21-7-1001" })).userSid).toBeUndefined();
    expect(validateRuleForm(values({ userSid: "OMEN\\Vraj" })).userSid).toBeDefined();
  });
});

describe("draftFromFormValues", () => {
  it("trims text, nulls blanks and parses numbers", () => {
    const draft = draftFromFormValues(
      values({
        name: "  Block telemetry  ",
        remoteIp: " 10.0.0.0/8 ",
        remotePort: " 443 ",
        localPort: "",
        processPath: "  ",
        priority: "20",
      }),
    );
    expect(draft).toEqual({
      name: "Block telemetry",
      action: "block",
      direction: "both",
      protocol: "any",
      remoteIp: "10.0.0.0/8",
      remotePort: 443,
      localPort: null,
      processPath: null,
      userSid: null,
      enabled: true,
      priority: 20,
    });
  });

  it("refuses to build a draft from an invalid form", () => {
    expect(draftFromFormValues(values({ name: "" }))).toBeNull();
  });
});

describe("formValuesFromDraft", () => {
  it("round-trips through the form representation", () => {
    const draft: RuleDraft = {
      name: "Allow updates",
      action: "allow",
      direction: "in",
      protocol: "udp",
      remoteIp: "1.1.1.1",
      remotePort: 53,
      localPort: null,
      processPath: "C:\\Windows\\System32\\svchost.exe",
      userSid: null,
      enabled: false,
      priority: 10,
    };
    expect(draftFromFormValues(formValuesFromDraft(draft))).toEqual(draft);
  });
});

describe("summarizeCriteria", () => {
  it("joins the criteria a rule actually names", () => {
    expect(
      summarizeCriteria(rule({ remoteIp: "10.0.0.0/8", remotePort: 443, processPath: "C:\\a\\b.exe" })),
    ).toBe("TCP · 10.0.0.0/8:443 · b.exe");
  });

  it("names a bare port when there is no address", () => {
    expect(summarizeCriteria(rule({ protocol: "any", remotePort: 53 }))).toBe("remote port 53");
  });

  it("says so when a rule matches everything", () => {
    expect(summarizeCriteria(rule({ protocol: "any" }))).toBe("Any traffic");
  });
});

describe("fileName", () => {
  it("takes the trailing component of a Windows path", () => {
    expect(fileName("C:\\Program Files\\App\\telemetry.exe")).toBe("telemetry.exe");
  });

  it("leaves a bare executable name alone", () => {
    expect(fileName("telemetry.exe")).toBe("telemetry.exe");
  });
});
