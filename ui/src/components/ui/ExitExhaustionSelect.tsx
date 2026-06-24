import type { ExitOnExhaustion } from "../../api/client";

const OPTIONS: { id: ExitOnExhaustion; label: string }[] = [
  { id: "kill_switch", label: "Kill switch (block all traffic)" },
  { id: "blocked", label: "Blocked (this app only)" },
  { id: "direct", label: "Direct (no tunnel)" },
];

export function ExitExhaustionSelect({
  value,
  onChange,
  disabled,
}: {
  value: ExitOnExhaustion;
  onChange: (v: ExitOnExhaustion) => void;
  disabled?: boolean;
}) {
  return (
    <select
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value as ExitOnExhaustion)}
      className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm disabled:opacity-50"
    >
      {OPTIONS.map((o) => (
        <option key={o.id} value={o.id}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
