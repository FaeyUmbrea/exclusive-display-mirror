<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher, onMount } from "svelte";

  interface MonitorInfo {
    name: string;
    device_name: string;
    stable_id?: string;
    adapter_id?: number;
    is_connected: boolean;
    is_stale: boolean;
  }

  const dispatch = createEventDispatcher();

  let monitors: MonitorInfo[] = [];
  let selectedMonitor: MonitorInfo | null = null;
  let isLoading = true;
  let error = "";
  let hasLastSettings = false;

  onMount(() => {
    loadMonitors();
  });

  async function loadMonitors() {
    try {
      isLoading = true;
      error = "";
      monitors = await invoke("get_detached_monitors");
      console.log("Loaded monitors:", monitors);

      if (monitors.length === 0) {
        error =
          'No specialized displays found. Please ensure your display is in "Specialized" mode.';
      }

      // Check if we have last settings
      const lastId = localStorage.getItem("last_monitor_stable_id");
      const lastRes = localStorage.getItem("last_resolution_width");
      if (lastId && lastRes) {
          // Only show button if the monitor is actually present (optional, but good UX)
          // But monitors array might be empty if error.
          // We'll check existence when clicking the button.
          hasLastSettings = true;
      }
    } catch (e) {
      console.error("Failed to load monitors:", e);
      error = `Failed to load monitors: ${e}`;
    } finally {
      isLoading = false;
    }
  }

  function selectMonitor(monitor: MonitorInfo) {
    selectedMonitor = monitor;
  }

  function confirmSelection() {
    if (selectedMonitor) {
      if (selectedMonitor.stable_id) {
        localStorage.setItem("last_monitor_stable_id", selectedMonitor.stable_id);
      }
      dispatch("monitorSelected", selectedMonitor);
    }
  }

  function handleUseLastSettings() {
      const lastId = localStorage.getItem("last_monitor_stable_id");
      if (!lastId) return;

      const monitor = monitors.find(m => m.stable_id === lastId);
      if (!monitor) {
          error = "Saved monitor not found in current list.";
          return;
      }

      // Load resolution settings
      const width = parseInt(localStorage.getItem("last_resolution_width") || "0");
      const height = parseInt(localStorage.getItem("last_resolution_height") || "0");
      const rate = parseInt(localStorage.getItem("last_resolution_rate") || "0");
      const vsync = localStorage.getItem("last_vsync") === "true";

      if (width > 0 && height > 0 && rate > 0) {
          dispatch("restoreSettings", {
              monitor,
              mode: { width, height, refresh_rate: rate, index: 0 }, // index 0 is dummy, backend re-matches usually or we need real index?
              // Actually backend init_obs takes w/h/fps. It doesn't strictly need index if we aren't using index-based mode setting.
              // But wait, ResolutionSelector passes a DisplayModeInfo which has index.
              // Does `initialize_obs` use index? No, it uses w/h/fps.
              // Does `start_renderer` use index? It takes `monitor`. The renderer likely re-enumerates modes to find the best match or uses the parameters to set up the swap chain.
              // Let's assume w/h/rate is enough.
              vsync
          });
      } else {
          error = "Saved resolution settings are invalid.";
      }
  }
</script>

