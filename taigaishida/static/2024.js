let currentImage = null;
let highlighted = null;

const xOffset = 40;
const yOffset = 40;

htmx.on("htmx:afterSettle", function (elt) {
  if (elt["target"].id == "gallery") {
    const popup = document.getElementById("popup");
    const table = document.getElementById("galleryTable");
    const jumbo = document.getElementById("jumboImg");
    const currentPageNumber = document.getElementById("currentPageNumber");
    const lastPageNumber = document.getElementById("lastPageNumber");

    function prevPage() {
      console.log("prev...");
      // htmx.trigger("#prevButton", "goPrev");
    }

		document.getElementById('prevPageBtn').addEventListener('click', prevPage);

    function nextPage() {
      console.log("next...");
			// htmx.trigger("#nextButton", "goNext");
    }

		document.getElementById('nextPageBtn').addEventListener('click', nextPage);


    /*
     * Event listeners for row interactions
     */

    table.addEventListener("mouseover", (event) => {
      if (event.target.tagName === "TD") {
        if (popup.childNodes.length >= 1) {
          popup.removeChild(currentImage);
          currentImage = null;
          popup.style.left = 0 + "px";
          popup.style.top = 0 + "px";
        }

        const row = event.target.closest("tr");
        const mediaContent = row.getAttribute("data-src");
				if (mediaContent == 'Null') {
					return;
				}	
        currentImage = new Image((width = 200));
        popup.appendChild(currentImage);
        currentImage.onload = (event) => {
          popup.style.left = event.pageX + xOffset + "px";
          popup.style.top = event.pageY + yOffset + "px";
          popup.classList.add("bg-white");
          popup.classList.remove("hidden");
        };
        currentImage.src = mediaContent;
      }
    });

    table.addEventListener("mousemove", (event) => {
      if (event.target.tagName == "TD") {
        if (currentImage) {
          popup.style.left = event.pageX + xOffset + "px";
          popup.style.top = event.pageY + yOffset + "px";
        }
      }
    });

    table.addEventListener("mouseout", (event) => {
      popup.classList.add("hidden");
      popup.classList.remove("bg-white");
      if (popup.childNodes.length >= 1) {
        popup.removeChild(currentImage);
        currentImage = null;
        popup.style.left = 0 + "px";
        popup.style.top = 0 + "px";
      }
    });

    table.addEventListener("click", (event) => {
      if (event.target.tagName == "TD") {
        highlighted = event.target.closest("tr");
        imageSrc = highlighted.getAttribute("data-src");
				if (imageSrc == "Null") {
					return;
				}
				console.log(imageSrc);
				jumbo.src = imageSrc;
      }
    });
  }
});

