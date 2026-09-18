import "./style.css";

async function init() {
  const appElement = document.querySelector<HTMLDivElement>("#app");
  if (!appElement) return;

  let systemStatus = "Connecting to Sonora Core...";

  try {
    const { invoke } = await import("@tauri-apps/api/core");
    systemStatus = await invoke<string>("get_system_status");
  } catch (_e) {
    systemStatus = "Sonora Core Initialized (Web / Preview Mode)";
  }

  appElement.innerHTML = `
    <div>
      <h1>Sonora Audio Canvas</h1>
      <span class="status-badge">v0.1.0 Bootstrap</span>
      <div class="card">
        <h3>System Status</h3>
        <p id="core-status"><strong>${systemStatus}</strong></p>
        <hr style="border-color: var(--border-subtle); margin: 1rem 0;" />
        <ul class="system-details">
          <li><strong>Architecture:</strong> Decoupled Core + Tauri 2 Host</li>
          <li><strong>Audio Engine:</strong> CPAL (Real-Time Synchronous Render Thread)</li>
          <li><strong>Decoder:</strong> Symphonia (Gapless Multi-format)</li>
          <li><strong>Metadata:</strong> Lofty Audio Tagging</li>
          <li><strong>Library Storage:</strong> SQLite (WAL Mode) + FTS5 Full-Text Search</li>
          <li><strong>Async Runtime:</strong> Tokio (Restricted to non-realtime daemon tasks)</li>
        </ul>
      </div>
    </div>
  `;
}

init();
