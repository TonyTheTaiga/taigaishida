<script lang="ts">
	type GalleryItem = {
		url: string;
		haiku?: string[];
		line1?: string;
		line2?: string;
		line3?: string;
	};

	let { data } = $props<{ data: { items: GalleryItem[]; cursor: string | null; limit: number } }>();

	let pageNumber = $state(0);
	let items = $state(data.items ?? []);
	let cursor = $state<string | null>(data.cursor ?? null);
	let loadingMore = $state(false);
	let isScrolling = $state(false);
	let haikuVisible = $state(false);

	let maxPage = $derived(items.length);

	function goToPage(page: number) {
		if (page < 0 || page > maxPage) return;
		pageNumber = page;
		haikuVisible = false;
	}

	function nextPage() {
		if (pageNumber < maxPage) {
			goToPage(pageNumber + 1);
		} else {
			loadMoreImages();
		}
	}

	function prevPage() {
		if (pageNumber > 0) {
			goToPage(pageNumber - 1);
		}
	}

	async function loadMoreImages() {
		if (!cursor || loadingMore) return;
		loadingMore = true;
		try {
			const res = await fetch(
				`/api/images?limit=${data.limit}&cursor=${encodeURIComponent(cursor)}`
			);
			if (res.ok) {
				const json = await res.json();
				items = [...items, ...(json.items ?? [])];
				cursor = json.cursor ?? null;
			}
		} finally {
			loadingMore = false;
		}
	}

	$effect(() => {
		if (typeof window === 'undefined') return;

		function handleKeydown(e: KeyboardEvent) {
			if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
				e.preventDefault();
				nextPage();
			} else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
				e.preventDefault();
				prevPage();
			} else if (e.key === ' ' || e.key === 'Enter') {
				e.preventDefault();
				if (pageNumber > 0) {
					haikuVisible = !haikuVisible;
				}
			}
		}

		window.addEventListener('keydown', handleKeydown);
		return () => window.removeEventListener('keydown', handleKeydown);
	});

	$effect(() => {
		if (typeof window === 'undefined') return;

		let scrollTimeout: ReturnType<typeof setTimeout>;
		let lastScrollTime = 0;
		let scrollDelta = 0;

		function handleWheel(e: WheelEvent) {
			if (isScrolling) return;

			const now = Date.now();
			const timeDiff = now - lastScrollTime;

			scrollDelta += e.deltaY;

			if (timeDiff > 150) {
				scrollDelta = e.deltaY;
			}

			lastScrollTime = now;

			clearTimeout(scrollTimeout);

			scrollTimeout = setTimeout(() => {
				if (Math.abs(scrollDelta) > 100) {
					isScrolling = true;
					if (scrollDelta > 0) {
						nextPage();
					} else {
						prevPage();
					}
					setTimeout(() => {
						isScrolling = false;
						scrollDelta = 0;
					}, 300);
				} else {
					scrollDelta = 0;
				}
			}, 50);
		}

		window.addEventListener('wheel', handleWheel, { passive: true });
		return () => {
			window.removeEventListener('wheel', handleWheel);
			clearTimeout(scrollTimeout);
		};
	});
</script>

<div class="fixed inset-0 overflow-hidden bg-white">
	<div
		class="fixed top-4 right-4 z-50 rounded-full bg-white/80 px-3 py-1 text-sm text-gray-500 backdrop-blur-sm"
	>
		{pageNumber} / {maxPage}
	</div>

	<div
		class="fixed bottom-4 left-4 z-50 rounded-full bg-white/80 px-3 py-1 text-xs text-gray-400 backdrop-blur-sm"
	>
		← → arrows or scroll to navigate{pageNumber > 0 ? ' • space/enter for haiku' : ''}
	</div>

	<div class="flex h-full items-center justify-center transition-all duration-500 ease-out">
		{#if pageNumber === 0}
			<div class="animate-in fade-in mx-auto max-w-md p-8 duration-500">
				<div
					class="border border-gray-50 bg-white p-8 transition-all duration-300 hover:border-gray-200 hover:shadow-lg"
				>
					<h1 class="mb-2 text-2xl font-medium text-gray-900">Taiga Ishida</h1>
					<p class="mb-6 text-gray-500">30</p>
					<div class="mb-6">
						<p class="text-sm font-medium text-gray-900">Brooklyn</p>
						<p class="text-sm text-gray-500">NY</p>
					</div>
					<div class="space-y-3">
						<a
							href="mailto:ishidataiga@gmail.com"
							class="block text-sm text-gray-600 transition-colors hover:text-gray-900"
							>ishidataiga@gmail.com</a
						>
						<a
							href="https://github.com/TonyTheTaiga"
							target="_blank"
							class="block text-sm text-gray-600 transition-colors hover:text-gray-900">github</a
						>
					</div>
				</div>
			</div>
		{:else if pageNumber > 0 && pageNumber <= items.length}
			{@const currentItem = items[pageNumber - 1]}
			<div class="flex h-full w-full items-center justify-center p-4">
				<div class="relative max-h-full max-w-full">
					<img
						src={currentItem.url}
						alt={currentItem.line1 ?? currentItem.haiku?.[0] ?? ''}
						class="max-h-[90vh] max-w-full object-contain transition-all duration-300"
					/>

					<!-- Haiku overlay -->
					{#if currentItem.line1 || currentItem.line2 || currentItem.line3 || currentItem.haiku?.length}
						<button
							type="button"
							aria-expanded={haikuVisible}
							aria-label="Toggle haiku overlay"
							class="absolute inset-0 cursor-pointer bg-gradient-to-t from-black/60 via-transparent to-transparent transition-opacity duration-300"
							class:opacity-0={!haikuVisible}
							class:opacity-100={haikuVisible}
							onclick={() => (haikuVisible = !haikuVisible)}
						>
							<div class="absolute right-0 bottom-0 left-0 p-6 text-left text-white">
								{#if currentItem.line1 || currentItem.haiku?.[0]}
									<div class="mb-2 text-lg font-light">
										{currentItem.line1 ?? currentItem.haiku?.[0]}
									</div>
								{/if}
								{#if currentItem.line2 || currentItem.haiku?.[1]}
									<div class="mb-1 text-base opacity-90">
										{currentItem.line2 ?? currentItem.haiku?.[1]}
									</div>
								{/if}
								{#if currentItem.line3 || currentItem.haiku?.[2]}
									<div class="text-base opacity-80">
										{currentItem.line3 ?? currentItem.haiku?.[2]}
									</div>
								{/if}
							</div>
						</button>
					{/if}
				</div>
			</div>
		{:else if loadingMore}
			<div class="flex items-center justify-center text-gray-500">
				<div class="animate-pulse">Loading more images...</div>
			</div>
		{:else}
			<div class="flex items-center justify-center text-gray-400">
				<div>End of gallery</div>
			</div>
		{/if}
	</div>
</div>

<style>
	@keyframes fade-in {
		from {
			opacity: 0;
			transform: translateY(20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.animate-in {
		animation: fade-in 0.5s ease-out;
	}

	/* On hover (desktop), reveal haiku overlay */
	@media (hover: hover) and (pointer: fine) {
		img:hover + button,
		button:hover {
			opacity: 1 !important;
		}
	}
</style>
