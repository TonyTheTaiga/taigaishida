<script lang="ts">
  type GalleryItem = {
    url: string;
    haiku?: string[];
    line1?: string;
    line2?: string;
    line3?: string;
  };

  let { data } = $props<{
    data: { items: GalleryItem[]; cursor: string | null; limit: number };
  }>();

  let pageNumber = $state(0);
  let items = $state(data.items ?? []);
  let cursor = $state<string | null>(data.cursor ?? null);
  let loadingMore = $state(false);

  let maxPage = $derived(items.length);

  function goToPage(page: number) {
    if (page < 0 || page > maxPage) return;
    pageNumber = page;
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
        `/api/images?limit=${data.limit}&cursor=${encodeURIComponent(cursor)}`,
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
    if (typeof window === "undefined") return;

    function handleKeydown(e: KeyboardEvent) {
      if (
        e.key === "ArrowRight" ||
        e.key === "ArrowDown" ||
        e.key === "d" ||
        e.key === "s"
      ) {
        e.preventDefault();
        nextPage();
      } else if (
        e.key === "ArrowLeft" ||
        e.key === "ArrowUp" ||
        e.key === "a" ||
        e.key === "w"
      ) {
        e.preventDefault();
        prevPage();
      }
    }

    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  });

  // Mobile swipe navigation
  $effect(() => {
    if (typeof window === "undefined") return;

    let startX = 0;
    let startY = 0;
    let endX = 0;
    let endY = 0;

    function handleTouchStart(e: TouchEvent) {
      startX = e.touches[0].clientX;
      startY = e.touches[0].clientY;
    }

    function handleTouchEnd(e: TouchEvent) {
      endX = e.changedTouches[0].clientX;
      endY = e.changedTouches[0].clientY;

      const deltaX = endX - startX;
      const deltaY = endY - startY;

      // Require at least 50px swipe distance and more horizontal than vertical
      if (Math.abs(deltaX) > 50 && Math.abs(deltaX) > Math.abs(deltaY)) {
        if (deltaX > 0) {
          prevPage(); // Swipe right = previous
        } else {
          nextPage(); // Swipe left = next
        }
      }
    }

    window.addEventListener("touchstart", handleTouchStart, { passive: true });
    window.addEventListener("touchend", handleTouchEnd, { passive: true });

    return () => {
      window.removeEventListener("touchstart", handleTouchStart);
      window.removeEventListener("touchend", handleTouchEnd);
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
    ← → arrows, WASD or swipe to navigate
  </div>

  <div class="flex h-full items-center justify-center">
    {#if pageNumber === 0}
      <div class="animate-in fade-in mx-auto max-w-md p-8">
        <div
          class="border border-gray-50 bg-white p-8 transition-all duration-300 hover:border-gray-200 hover:shadow-lg"
        >
          <h1 class="mb-2 text-2xl font-medium text-gray-900">Taiga Ishida</h1>
          <p class="mb-6 text-gray-500">30</p>
          <div class="mb-6">
            <p class="text-sm font-medium text-gray-900">Brooklyn</p>
            <p class="text-sm text-gray-500">NY</p>
          </div>
          <div class="mb-6">
            <p class="text-sm">
              i've been a ML guy for most of my career with hands on experience training models and deploying CV and NLP solutions.
            </p>
          </div>
          <div class="mb-6">
            <p class="text-sm">
              currently i am building <a href="https://toracker.com" class="text-blue-400">Tora</a> to try and make model training fun (again).
            </p>
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
              class="block text-sm text-gray-600 transition-colors hover:text-gray-900"
              >github</a
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
            alt={currentItem.line1 ?? currentItem.haiku?.[0] ?? ""}
            class="max-h-[90vh] max-w-full object-contain"
          />

          <!-- Haiku overlay -->
          {#if currentItem.line1 || currentItem.line2 || currentItem.line3 || currentItem.haiku?.length}
            <div
              class="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent"
            >
              <div
                class="absolute right-0 bottom-0 left-0 p-6 text-left text-white"
              >
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
            </div>
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

<!-- styles moved to src/app.css to avoid Tailwind parser issues -->
