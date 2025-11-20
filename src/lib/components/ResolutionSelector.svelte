<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { createEventDispatcher } from "svelte";

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

  export let selectedMonitor: MonitorInfo;

  const dispatch = createEventDispatcher();

  let availableModes: DisplayModeInfo[] = [];
  let selectedMode: DisplayModeInfo | null = null;
  let isLoading = true;
  let error = "";
  let isApplying = false;
  let searchQuery = "";
  let vsyncEnabled = true; // Default to enabled

  $: uniqueModes = availableModes.reduce((acc, mode) => {
    const key = `${mode.width}x${mode.height}@${mode.refresh_rate}`;
    if (!acc.has(key)) {
      acc.set(key, mode);
    }
    return acc;
  }, new Map<string, DisplayModeInfo>());

  $: deduplicatedModes = Array.from(uniqueModes.values());

  $: filteredModes = deduplicatedModes.filter((mode) => {
    if (!searchQuery) return true;
    const query = searchQuery.toLowerCase();
    const resolutionStr = `${mode.width}x${mode.height}`;
    const refreshStr = `${mode.refresh_rate}hz`;
    const combinedStr = `${resolutionStr} ${refreshStr} ${mode.width} ${mode.height} ${mode.refresh_rate}`;
    return combinedStr.toLowerCase().includes(query);
  });

  $: groupedModes = filteredModes.reduce(
    (acc, mode) => {
      const key = `${mode.width}x${mode.height}`;
      if (!acc[key]) {
        acc[key] = [];
      }
      acc[key].push(mode);
      return acc;
    },
    {} as Record<string, DisplayModeInfo[]>,
  );

  $: resolutions = Object.keys(groupedModes).sort((a, b) => {
    const [aW, aH] = a.split("x").map(Number);
    const [bW, bH] = b.split("x").map(Number);
    return bW * bH - aW * aH;
  });

  $: totalModes = deduplicatedModes.length;
  $: filteredCount = filteredModes.length;

  async function initializeConfiguration() {
    try {
      isLoading = true;
      error = "";
      availableModes = await invoke("initialize_display_configuration", {
        monitor: selectedMonitor,
      });
      if (availableModes.length === 0) {
        error = "No display modes available for this monitor.";
      }
    } catch (e) {
      console.error("Failed to initialize display configuration:", e);
      error = `Failed to initialize: ${e}`;
    } finally {
      isLoading = false;
    }
  }

  function selectMode(mode: DisplayModeInfo) {
    selectedMode = mode;
  }

  async function confirmSelection() {
    if (!selectedMode) return;
    try {
      isApplying = true;
      error = "";
      
      // Save settings for "Use Last Settings" feature
      localStorage.setItem("last_resolution_width", selectedMode.width.toString());
      localStorage.setItem("last_resolution_height", selectedMode.height.toString());
      localStorage.setItem("last_resolution_rate", selectedMode.refresh_rate.toString());
      localStorage.setItem("last_vsync", vsyncEnabled.toString());

      await invoke("apply_display_configuration", {
        modeIndex: selectedMode.index,
      });
      dispatch("configurationComplete", {
        monitor: selectedMonitor,
        mode: selectedMode,
        vsync: vsyncEnabled,
      });
    } catch (e) {
      console.error("Failed to apply display configuration:", e);
      error = `Failed to apply configuration: ${e}`;
    } finally {
      isApplying = false;
    }
  }

  function goBack() {
    invoke("cancel_display_configuration").catch(console.error);
    dispatch("back");
  }

  function clearSearch() {
    searchQuery = "";
  }

  initializeConfiguration();
</script>

