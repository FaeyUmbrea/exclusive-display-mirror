<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface CaptureSourceInfo {
    id: string;
    name: string;
    source_type: {
      type: 'monitor' | 'window' | 'game';
      monitor_id?: string;
      monitor_name?: string;
      window_title?: string;
      window_class?: string;
      executable?: string;
      color_space?: string;
      force_sdr?: boolean;
    };
  }

  let monitors: CaptureSourceInfo[] = [];
  let windows: CaptureSourceInfo[] = [];
  let selectedSource: CaptureSourceInfo | null = null;
  let isLoading = false;
  let error = '';
  let successMessage = '';
  let sourceType: 'monitor' | 'window' | 'game' = 'monitor';
  let searchTerm = '';

  $: filteredMonitors = monitors.filter(m => m.name.toLowerCase().includes(searchTerm.toLowerCase()));
  $: filteredWindows = windows.filter(w => w.name.toLowerCase().includes(searchTerm.toLowerCase()));

  // Capture settings
  let gameColorSpace: 'srgb' | '2100pq' = 'srgb';
  let forceSdr = true;

  onMount(async () => {
    await loadMonitors();
  });

  async function loadMonitors() {
    try {
      monitors = await invoke('list_monitors');
      console.log('Available monitors:', monitors);
    } catch (e) {
      console.error('Failed to load monitors:', e);
      error = `Failed to load monitors: ${e}`;
    }
  }

  async function loadWindows() {
    try {
      windows = await invoke('list_windows');
      console.log('Available windows:', windows);
      if (windows.length === 0) {
        error = 'No windows available';
      }
    } catch (e) {
      console.error('Failed to load windows:', e);
      error = `Failed to load windows: ${e}`;
    }
  }

  async function handleWindowClick(window: CaptureSourceInfo) {
    if (sourceType === 'game') {
      const gameSource: CaptureSourceInfo = {
        ...window,
        source_type: {
          ...window.source_type,
          type: 'game',
          color_space: gameColorSpace,
        }
      };
      await setSource(gameSource);
    } else {
      await setSource(window);
    }
  }

  async function setSource(source: CaptureSourceInfo) {
    isLoading = true;
    error = '';
    successMessage = '';

    // Apply current settings
    const sourceWithSettings = { ...source };
    if (sourceWithSettings.source_type.type !== 'game') {
        sourceWithSettings.source_type.force_sdr = forceSdr;
    }

    try {
      const result = await invoke('set_capture_source', { sourceInfo: sourceWithSettings });
      selectedSource = sourceWithSettings;
      successMessage = String(result);
      console.log('Source set:', result);
    } catch (e) {
      console.error('Failed to set source:', e);
      error = `Failed to set source: ${e}`;
    } finally {
      isLoading = false;
    }
  }

  function handleSourceTypeChange() {
    error = '';
    successMessage = '';
    if ((sourceType === 'window' || sourceType === 'game') && windows.length === 0) {
      loadWindows();
    }
  }
</script>

