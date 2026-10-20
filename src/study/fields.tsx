import type { ReactNode } from "react";

export function StudyField({ id, label, children, hint }: { id: string; label: string; children: ReactNode; hint?: string }) {
  return <div className="field"><label htmlFor={id}>{label}</label>{children}{hint && <p id={`${id}-hint`} className="muted field-hint">{hint}</p>}</div>;
}

export function localDateTime(value = new Date()): string {
  const local = new Date(value.getTime() - value.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

export function utcDateTime(value: string): string | null {
  if (!value.trim()) return null;
  const time = new Date(value);
  return Number.isNaN(time.getTime()) ? null : time.toISOString();
}

export function displayTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

export function localTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

export const optionalText = (value: string) => value.trim() ? value : null;