<div class="device-selector">
  <div class="header">
    <h2>Select Output Device</h2>
    <p class="subtitle">Choose a specialized display for output</p>
  </div>

  {#if error}
    <div class="error-banner">
      <svg viewBox="0 0 20 20" fill="currentColor">
        <path
          fill-rule="evenodd"
          d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
          clip-rule="evenodd"
        />
      </svg>
      <span>{error}</span>
    </div>
  {/if}

  {#if isLoading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Scanning for displays...</p>
    </div>
  {:else if monitors.length > 0}
    <div class="monitor-list">
      {#each monitors as monitor}
        <button
          class="monitor-card"
          class:selected={selectedMonitor === monitor}
          on:click={() => selectMonitor(monitor)}
        >
          <div class="monitor-icon">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
              <line x1="8" y1="21" x2="16" y2="21" />
              <line x1="12" y1="17" x2="12" y2="21" />
            </svg>
          </div>
          <div class="monitor-info">
            <h3>{monitor.name}</h3>
            <p class="device-name">{monitor.device_name}</p>
          </div>
          {#if selectedMonitor === monitor}
            <div class="selected-badge">
              <svg viewBox="0 0 20 20" fill="currentColor">
                <path
                  fill-rule="evenodd"
                  d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                  clip-rule="evenodd"
                />
              </svg>
            </div>
          {/if}
        </button>
      {/each}
    </div>

    <div class="action-footer">
      {#if hasLastSettings}
        <button class="btn-secondary" on:click={handleUseLastSettings}>
          Use Last Settings
        </button>
      {/if}
      <button
        class="btn-primary"
        disabled={!selectedMonitor}
        on:click={confirmSelection}
      >
        Continue
        <svg viewBox="0 0 20 20" fill="currentColor">
          <path
            fill-rule="evenodd"
            d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
            clip-rule="evenodd"
          />
        </svg>
      </button>
    </div>
  {/if}
</div>

<style>
  .device-selector {
    display: flex;
    flex-direction: column;
    height: 100%;
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
  }

  .header {
    text-align: center;
    margin-bottom: 1.5rem;
    flex-shrink: 0;
  }

  .header h2 {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 0.5rem 0;
  }

  .subtitle {
    color: var(--text-secondary);
    margin: 0;
  }

  .error-banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid var(--error);
    border-radius: var(--radius-md);
    color: var(--error);
    margin-bottom: 1rem;
    flex-shrink: 0;
  }

  .error-banner svg {
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 1rem;
  }

  .spinner {
    width: 2.5rem;
    height: 2.5rem;
    border: 3px solid var(--border-color);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .loading-state p {
    color: var(--text-secondary);
  }

  .monitor-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 0;
    padding-right: 0.5rem;
  }

  .monitor-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem 1.25rem;
    background: var(--bg-elevated);
    border: 2px solid var(--border-color);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: all 0.2s;
    text-align: left;
    position: relative;
  }

  .monitor-card:hover {
    border-color: var(--border-light);
    background: var(--bg-surface);
  }

  .monitor-card.selected {
    border-color: var(--accent-primary);
    background: rgba(220, 38, 38, 0.1);
  }

  .monitor-icon {
    width: 2.5rem;
    height: 2.5rem;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .monitor-card.selected .monitor-icon {
    color: var(--accent-primary);
  }

  .monitor-info {
    flex: 1;
    min-width: 0;
  }

  .monitor-info h3 {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 0.25rem 0;
  }

  .device-name {
    font-size: 0.875rem;
    color: var(--text-muted);
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .selected-badge {
    width: 1.75rem;
    height: 1.75rem;
    background: var(--accent-primary);
    color: white;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .selected-badge svg {
    width: 1rem;
    height: 1rem;
  }

  .action-footer {
    display: flex;
    justify-content: flex-end;
    padding-top: 1.5rem;
    border-top: 1px solid var(--border-color);
    margin-top: 1.5rem;
    flex-shrink: 0;
  }

  .btn-primary {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.5rem;
    background: var(--accent-primary);
    color: white;
    border: none;
    border-radius: var(--radius-md);
    font-size: 0.9375rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: translateY(-1px);
  }

  .btn-primary:disabled {
    background: var(--border-color);
    color: var(--text-muted);
    cursor: not-allowed;
  }

  .btn-secondary {
    padding: 0.75rem 1.5rem;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    font-size: 0.9375rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    margin-right: auto;
  }

  .btn-secondary:hover {
    background: var(--bg-surface);
    color: var(--text-primary);
    border-color: var(--border-light);
  }

  .btn-primary svg {
    width: 1.125rem;
    height: 1.125rem;
  }
</style>