<div class="capture-selector">
  <div class="header">
    <h3>Capture Source</h3>

    <div class="source-type-tabs">
      <button
        class="tab"
        class:active={sourceType === "game"}
        on:click={() => {
          sourceType = "game";
          handleSourceTypeChange();
        }}
      >
        🎮 Game
      </button>
      <button
        class="tab"
        class:active={sourceType === "window"}
        on:click={() => {
          sourceType = "window";
          handleSourceTypeChange();
        }}
      >
        🪟 Window
      </button>
      <button
        class="tab"
        class:active={sourceType === "monitor"}
        on:click={() => {
          sourceType = "monitor";
          handleSourceTypeChange();
        }}
      >
        🖥️ Monitor
      </button>
    </div>

    {#if sourceType !== 'game'}
      <div class="settings-row">
        <label class="setting-toggle">
          <input type="checkbox" bind:checked={forceSdr} />
          <span>Force SDR</span>
        </label>
      </div>
    {:else}
      <div class="settings-row">
        <label class="setting-select">
          <span>Color Space:</span>
          <select bind:value={gameColorSpace}>
            <option value="srgb">sRGB</option>
            <option value="2100pq">2100pq (HDR)</option>
          </select>
        </label>
      </div>
    {/if}
  </div>

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
      placeholder="Search sources..."
      bind:value={searchTerm}
    />
  </div>

  {#if error}
    <div class="message error">{error}</div>
  {/if}

  {#if successMessage}
    <div class="message success">{successMessage}</div>
  {/if}

  <div class="source-list">
    {#if sourceType === "monitor"}
      {#if filteredMonitors.length === 0}
        <p class="no-sources">No monitors found</p>
      {:else}
        {#each filteredMonitors as monitor}
          <button
            class="source-item"
            class:selected={selectedSource?.id === String(monitor.id)}
            on:click={() => setSource(monitor)}
            disabled={isLoading}
          >
            <span class="source-name">{monitor.name}</span>
            {#if selectedSource?.id === String(monitor.id)}
              <span class="active-badge">Active</span>
            {/if}
          </button>
        {/each}
      {/if}
    {:else}
      <div class="list-header">
        <span
          >{sourceType === "game"
            ? "Available Games"
            : "Available Windows"}</span
        >
        <button class="refresh-btn" on:click={loadWindows} disabled={isLoading}>
          🔄 Refresh
        </button>
      </div>

      {#if filteredWindows.length === 0}
        <p class="no-sources">
          No {sourceType === "game" ? "games" : "windows"} found
        </p>
      {:else}
        {#each filteredWindows as window}
          <button
            class="source-item"
            class:selected={selectedSource?.id === window.id}
            on:click={() => handleWindowClick(window)}
            disabled={isLoading}
          >
            <span class="source-name">{window.name}</span>
            {#if selectedSource?.id === window.id}
              <span class="active-badge">Active</span>
            {/if}
          </button>
        {/each}
      {/if}
    {/if}
  </div>

  {#if isLoading}
    <div class="loading-overlay">
      <div class="spinner"></div>
    </div>
  {/if}
</div>

<style>
  .capture-selector {
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    max-height: 500px;
    overflow: hidden;
    position: relative;
  }

  .header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  h3 {
    margin: 0 0 0.75rem 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .settings-row {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-color);
    display: flex;
    align-items: center;
  }

  .setting-toggle, .setting-select {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--text-primary);
    cursor: pointer;
    user-select: none;
  }

  .setting-toggle input[type="checkbox"] {
    width: 1rem;
    height: 1rem;
    accent-color: var(--accent-primary);
  }

  .setting-select select {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: var(--radius-sm);
    padding: 0.25rem 0.5rem;
    font-size: 0.875rem;
    outline: none;
  }

  .source-type-tabs {
    display: flex;
    gap: 0.5rem;
  }

  .tab {
    flex: 1;
    padding: 0.5rem 0.75rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .tab:hover {
    background: var(--bg-hover);
  }

  .tab.active {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: white;
  }

  .search-bar {
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .search-bar svg {
    width: 1rem;
    height: 1rem;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .search-bar input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 0.875rem;
    outline: none;
  }

  .search-bar input::placeholder {
    color: var(--text-muted);
  }

  .message {
    margin: 0.75rem 1.25rem;
    padding: 0.5rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.8125rem;
    flex-shrink: 0;
  }

  .message.error {
    background: rgba(239, 68, 68, 0.15);
    color: var(--error);
  }

  .message.success {
    background: rgba(34, 197, 94, 0.15);
    color: var(--success);
  }

  .source-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.25rem 0.5rem;
    margin-bottom: 0.25rem;
  }

  .list-header span {
    font-size: 0.75rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .refresh-btn {
    padding: 0.25rem 0.5rem;
    background: transparent;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .refresh-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
  }

  .source-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.625rem 0.75rem;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.8125rem;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s;
  }

  .source-item:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--border-light);
  }

  .source-item.selected {
    background: rgba(220, 38, 38, 0.1);
    border-color: var(--accent-primary);
  }

  .source-item:disabled {
    opacity: 0.6;
  }

  .source-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .active-badge {
    background: var(--accent-primary);
    color: white;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    font-size: 0.625rem;
    font-weight: 600;
    text-transform: uppercase;
    flex-shrink: 0;
    margin-left: 0.5rem;
  }

  .no-sources {
    color: var(--text-muted);
    text-align: center;
    padding: 2rem 1rem;
    font-size: 0.875rem;
    margin: 0;
  }

  .loading-overlay {
    position: absolute;
    inset: 0;
    background: rgba(10, 10, 10, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spinner {
    width: 2rem;
    height: 2rem;
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
</style>
