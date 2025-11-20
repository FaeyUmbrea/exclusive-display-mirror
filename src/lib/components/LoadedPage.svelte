<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import Preview from "./Preview.svelte";
  import CaptureSourceSelector from "./CaptureSourceSelector.svelte";
  import OutputDeviceSelector from "./OutputDeviceSelector.svelte";
  import ResolutionSelector from "./ResolutionSelector.svelte";
  import Toast from "$lib/Toast.svelte";

  interface MonitorInfo {
    name: string;
    device_name: string;
    stable_id?: string;
    adapter_id?: number;
    is_connected: boolean;
    is_stale: boolean;
  }

  interface DisplayModeInfo {
    width: number;
    height: number;
    refresh_rate: number;
    index: number;
  }

  // App state management
  type AppStep =
    | "output-selection"
    | "resolution-selection"
    | "initializing"
    | "capture-setup";

  let currentStep: AppStep = "output-selection";
  let selectedMonitor: MonitorInfo | null = null;
  let selectedMode: DisplayModeInfo | null = null;
  let initializingMessage: string = "";

  let unlisteners: UnlistenFn[] = [];
  let status: string = "";
  let frameBufferStatus = { depth: 0, width: 0, height: 0, hasFrames: false };
  let frameStats = {
    frames_produced: 0,
    frames_consumed: 0,
    frames_dropped: 0,
  };
  let frameCheckInterval: number | null = null;

  // Simple toast list
  type ToastItem = {
    id: string;
    message: string;
    kind?: "info" | "success" | "warn" | "error";
  };
  let toasts: ToastItem[] = [];

  function pushToast(message: string, kind: ToastItem["kind"] = "info") {
    const id = String(Date.now()) + Math.floor(Math.random() * 1000);
    toasts = [...toasts, { id, message, kind }];
    setTimeout(() => {
      toasts = toasts.filter((t) => t.id !== id);
    }, 5000);
  }

  onMount(() => {
    // Listen to backend events for visibility into render attempts
    (async () => {
      unlisteners.push(
        await listen("render_attempt", (e) => {
          status = `Attempt: ${e.payload}`;
          console.log(status);
        }),
      );
      unlisteners.push(
        await listen("render_success", (e) => {
          status = `Success: ${e.payload}`;
          console.log(status);
        }),
      );
      unlisteners.push(
        await listen("render_error", (e) => {
          status = `Error: ${e.payload}`;
          console.error(status);
        }),
      );
    })();
  });

  onDestroy(() => {
    unlisteners.forEach((fn) => fn());
    if (frameCheckInterval) clearInterval(frameCheckInterval);
  });

  function handleMonitorSelected(event: CustomEvent<MonitorInfo>) {
    selectedMonitor = event.detail;
    console.log("Monitor selected:", selectedMonitor);
    currentStep = "resolution-selection";
  }

  function handleBackToOutputSelection() {
    currentStep = "output-selection";
    selectedMonitor = null;
  }

  async function handleRestoreSettings(
    event: CustomEvent<{
      monitor: MonitorInfo;
      mode: DisplayModeInfo;
      vsync: boolean;
    }>
  ) {
    const { monitor, mode, vsync } = event.detail;
    selectedMonitor = monitor;
    selectedMode = mode;
    
    // Show loading state
    currentStep = "initializing";
    initializingMessage = "Restoring display settings...";

    try {
      // 1. Initialize display configuration to get available modes
      console.log("Restoring: Initializing display config...");
      const modes = (await invoke("initialize_display_configuration", {
        monitor: selectedMonitor,
      })) as DisplayModeInfo[];

      // 2. Find the matching mode index
      const matchedMode = modes.find(
        (m) =>
          m.width === mode.width &&
          m.height === mode.height &&
          Math.abs(m.refresh_rate - mode.refresh_rate) < 1 // Allow small refresh rate variance
      );

      if (!matchedMode) {
        throw new Error(
          `Saved mode ${mode.width}x${mode.height}@${mode.refresh_rate}Hz not supported by this display.`
        );
      }

      console.log("Restoring: Found matching mode", matchedMode);
      selectedMode = matchedMode; // Update with correct index

      // 3. Apply configuration
      initializingMessage = "Applying display settings...";
      await invoke("apply_display_configuration", {
        modeIndex: matchedMode.index,
      });

      // 4. Proceed to standard completion
      await handleConfigurationComplete(
        new CustomEvent("configurationComplete", {
          detail: {
            monitor: selectedMonitor,
            mode: selectedMode,
            vsync: vsync,
          },
        })
      );
    } catch (err) {
      console.error("Failed to restore settings:", err);
      status = `Failed to restore settings: ${err}`;
      initializingMessage = `Error: ${err}`;
      // Optional: Go back to output selection after error?
      setTimeout(() => {
        currentStep = "output-selection";
        pushToast(`Restore failed: ${err}`, "error");
      }, 3000);
    }
  }

  async function handleConfigurationComplete(
    event: CustomEvent<{
      monitor: MonitorInfo;
      mode: DisplayModeInfo;
      vsync: boolean;
    }>,
  ) {
    selectedMonitor = event.detail.monitor;
    selectedMode = event.detail.mode;
    const vsync = event.detail.vsync;

    console.log("Configuration complete:", {
      monitor: selectedMonitor,
      mode: selectedMode,
      vsync,
    });

    // Show loading state to prevent user interaction
    currentStep = "initializing";

    // Initialize OBS with the selected display resolution and refresh rate
    try {
      initializingMessage = "Initializing OBS...";
      console.log(
        "Initializing OBS with",
        selectedMode.width,
        "x",
        selectedMode.height,
        "@",
        selectedMode.refresh_rate,
        "fps",
      );

      const obsResult = await invoke("initialize_obs", {
        width: selectedMode.width,
        height: selectedMode.height,
        fps: selectedMode.refresh_rate,
        captureMode: "cpu", // GPU mode removed, always use CPU
      });

      console.log("OBS initialization result:", obsResult);
      initializingMessage = "Starting renderer...";
    } catch (err) {
      console.error("Failed to initialize OBS:", err);
      status = `Failed to initialize OBS: ${err}`;
      // Stay on initializing screen with error
      initializingMessage = `Error: ${err}`;
      return;
    }

    // Start the renderer with the configured display
    try {
      const result = await invoke("start_renderer", {
        params: {
          monitor: selectedMonitor,
          vsync: vsync,
          max_fps: selectedMode.refresh_rate,
        },
      });
      console.log("Renderer started:", result);

      initializingMessage = "Renderer started successfully";

      // Start monitoring frame buffer
      startFrameMonitoring();

      // Proceed to capture setup
      setTimeout(() => {
        currentStep = "capture-setup";
      }, 500);
    } catch (err) {
      console.error("Failed to start renderer:", err);
      status = `Failed to start renderer: ${err}`;
      initializingMessage = `Error: ${err}`;
      return;
    }
  }

  async function checkFrameBuffer() {
    try {
      const [depth, dimensions] = (await invoke("get_frame_buffer_status")) as [
        number,
        [number, number] | null,
      ];
      frameBufferStatus = {
        depth,
        width: dimensions?.[0] ?? 0,
        height: dimensions?.[1] ?? 0,
        hasFrames: depth > 0,
      };

      // Also fetch frame stats
      const stats = (await invoke("get_frame_stats")) as {
        frames_produced: number;
        frames_consumed: number;
        frames_dropped: number;
      };
      frameStats = stats;

      if (depth > 0 && !status.includes("Frames flowing")) {
        status = `✓ Frames flowing: ${frameBufferStatus.width}x${frameBufferStatus.height}`;
        console.log("Frame buffer active:", frameBufferStatus);
      }
    } catch (err) {
      console.error("Failed to check frame buffer:", err);
    }
  }

  function startFrameMonitoring() {
    // Clear any existing interval
    if (frameCheckInterval) clearInterval(frameCheckInterval);

    // Check frame buffer every second
    frameCheckInterval = setInterval(checkFrameBuffer, 1000);

    // Do an immediate check
    checkFrameBuffer();
  }
