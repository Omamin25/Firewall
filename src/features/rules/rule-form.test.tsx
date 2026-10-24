// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { RuleForm } from "@/features/rules/rule-form";
import { useRuleForm } from "@/features/rules/use-rule-form";
import type { RuleDraft } from "@/types/ipc";

afterEach(cleanup);

const FORM_ID = "test-rule-form";

/** Stands in for `RuleDialog`: the hook plus the footer submit it drives. */
function Harness({
  onSubmit = () => undefined,
  serverError = null,
}: {
  onSubmit?: (draft: RuleDraft | null) => void;
  serverError?: string | null;
}): JSX.Element {
  const form = useRuleForm();
  return (
    <>
      <RuleForm
        form={form}
        formId={FORM_ID}
        serverError={serverError}
        onSubmit={() => onSubmit(form.draft)}
      />
      <button type="submit" form={FORM_ID} disabled={!form.valid}>
        Create rule
      </button>
    </>
  );
}

function field(label: string): HTMLElement {
  return screen.getByLabelText(label);
}

function submitButton(): HTMLButtonElement {
  return screen.getByRole("button", { name: "Create rule" });
}

describe("RuleForm", () => {
  it("keeps submit disabled until the rule has a name", () => {
    render(<Harness />);
    expect(submitButton().disabled).toBe(true);
    fireEvent.change(field("Name"), { target: { value: "Block telemetry" } });
    expect(submitButton().disabled).toBe(false);
  });

  it("does not show a message until the field is left", () => {
    render(<Harness />);
    fireEvent.change(field("Remote address"), { target: { value: "not-an-address" } });
    expect(screen.queryByText(/Use an address or CIDR block/)).toBeNull();
    fireEvent.blur(field("Remote address"));
    expect(screen.getByText(/Use an address or CIDR block/)).toBeTruthy();
  });

  it("clears the message and re-enables submit once the value is fixed", () => {
    render(<Harness />);
    fireEvent.change(field("Name"), { target: { value: "Block telemetry" } });
    fireEvent.change(field("Remote port"), { target: { value: "99999" } });
    fireEvent.blur(field("Remote port"));
    expect(screen.getByText(/Ports run from 1 to 65535/)).toBeTruthy();
    expect(submitButton().disabled).toBe(true);

    fireEvent.change(field("Remote port"), { target: { value: "443" } });
    expect(screen.queryByText(/Ports run from 1 to 65535/)).toBeNull();
    expect(submitButton().disabled).toBe(false);
  });

  it("submits the validated draft, with blanks left as any", () => {
    const onSubmit = vi.fn();
    render(<Harness onSubmit={onSubmit} />);
    fireEvent.change(field("Name"), { target: { value: "  Block telemetry  " } });
    fireEvent.change(field("Remote address"), { target: { value: "10.0.0.0/8" } });
    fireEvent.change(field("Priority"), { target: { value: "20" } });
    fireEvent.click(submitButton());

    expect(onSubmit).toHaveBeenCalledWith({
      name: "Block telemetry",
      action: "block",
      direction: "both",
      protocol: "any",
      remoteIp: "10.0.0.0/8",
      remotePort: null,
      localPort: null,
      processPath: null,
      userSid: null,
      enabled: true,
      priority: 20,
    });
  });

  it("toggles the enabled switch through its accessible role", () => {
    render(<Harness />);
    const toggle = screen.getByRole("switch", { name: "Rule enabled" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("false");
  });

  it("surfaces a rejection from the Rust side above the fields", () => {
    render(<Harness serverError="a rule needs a name" />);
    expect(screen.getByRole("alert").textContent).toContain("a rule needs a name");
  });
});