<div class="selector-container">
  <div class="header">
    <h2>Select Display Mode</h2>
    <p class="subtitle">
      For {selectedMonitor.name}
    </p>
  </div>

  {#if error && totalModes === 0}
    <div class="content-centered">
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
    </div>
  {/if}

  {#if isLoading}
    <div class="content-centered">
      <div class="spinner"></div>
      <p>Loading available display modes...</p>
    </div>
  {:else if isApplying}
    <div class="content-centered">
      <div class="spinner"></div>
      <p>Activating display...</p>
      <p class="loading-detail">This may take a few moments</p>
    </div>
  {:else if totalModes > 0}
    <div class="controls">
      <div class="search-bar">
        <svg viewBox="0 0 20 20" fill="currentColor">
          <path
            fill-rule="evenodd"
            d="M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z"
            clip-rule="evenodd"
          />
        </svg>
        <input
          type="text"
          placeholder="Search resolutions or refresh rates..."
          bind:value={searchQuery}
        />
        {#if searchQuery}
          <button
            class="clear-btn"
            on:click={clearSearch}
            aria-label="Clear search"
          >
            <svg viewBox="0 0 20 20" fill="currentColor">
              <path
                fill-rule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                clip-rule="evenodd"
              />
            </svg>
          </button>
        {/if}
      </div>
      <div class="mode-count">
        {#if searchQuery}
          Showing {filteredCount} of {totalModes} modes
        {:else}
          {totalModes} available modes
        {/if}
      </div>
    </div>

    {#if filteredCount === 0}
      <div class="content-centered">
        <p>No modes match your search</p>
        <button class="btn-secondary small" on:click={clearSearch}
          >Clear Search</button
        >
      </div>
    {:else}
      <div class="modes-list">
        {#each resolutions as resolution}
          <div class="resolution-group">
            <h3 class="resolution-title">{resolution}</h3>
            <div class="refresh-rate-options">
              {#each groupedModes[resolution] as mode}
                <button
                  class="refresh-rate-btn"
                  class:selected={selectedMode?.index === mode.index}
                  on:click={() => selectMode(mode)}
                >
                  <span class="refresh-rate">{mode.refresh_rate}</span>
                  <span class="hz">Hz</span>
                </button>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  <div class="action-footer">
    <button class="btn-secondary" on:click={goBack}>
      <svg viewBox="0 0 20 20" fill="currentColor">
        <path
          fill-rule="evenodd"
          d="M12.707 5.293a1 1 0 010 1.414L9.414 10l3.293 3.293a1 1 0 01-1.414 1.414l-4-4a1 1 0 010-1.414l4-4a1 1 0 011.414 0z"
          clip-rule="evenodd"
        />
      </svg>
      Back
    </button>

    <label class="vsync-toggle">
      <input type="checkbox" bind:checked={vsyncEnabled} />
      <span class="label-text">Enable VSync</span>
    </label>

    {#if totalModes > 0}
      <button
        class="btn-primary"
        disabled={!selectedMode || isApplying}
        on:click={confirmSelection}
      >
        Activate Display
        <svg viewBox="0 0 20 20" fill="currentColor">
          <path
            fill-rule="evenodd"
            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
            clip-rule="evenodd"
          />
        </svg>
      </button>
    {/if}
  </div>
</div>

<style>
  .selector-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    max-height: calc(100vh - 4rem);
    width: 100%;
    max-width: 680px;
    margin: 2rem auto;
    background: var(--bg-surface);
    border-radius: var(--radius-xl);
    border: 1px solid var(--border-color);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }

  .header {
    text-align: center;
    padding: 1.5rem 2rem 1rem;
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .header h2 {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 0.25rem 0;
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
    margin: 1rem;
  }

  .error-banner svg {
    width: 1.25rem;
    height: 1.25rem;
    flex-shrink: 0;
  }

  .content-centered {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    min-height: 0;
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

  .loading-detail {
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  .controls {
    padding: 1rem 2rem 0;
    flex-shrink: 0;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    margin-bottom: 0.75rem;
  }

  .search-bar svg {
    width: 1.125rem;
    height: 1.125rem;
    color: var(--text-muted);
  }

  .search-bar input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 0.9375rem;
    outline: none;
  }

  .clear-btn {
    background: transparent;
    border: none;
    padding: 0.25rem;
    cursor: pointer;
    color: var(--text-muted);
    display: flex;
  }

  .mode-count {
    text-align: center;
    font-size: 0.8125rem;
    color: var(--text-muted);
    padding-bottom: 1rem;
  }

  .modes-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 2rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 0;
  }

  .resolution-group {
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    padding: 1rem 1.5rem;
    border: 1px solid var(--border-color);
  }

  .resolution-title {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 1rem 0;
  }

  .refresh-rate-options {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .refresh-rate-btn {
    padding: 0.5rem 1rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s;
    font-weight: 600;
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
  }

  .refresh-rate-btn:hover {
    border-color: var(--border-light);
    background: var(--bg-hover);
  }

  .refresh-rate-btn.selected {
    border-color: var(--accent-primary);
    background: var(--accent-primary-faded);
    color: var(--accent-primary);
  }

  .refresh-rate {
    font-size: 1rem;
  }

  .hz {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .refresh-rate-btn.selected .hz {
    color: var(--accent-primary);
  }

  .action-footer {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 1.5rem 2rem;
    border-top: 1px solid var(--border-color);
    background: var(--bg-surface);
    flex-shrink: 0;
  }

  .btn-primary,
  .btn-secondary {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1.25rem;
    border: none;
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-secondary.small {
    padding: 0.5rem 1rem;
    font-size: 0.8125rem;
  }

  .btn-primary {
    background: var(--accent-primary);
    color: white;
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
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
  }

  .btn-secondary:hover {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .btn-primary svg,
  .btn-secondary svg {
    width: 1rem;
    height: 1rem;
  }

  .vsync-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    color: var(--text-primary);
    font-size: 0.875rem;
    font-weight: 500;
    user-select: none;
  }

  .vsync-toggle input[type="checkbox"] {
    width: 1rem;
    height: 1rem;
    accent-color: var(--accent-primary);
    cursor: pointer;
  }
</style>