</script>

<div class="app-container">
  <div class="toast-outer" aria-live="polite">
    {#each toasts as t (t.id)}
      <Toast id={t.id} message={t.message} kind={t.kind} />
    {/each}
  </div>
  {#if currentStep === "output-selection"}
    <OutputDeviceSelector 
      on:monitorSelected={handleMonitorSelected} 
      on:restoreSettings={handleRestoreSettings}
    />
  {:else if currentStep === "resolution-selection" && selectedMonitor}
    <ResolutionSelector
      {selectedMonitor}
      on:back={handleBackToOutputSelection}
      on:configurationComplete={handleConfigurationComplete}
    />
  {:else if currentStep === "initializing"}
    <div class="initializing-container">
      <div class="initializing-card">
        <div class="spinner"></div>
        <h2>Initializing System</h2>
        <p class="initializing-message">{initializingMessage}</p>
        <div class="progress-dots">
          <span class="dot"></span>
          <span class="dot"></span>
          <span class="dot"></span>
        </div>
      </div>
    </div>
  {:else if currentStep === "capture-setup"}
    <div class="capture-setup-container">
      <div class="status-banner">
        <svg class="icon" viewBox="0 0 20 20" fill="currentColor">
          <path
            fill-rule="evenodd"
            d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
            clip-rule="evenodd"
          />
        </svg>
        <div>
          <h3>Display Activated</h3>
          <p>
            {selectedMonitor?.name} is now active at {selectedMode?.width}×{selectedMode?.height}
            @ {selectedMode?.refresh_rate}Hz
          </p>
        </div>
      </div>

      <CaptureSourceSelector />

      <!-- Frame Statistics Panel -->
      <div class="frame-stats-panel">
        <div class="stats-header">
          <svg class="stats-icon" viewBox="0 0 20 20" fill="currentColor">
            <path
              d="M2 11a1 1 0 011-1h2a1 1 0 011 1v5a1 1 0 01-1 1H3a1 1 0 01-1-1v-5z"
            />
            <path
              d="M8 7a1 1 0 011-1h2a1 1 0 011 1v9a1 1 0 01-1 1H9a1 1 0 01-1-1V7z"
            />
            <path
              d="M14 4a1 1 0 011-1h2a1 1 0 011 1v12a1 1 0 01-1 1h-2a1 1 0 01-1-1V4z"
            />
          </svg>
          <span>Frame Statistics</span>
        </div>
        <div class="stats-grid">
          <div class="stat-item">
            <span class="stat-label">Produced</span>
            <span class="stat-value"
              >{frameStats.frames_produced.toLocaleString()}</span
            >
          </div>
          <div class="stat-item">
            <span class="stat-label">Rendered</span>
            <span class="stat-value"
              >{frameStats.frames_consumed.toLocaleString()}</span
            >
          </div>
          <div
            class="stat-item"
            class:stat-warning={frameStats.frames_dropped > 0}
          >
            <span class="stat-label">Dropped</span>
            <span class="stat-value">
              {frameStats.frames_dropped.toLocaleString()}
              {#if frameStats.frames_dropped > 0}
                <svg
                  class="warning-icon"
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path
                    fill-rule="evenodd"
                    d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                    clip-rule="evenodd"
                  />
                </svg>
              {/if}
            </span>
          </div>
        </div>
        {#if frameStats.frames_produced > 0}
          <div class="stats-footer">
            Drop rate: {(
              (frameStats.frames_dropped / frameStats.frames_produced) *
              100
            ).toFixed(1)}%
          </div>
        {/if}
      </div>

      {#if status}
        <div class="status-message">
          {status}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .app-container {
    min-height: 100vh;
    background: var(--bg-primary);
    padding: 1.5rem;
  }

  .initializing-container {
    min-height: calc(100vh - 3rem);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .initializing-card {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 3rem;
    text-align: center;
    max-width: 400px;
  }

  .spinner {
    width: 3rem;
    height: 3rem;
    border: 4px solid var(--border-color);
    border-top-color: var(--accent-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 1.5rem;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .initializing-card h2 {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 0.75rem 0;
  }

  .initializing-message {
    color: var(--text-secondary);
    font-size: 1rem;
    margin: 0 0 1.5rem 0;
  }

  .progress-dots {
    display: flex;
    justify-content: center;
    gap: 0.5rem;
  }

  .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--accent-primary);
    animation: pulse 1.4s ease-in-out infinite;
  }

  .dot:nth-child(2) {
    animation-delay: 0.2s;
  }

  .dot:nth-child(3) {
    animation-delay: 0.4s;
  }

  @keyframes pulse {
    0%,
    80%,
    100% {
      opacity: 0.3;
      transform: scale(0.8);
    }
    40% {
      opacity: 1;
      transform: scale(1);
    }
  }

  .capture-setup-container {
    max-width: 900px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .status-banner {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem 1.25rem;
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.3);
    border-radius: var(--radius-lg);
    color: var(--success);
  }

  .status-banner .icon {
    width: 1.5rem;
    height: 1.5rem;
    flex-shrink: 0;
  }

  .status-banner h3 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 0.125rem 0;
  }

  .status-banner p {
    font-size: 0.8125rem;
    margin: 0;
    opacity: 0.9;
  }

  .status-message {
    margin-top: 0.5rem;
    padding: 0.75rem 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 0.875rem;
    text-align: center;
  }

  /* Frame Statistics Panel */
  .frame-stats-panel {
    padding: 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
  }

  .stats-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 500;
    font-size: 0.8125rem;
    margin-bottom: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stats-icon {
    width: 1rem;
    height: 1rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.75rem;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    padding: 0.75rem;
    background: var(--bg-surface);
    border-radius: var(--radius-sm);
  }

  .stat-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stat-value {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .stat-warning .stat-label {
    color: var(--warning);
  }

  .stat-warning .stat-value {
    color: var(--warning);
  }

  .warning-icon {
    width: 1rem;
    height: 1rem;
  }

  .stats-footer {
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
    font-size: 0.75rem;
    color: var(--text-muted);
    text-align: right;
  }

  .toast-outer {
    position: fixed;
    top: 1rem;
    right: 1rem;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
</style>
