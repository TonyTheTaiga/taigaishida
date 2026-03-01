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

	<footer class="fixed bottom-0 left-0 right-0 z-40 pointer-events-none">
		<div class="flex items-center justify-center gap-4 py-2 font-mono text-[10px] tracking-[0.2em] uppercase text-neutral-400">
			<span>Taiga Ishida</span>
			<span class="text-neutral-300">|</span>
			<span>ishidataiga@gmail.com</span>
			<span class="text-neutral-300">|</span>
			<span>Brooklyn, NY</span>
		</div>
	</footer>
</div>
