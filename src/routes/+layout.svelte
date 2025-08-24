<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
  </svelte:head>

<div class="min-h-screen bg-gradient-to-b from-neutral-50 to-white text-neutral-800 antialiased font-sans selection:bg-black/10 selection:text-black/80">
	{@render children?.()}
</div>
