<script lang="ts">
	import { onMount } from 'svelte';
	import { FireworkEngine } from '$lib/fireworks';

	let canvas: HTMLCanvasElement;
	let engine: FireworkEngine | null = null;

	onMount(() => {
		engine = new FireworkEngine(canvas);
		engine.start();

		const onResize = () => engine?.resize();
		window.addEventListener('resize', onResize);

		return () => {
			engine?.stop();
			window.removeEventListener('resize', onResize);
		};
	});
</script>

<svelte:head>
	<title>Taiga Ishida</title>
</svelte:head>

<div class="fixed inset-0 bg-black overflow-hidden">
	<canvas bind:this={canvas} class="block w-full h-full"></canvas>
</div>

<style>
	:global(body) {
		overflow: hidden;
		margin: 0;
	}
</style>
