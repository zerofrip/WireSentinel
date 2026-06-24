import type { ReactNode } from "react";

export function Panel({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`bg-sentinel-panel rounded-lg border border-slate-700 p-4 ${className}`.trim()}
    >
      {children}
    </div>
  );
}
