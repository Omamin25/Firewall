import type { ReactNode } from "react";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/cn";
import type { RuleFormController } from "@/features/rules/use-rule-form";
import type { RuleAction, RuleDirection, RuleProtocol } from "@/types/ipc";

const ACTIONS = [
  { value: "block", label: "Block — drop matching packets" },
  { value: "allow", label: "Allow — let matching packets through" },
] as const;

const DIRECTIONS = [
  { value: "both", label: "Both directions" },
  { value: "out", label: "Outbound only" },
  { value: "in", label: "Inbound only" },
] as const;

const PROTOCOLS = [
  { value: "any", label: "Any protocol" },
  { value: "tcp", label: "TCP" },
  { value: "udp", label: "UDP" },
  { value: "icmp", label: "ICMP" },
] as const;

export type RuleFormProps = {
  form: RuleFormController;
  /** Lets the dialog's footer button submit this form from outside it. */
  formId: string;
  /** Rejection text from Rust, shown above the fields. */
  serverError: string | null;
  onSubmit: () => void;
};

/** Create/edit rule fields. Criteria left blank match anything, which is what makes a
 *  narrow rule easy to write and a broad one deliberate. */
export function RuleForm({ form, formId, serverError, onSubmit }: RuleFormProps): JSX.Element {
  const { values, visibleErrors, setField, markTouched } = form;

  return (
    <form
      id={formId}
      noValidate
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
      className="space-y-4"
    >
      {serverError ? (
        <p
          role="alert"
          className="rounded-md border border-danger bg-danger/10 p-3 text-xs text-danger-strong"
        >
          {serverError}
        </p>
      ) : null}

      <Field id={formId} name="name" label="Name" error={visibleErrors.name}>
        {(control) => (
          <Input
            {...control}
            value={values.name}
            invalid={visibleErrors.name !== undefined}
            onChange={(event) => setField("name", event.target.value)}
            onBlur={() => markTouched("name")}
            placeholder="Block telemetry.exe"
            autoComplete="off"
          />
        )}
      </Field>

      <div className="grid grid-cols-2 gap-3">
        <Field id={formId} name="action" label="Action">
          {(control) => (
            <Select
              {...control}
              value={values.action}
              options={ACTIONS}
              onChange={(event) => setField("action", event.target.value as RuleAction)}
            />
          )}
        </Field>
        <Field
          id={formId}
          name="priority"
          label="Priority"
          error={visibleErrors.priority}
          hint="Lower is evaluated first."
        >
          {(control) => (
            <Input
              {...control}
              mono
              inputMode="numeric"
              value={values.priority}
              invalid={visibleErrors.priority !== undefined}
              onChange={(event) => setField("priority", event.target.value)}
              onBlur={() => markTouched("priority")}
            />
          )}
        </Field>
        <Field id={formId} name="direction" label="Direction">
          {(control) => (
            <Select
              {...control}
              value={values.direction}
              options={DIRECTIONS}
              onChange={(event) => setField("direction", event.target.value as RuleDirection)}
            />
          )}
        </Field>
        <Field id={formId} name="protocol" label="Protocol">
          {(control) => (
            <Select
              {...control}
              value={values.protocol}
              options={PROTOCOLS}
              onChange={(event) => setField("protocol", event.target.value as RuleProtocol)}
            />
          )}
        </Field>
      </div>

      <Field
        id={formId}
        name="remote-ip"
        label="Remote address"
        error={visibleErrors.remoteIp}
        hint="Exact address or CIDR block. Blank matches any remote."
      >
        {(control) => (
          <Input
            {...control}
            mono
            value={values.remoteIp}
            invalid={visibleErrors.remoteIp !== undefined}
            onChange={(event) => setField("remoteIp", event.target.value)}
            onBlur={() => markTouched("remoteIp")}
            placeholder="93.184.216.0/24"
            autoComplete="off"
          />
        )}
      </Field>

      <div className="grid grid-cols-2 gap-3">
        <Field id={formId} name="remote-port" label="Remote port" error={visibleErrors.remotePort}>
          {(control) => (
            <Input
              {...control}
              mono
              inputMode="numeric"
              value={values.remotePort}
              invalid={visibleErrors.remotePort !== undefined}
              onChange={(event) => setField("remotePort", event.target.value)}
              onBlur={() => markTouched("remotePort")}
              placeholder="443"
            />
          )}
        </Field>
        <Field id={formId} name="local-port" label="Local port" error={visibleErrors.localPort}>
          {(control) => (
            <Input
              {...control}
              mono
              inputMode="numeric"
              value={values.localPort}
              invalid={visibleErrors.localPort !== undefined}
              onChange={(event) => setField("localPort", event.target.value)}
              onBlur={() => markTouched("localPort")}
              placeholder="Any"
            />
          )}
        </Field>
      </div>

      <Field
        id={formId}
        name="process"
        label="Process"
        error={visibleErrors.processPath}
        hint="Full image path, or a bare file name to match the executable anywhere."
      >
        {(control) => (
          <Input
            {...control}
            value={values.processPath}
            invalid={visibleErrors.processPath !== undefined}
            onChange={(event) => setField("processPath", event.target.value)}
            onBlur={() => markTouched("processPath")}
            placeholder="telemetry.exe"
            autoComplete="off"
          />
        )}
      </Field>

      <Field
        id={formId}
        name="user-sid"
        label="User SID"
        error={visibleErrors.userSid}
        hint="Restricts the rule to one Windows account."
      >
        {(control) => (
          <Input
            {...control}
            mono
            value={values.userSid}
            invalid={visibleErrors.userSid !== undefined}
            onChange={(event) => setField("userSid", event.target.value)}
            onBlur={() => markTouched("userSid")}
            placeholder="S-1-5-21-…"
            autoComplete="off"
          />
        )}
      </Field>

      <div className="flex items-center justify-between rounded-md border border-border px-3 py-2">
        <span className="text-xs font-medium text-foreground-muted">
          Enabled
          <span className="ml-2 font-normal text-foreground-subtle">
            A disabled rule is kept but never matched.
          </span>
        </span>
        <Switch
          checked={values.enabled}
          onChange={(checked) => setField("enabled", checked)}
          label="Rule enabled"
        />
      </div>
    </form>
  );
}

/** What a field hands its control so the pairing is programmatic, not just visual. */
type ControlProps = { id: string; "aria-describedby": string | undefined };

type FieldProps = {
  /** The form's id, used as the prefix so two open forms cannot collide. */
  id: string;
  name: string;
  label: string;
  error?: string | undefined;
  hint?: string | undefined;
  children: (control: ControlProps) => ReactNode;
  className?: string | undefined;
};

/** Label above, hint or inline error below (`ui_rules.md` §4). The label names the control
 *  and the note below describes it, so a screen reader announces both in the right roles. */
function Field({ id, name, label, error, hint, children, className }: FieldProps): JSX.Element {
  const controlId = `${id}-${name}`;
  const note = error ?? hint;
  const noteId = note === undefined ? undefined : `${controlId}-note`;

  return (
    <div className={cn("min-w-0", className)}>
      <label htmlFor={controlId} className="mb-1 block text-xs font-medium text-foreground-muted">
        {label}
      </label>
      {children({ id: controlId, "aria-describedby": noteId })}
      {note === undefined ? null : (
        <p
          id={noteId}
          className={cn("mt-1 text-xs", error === undefined ? "text-foreground-subtle" : "text-danger")}
        >
          {note}
        </p>
      )}
    </div>
  );
}
