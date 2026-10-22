import { useEffect, useState } from "react";
import * as api from "../platform/client";

export default function BackupPanel({ generation, busy, onPreview, onExport }: { generation: string; busy: boolean; onPreview: () => void; onExport: () => void }) {
  const [status, setStatus] = useState<api.BackupStatus | null>(null);
  const [error, setError] = useState("");
  const [refresh, setRefresh] = useState(0);
  useEffect(() => {
    let cancelled = false; let timer: number | undefined;
    async function read() {
      try {
        const next = await api.backupStatus(); if (cancelled) return;
        setStatus(next); setError("");
        if (next.state === "checking" || next.state === "running") timer = window.setTimeout(() => void read(), 1500);
      } catch (cause) { if (!cancelled) setError(api.normalizeError(cause).message); }
    }
    void read(); return () => { cancelled = true; window.clearTimeout(timer); };
  }, [generation, refresh]);
  async function retry() {
    try { setStatus(await api.retryBackup()); setError(""); setRefresh((value) => value + 1); } catch (cause) { setError(api.normalizeError(cause).message); }
  }
  return <section className="backup-panel" aria-label="Local recovery status"><h3>Local recovery</h3>
    {status && <p role="status">{status.state === "ready" ? `Latest automatic recovery copy: ${status.latest_successful_day ?? "available"}.` : status.state === "running" || status.state === "checking" ? "Checking local recovery in the background…" : status.state === "warning" ? "Automatic recovery needs attention. Your saved work remains available." : "Automatic recovery will run after saved work."}</p>}
    {(error || status?.warning) && <p role="alert" className="field-error">{error || status?.warning}</p>}
    {status?.latest_successful_day && <p className="muted">Verified recovery copies: {status.snapshot_count}. Latest verified date: {status.latest_successful_day}.</p>}
    <p className="muted">Recovery copies stay on this device. Export records to a separate device or location for protection from device loss.</p>
    <div className="button-row"><button type="button" className="secondary small" disabled={busy || status?.state === "running"} onClick={() => void retry()}>Check recovery now</button><button type="button" className="secondary small" disabled={busy} onClick={onPreview}>Preview recovery</button><button type="button" className="secondary small" disabled={busy} onClick={onExport}>Export recovery copy</button></div>
  </section>;
}
