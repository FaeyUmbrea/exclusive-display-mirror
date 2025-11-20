<script lang="ts">
  import '../index.css';
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { exit } from '@tauri-apps/plugin-process';
  import LoadedPage from '$lib/components/LoadedPage.svelte';

  // Removed progress-related state - bootstrapper now communicates a single "restart required" or "no update" outcome
  let bootstrapState: 'checking' | 'ready' | 'restart' | 'error' = 'checking';
  let errorMessage = '';
  let lastEvent = '';

  let unlisteners: UnlistenFn[] = [];
  let bootstrapStarted = false;

  onMount(async () => {
    // Guard against multiple mounts
    if (bootstrapStarted) {
      console.log('[App] Bootstrap already started, ignoring duplicate mount');
      return;
    }
    bootstrapStarted = true;

    console.log('[App] Registering bootstrap listeners');

    // Listen for bootstrap completion
    unlisteners.push(await listen('bootstrap_done', () => {
      console.log('[App] bootstrap_done event received');
      bootstrapState = 'ready';
      lastEvent = 'bootstrap_done';
    }));

    // Listen for restart requirement
    unlisteners.push(await listen('bootstrap_restart_required', () => {
      console.log('[App] bootstrap_restart_required event received');
      bootstrapState = 'restart';
      lastEvent = 'bootstrap_restart_required';
    }));

    // Listen for errors
    unlisteners.push(await listen('bootstrap_error', (event) => {
      console.error('[App] bootstrap_error event received:', event);
      errorMessage = String(event.payload || 'Unknown error');
      bootstrapState = 'error';
      lastEvent = `bootstrap_error:${errorMessage}`;
    }));

    // Check for manifest before starting bootstrapper
    console.log('[App] Checking for bootstrap manifest');
    try {
      const hasManifest = await invoke('check_bootstrap_manifest') as boolean;
      if (hasManifest) {
        console.log('[App] Manifest present — invoking run_bootstrapper_with_pid');
        const res = await invoke('run_bootstrapper_with_pid') as string;
        console.log('[App] run_bootstrapper_with_pid completed:', res);
      } else {
        console.log('[App] Manifest missing — showing downloader UI');
        bootstrapState = 'error';
        errorMessage = '';
      }
    } catch (e) {
      console.error('[App] check_bootstrap_manifest error:', e);
      errorMessage = String(e);
      bootstrapState = 'error';
    }
  });

  onDestroy(() => {
    unlisteners.forEach(fn => fn());
  });

  async function handleRestart() {
    console.log('[App] User initiated restart/exit');
    try {
      console.log('[App] Calling exit(0)...');
      await exit(0);
      console.log('[App] Exit called successfully');
    } catch (e) {
      console.error('[App] Failed to exit:', e);
      errorMessage = 'Failed to close application. Please close manually and restart.';
      bootstrapState = 'error';
    }
  }

  async function launchDownloader() {
    console.log('[App] User requested downloader launch');
    try {
      await invoke('launch_bootstrapper_downloader');
      // Inform the user the downloader was started and may prompt for admin rights
      errorMessage = 'Launcher started; it may ask for administrator permissions. If nothing appears, try running the installer manually.';
    } catch (e) {
      console.error('[App] launch_bootstrapper_downloader error:', e);
      errorMessage = String(e);
    }
  }

  function handleRetry() {
    console.log('[App] User initiated retry');
    // Reload the page to restart bootstrap process
    window.location.reload();
  }
</script>

<style>
  .loading-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    padding: 2rem;
  }

  .loading-card {
    background: white;
    border-radius: 1rem;
    padding: 3rem;
    max-width: 600px;
    width: 100%;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  }

  .loading-header {
    text-align: center;
    margin-bottom: 2rem;
  }

  .loading-header h1 {
    font-size: 2rem;
    font-weight: 700;
    color: #1a202c;
    margin: 0 0 0.5rem 0;
  }

  .loading-header p {
    color: #718096;
    font-size: 1rem;
    margin: 0;
  }

  .spinner {
    width: 3rem;
    height: 3rem;
    border: 4px solid #e2e8f0;
    border-top-color: #667eea;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 1.5rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .restart-content {
    text-align: center;
  }

  .restart-icon {
    width: 4rem;
    height: 4rem;
    margin: 0 auto 1.5rem;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    font-size: 2rem;
  }

  .restart-message {
    font-size: 1rem;
    color: #4a5568;
    line-height: 1.6;
    margin-bottom: 2rem;
  }

  .btn {
    display: inline-block;
    padding: 0.75rem 2rem;
    border: none;
    border-radius: 0.5rem;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
  }

  .btn-primary:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
  }

  .error-content {
    text-align: center;
  }

  .error-icon {
    width: 4rem;
    height: 4rem;
    margin: 0 auto 1.5rem;
    background: #fed7d7;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #c53030;
    font-size: 2rem;
  }

  .error-message {
    font-size: 0.875rem;
    color: #4a5568;
    background: #f7fafc;
    padding: 1rem;
    border-radius: 0.5rem;
    margin-bottom: 2rem;
    word-break: break-word;
  }
</style>

<main>
  {#if bootstrapState === 'ready'}
    <LoadedPage />
  {:else}
    <div class="loading-container">
      <div class="loading-card">
        <div class="loading-header">
          <h1>Exclusive Display Mirror</h1>
          <p>
            {#if bootstrapState === 'checking'}
              Checking dependencies...
            {:else if bootstrapState === 'restart'}
              Restart Required
            {:else if bootstrapState === 'error'}
              Setup Error
            {/if}
          </p>
        </div>

        {#if bootstrapState === 'checking'}
          <div class="spinner"></div>
          <p style="text-align: center; color: #a0aec0; font-size: 0.875rem;">
            Please wait...
          </p>

        {:else if bootstrapState === 'restart'}
          <div class="restart-content">
            <div class="restart-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" style="width: 2rem; height: 2rem;">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
            </div>
            <div class="restart-message">
              <p style="margin-bottom: 1rem;">
                <strong>OBS needs to be updated or downloaded.</strong>
              </p>
              <p>
                The bootstrapper is waiting for the application to exit so it can update or install OBS. Click the button below to close this app now.
                If the application does not restart automatically after a short while, please reopen it manually.
              </p>
            </div>
            <button class="btn btn-primary" on:click={handleRestart}>
              Close Application
            </button>
          </div>

        {:else if bootstrapState === 'error'}
          <div class="error-content">
            <div class="error-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" style="width: 2rem; height: 2rem;">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>

            {#if !errorMessage}
              <!-- Manifest missing UI -->
              <p style="margin-bottom: 1rem; color: #1a202c; font-weight: 600;">
                OBS is not installed or the bootstrap manifest is missing.
              </p>
              <p style="margin-bottom: 1rem; color: #4a5568;">
                To download and install OBS, press "Launch Downloader". The downloader may request administrator permissions to write to the folder where this app is installed.
              </p>
              <div style="display:flex; gap:1rem; justify-content:center;">
                <button class="btn btn-primary" on:click={launchDownloader}>
                  Launch Downloader
                </button>
                <button class="btn" on:click={handleRetry}>
                  Cancel
                </button>
              </div>
            {:else}
              <p style="margin-bottom: 1rem; color: #c53030; font-weight: 600;">
                Failed to initialize dependencies
              </p>
              <div class="error-message">{errorMessage}</div>
              <button class="btn btn-primary" on:click={handleRetry}>
                Retry
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</main>
