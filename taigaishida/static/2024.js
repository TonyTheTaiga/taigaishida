let currentImage = null;
let highlighted = null;

const xOffset = 40;
const yOffset = 40;

htmx.on("htmx:afterSettle", function (elt) {
  if (elt["target"].id == "gallery") {
    const popup = document.getElementById("popup");
    const table = document.getElementById("galleryTable");
    const jumboStatic = document.getElementById("jumboImg");
    const currentPageNumber = document.getElementById("currentPageNumber");
    const lastPageNumber = document.getElementById("lastPageNumber");


		const prevPageBtn = document.getElementById('prevPageBtn');
		if (currentPageNumber.textContent == 1) {
			prevPageBtn.disabled = true;
			prevPageBtn.classList.add('text-gray-800');
		};
		prevPageBtn.addEventListener('click', () => {htmx.trigger("#prevPageBtn", "loadPrevPage")});

		const nextPageBtn = document.getElementById('nextPageBtn');
		if (currentPageNumber.textContent.trim() == lastPageNumber.textContent.trim()) {
			nextPageBtn.disabled = true;
			nextPageBtn.classList.add('text-gray-800');
		};
		nextPageBtn.addEventListener('click', ()=>{htmx.trigger("#nextPageBtn", "loadNextPage")});
    
		/*
     * Event listeners for row interactions
     */

		function moveImage(event) {
			// console.log(currentImage.width, currentImage.height);
			// console.log(event);
			popup.style.left = event.pageX + xOffset + "px";
			popup.style.top = event.pageY + yOffset + "px";
		};

    table.addEventListener("mouseover", (event) => {
      if (event.target.tagName === "TD") {
				/*
				I think this is not needed but unsure
        if (popup.childNodes.length >= 1) {
          popup.removeChild(currentImage);
          currentImage = null;
          popup.style.left = 0 + "px";
          popup.style.top = 0 + "px";
					popup.classList.add("hidden");
					popup.classList.remove("bg-white");
        }
				*/

        const row = event.target.closest("tr");
        const mediaContent = row.getAttribute("data-src");
				if (mediaContent == 'Null') {
					return;
				}

        currentImage = new Image((width = 200));
        currentImage.src = mediaContent;
        popup.appendChild(currentImage);

				currentImage.onload = (event) => {
          // popup.style.left = event.pageX + xOffset + "px";
          // popup.style.top = event.pageY + yOffset + "px";
          // moveImage(event);
					popup.classList.add("bg-white");
          popup.classList.remove("hidden");
        };
      }
    });

    table.addEventListener("mousemove", (event) => {
      if (event.target.tagName == "TD") {
        if (currentImage) {
					// popup.style.left = event.pageX + xOffset + "px";
          // popup.style.top = event.pageY + yOffset + "px";
					moveImage(event);
        }
      }
    });

    table.addEventListener("mouseout", (event) => {
      if (popup.childNodes.length >= 1) {
        popup.removeChild(currentImage);
        currentImage = null;
        popup.style.left = 0 + "px";
        popup.style.top = 0 + "px";
				popup.classList.add("hidden");
				popup.classList.remove("bg-white");
      }
    });

    table.addEventListener("click", (event) => {
      if (event.target.tagName == "TD") {
        highlighted = event.target.closest("tr");
        imageSrc = highlighted.getAttribute("data-src");
				if (imageSrc == "Null") {
					return;
				}
				jumboStatic.src = imageSrc;
      }
    });
  }
});

