<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  let el: HTMLElement | null = null;
  let wasAdded = false;

  function getBounds() {
    if (!el) return null;
    const b = el.getBoundingClientRect();
    return {
      x: Math.round(b.x),
      y: Math.round(b.y),
      width: Math.round(b.width),
      height: Math.round(b.height)
    };
  }

  async function addOrUpdatePreview() {
    const bounds = getBounds();
    if (!bounds) return;
    try {
      if (!wasAdded) {
        await invoke('add_preview', bounds);
        wasAdded = true;
        console.log('Preview added');
      } else {
        await invoke('resize_preview', bounds);
        console.log('Preview updated');
      }
    } catch (err) {
      console.error('Preview invoke error:', err);
    }
  }

  onMount(() => {
    addOrUpdatePreview();

    const resizeHandler = () => {
      if (!wasAdded) return;
      addOrUpdatePreview();
    };

    window.addEventListener('resize', resizeHandler);
    window.addEventListener('scroll', resizeHandler);

    return () => {
      if (!wasAdded) return;
      console.log('Removing preview');
      invoke('close_preview')
        .then(() => console.log('Preview removed'))
        .catch((err) => console.error(err));
    };
  });
</script>

<div bind:this={el} class="w-full h-full aspect-video bg-gray-900 flex justify-center items-center"></div>
