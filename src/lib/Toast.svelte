<script lang="ts">
  export let id: string;
  export let message: string;
  export let kind: 'info' | 'success' | 'warn' | 'error' = 'info';
</script>

<div class="toast {kind}">
  <div class="toast-content">{message}</div>
</div>

<style>
  .toast {
    background: rgba(0,0,0,0.75);
    color: white;
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    box-shadow: 0 6px 18px rgba(0,0,0,0.4);
    margin-top: 0.5rem;
    max-width: 320px;
    font-size: 0.95rem;
  }
  .toast.success { background: linear-gradient(90deg,#10b981,#059669); }
  .toast.warn { background: linear-gradient(90deg,#f59e0b,#d97706); }
  .toast.error { background: linear-gradient(90deg,#ef4444,#dc2626); }
  .toast.info { background: rgba(0,0,0,0.75); }
</style>