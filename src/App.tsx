import { useEffect, useState } from "react";
import { getAppStatus, type AppStatus } from "./platform/client";
import "./App.css";

type Connection =
  | { state: "loading" }
  | { state: "ready"; status: AppStatus }
  | { state: "error"; message: string };

function App() {
  const [connection, setConnection] = useState<Connection>({ state: "loading" });
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let active = true;
    setConnection({ state: "loading" });
    getAppStatus().then(
      (status) => active && setConnection({ state: "ready", status }),
      (error: unknown) => active && setConnection({
        state: "error",
        message: error instanceof Error ? error.message : "The desktop connection could not be opened. Please try again.",
      }),
    );
    return () => { active = false; };
  }, [attempt]);

  return (
    <main className="work-shell">
      <header className="app-header">
        <p className="eyebrow">Your local workspace</p>
        <h1>Ascension</h1>
      </header>
      <section className="work-panel" aria-labelledby="work-heading">
        <h2 id="work-heading">Work</h2>
        <p>Your local workspace is being prepared.</p>
        {connection.state === "loading" && <p role="status">Connecting to the desktop app…</p>}
        {connection.state === "ready" && (
          <div className="connection-status" role="status">
            <p><strong>Desktop connection ready</strong></p>
            <p>{connection.status.app_name} · Record schema {connection.status.schema_version}</p>
          </div>
        )}
        {connection.state === "error" && (
          <div className="connection-error">
            <p role="alert">{connection.message}</p>
            <button type="button" onClick={() => setAttempt((value) => value + 1)}>Retry connection</button>
          </div>
        )}
      </section>
    </main>
  );
}

export default App;
