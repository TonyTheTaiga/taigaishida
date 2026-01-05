<script lang="ts">
	import { onMount } from 'svelte';

	interface ImageItem {
		url: string;
		haiku?: string[];
		line1?: string;
		line2?: string;
		line3?: string;
		brightness?: number;
		created: string;
	}

	let canvas: HTMLCanvasElement;
	let ctx: CanvasRenderingContext2D | null = null;
	let images: ImageItem[] = [];
	let loadedImages: HTMLImageElement[] = [];
	let loading = $state(true);
	let loadProgress = $state(0);

	// Justified gallery settings
	const TARGET_ROW_HEIGHT = 200; // Base row height (adjusts to fill width)
	const GAP = 4; // Gap between images

	async function fetchAllImages() {
		let cursor: string | null = null;
		const allItems: ImageItem[] = [];

		do {
			const url = cursor ? `/api/images?limit=100&cursor=${cursor}` : '/api/images?limit=100';
			const response = await fetch(url);
			const data = await response.json();
			allItems.push(...data.items);
			cursor = data.cursor;
		} while (cursor);

		return allItems;
	}

	async function loadImage(url: string): Promise<HTMLImageElement> {
		return new Promise((resolve, reject) => {
			const img = new Image();
			img.onload = () => {
				console.log('Loaded:', url, img.width, img.height);
				resolve(img);
			};
			img.onerror = (e) => {
				console.error('Failed to load:', url, e);
				reject(e);
			};
			img.src = url;
		});
	}

	function resizeCanvas() {
		if (!canvas || !ctx) return;

		const dpr = window.devicePixelRatio || 1;
		const width = window.innerWidth;
		const height = window.innerHeight;

		// Set the canvas internal resolution to match device pixels
		canvas.width = width * dpr;
		canvas.height = height * dpr;

		// Scale the canvas back down via CSS
		canvas.style.width = width + 'px';
		canvas.style.height = height + 'px';

		// Reset transform and scale the context to match device pixels
		ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

		drawImages();
	}

	function drawImages() {
		if (!ctx || loadedImages.length === 0) return;

		const screenWidth = window.innerWidth;
		const screenHeight = window.innerHeight;

		// Clear canvas
		ctx.fillStyle = '#000';
		ctx.fillRect(0, 0, screenWidth, screenHeight);

		// Justified gallery layout (like Google Photos)
		const availableWidth = screenWidth - GAP * 2; // padding on sides
		let y = GAP;
		let imageIndex = 0;

		while (imageIndex < loadedImages.length && y < screenHeight + TARGET_ROW_HEIGHT) {
			// Build a row: collect images until they fill the width
			const rowImages: HTMLImageElement[] = [];
			let rowAspectSum = 0;

			// Keep adding images to row until it would be full
			while (imageIndex < loadedImages.length) {
				const img = loadedImages[imageIndex];
				const aspectRatio = img.width / img.height;
				const widthIfAdded =
					(rowAspectSum + aspectRatio) * TARGET_ROW_HEIGHT +
					GAP * rowImages.length;

				if (widthIfAdded > availableWidth && rowImages.length > 0) {
					// Row is full, don't add this image
					break;
				}

				rowImages.push(img);
				rowAspectSum += aspectRatio;
				imageIndex++;
			}

			if (rowImages.length === 0) break;

			// Calculate actual row height to perfectly fill width
			const totalGaps = GAP * (rowImages.length - 1);
			const rowHeight = (availableWidth - totalGaps) / rowAspectSum;

			// Draw the row
			let x = GAP;
			for (const img of rowImages) {
				const aspectRatio = img.width / img.height;
				const imgWidth = rowHeight * aspectRatio;

				ctx.drawImage(img, x, y, imgWidth, rowHeight);
				x += imgWidth + GAP;
			}

			y += rowHeight + GAP;
		}
	}

	onMount(async () => {
		ctx = canvas.getContext('2d');
		resizeCanvas();

		window.addEventListener('resize', resizeCanvas);

		// Fetch images
		images = await fetchAllImages();

		// Load all images
		let loaded = 0;
		const imagePromises = images.map(async (item) => {
			try {
				const img = await loadImage(item.url);
				loaded++;
				loadProgress = Math.round((loaded / images.length) * 100);
				return img;
			} catch (e) {
				loaded++;
				loadProgress = Math.round((loaded / images.length) * 100);
				return null;
			}
		});

		const results = await Promise.all(imagePromises);
		loadedImages = results.filter((img): img is HTMLImageElement => img !== null);

		console.log('Total images fetched:', images.length);
		console.log('Successfully loaded:', loadedImages.length);

		loading = false;
		drawImages();
		console.log('Draw complete');

		return () => {
			window.removeEventListener('resize', resizeCanvas);
		};
	});
</script>

<svelte:head>
	<title>Image Mosaic</title>
</svelte:head>

<div class="fixed inset-0 bg-black">
	<canvas bind:this={canvas} class="block h-full w-full"></canvas>

	{#if loading}
		<div class="fixed inset-0 flex items-center justify-center bg-black/80">
			<div class="text-center text-white">
				<p class="text-xl">Loading images...</p>
				<p class="mt-2 text-gray-400">{loadProgress}%</p>
			</div>
		</div>
	{/if}
</div>
