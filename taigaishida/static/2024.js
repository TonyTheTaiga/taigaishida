const popup = document.getElementById('popup');
const table = document.getElementById('galleryTable');
const jumbo = document.getElementById('jumboImg');
const pageNumber = document.getElementById('currentPageNumber');
const prevButton = document.getElementById('prevButton');
const nextButton = document.getElementById('nextButton');

let currentImage = null;
let highlighted = null;

const xOffset = 40;
const yOffset = 40;

table.addEventListener('mouseover', (event) => {
if (event.target.tagName === 'TD') {
	if (popup.childNodes.length >= 1) {
		popup.removeChild(currentImage);
		currentImage = null;
		popup.style.left = 0 + 'px';
		popup.style.top = 0 + 'px';
	}

	const row = event.target.closest('tr');
	const mediaContent = row.getAttribute('data-src');
	currentImage = new Image(width = 200);
	popup.appendChild(currentImage);
	currentImage.onload = (event) => {
		popup.style.left = event.pageX + xOffset + 'px';
		popup.style.top = event.pageY + yOffset + 'px';
		popup.classList.add('bg-white');
		popup.classList.remove('hidden');
	};
	currentImage.src = mediaContent;
}
});

table.addEventListener('mousemove', (event) => {
if (event.target.tagName == 'TD') {
	if (currentImage) {
		popup.style.left = event.pageX + xOffset + 'px';
		popup.style.top = event.pageY + yOffset + 'px';
	}
}
});

table.addEventListener('mouseout', (event) => {
popup.classList.add('hidden');
popup.classList.remove('bg-white');
if (popup.childNodes.length >= 1) {
	popup.removeChild(currentImage);
	currentImage = null;
	popup.style.left = 0 + 'px';
	popup.style.top = 0 + 'px';
}
});

table.addEventListener('click', (event) => {
if (event.target.tagName == 'TD') {
	highlighted = event.target.closest('tr');
	jumbo.src = highlighted.getAttribute('data-src');
}
});


function prevPage() {
	console.log('prev...');
	console.log(pageNumber.textContent);
	htmx.trigger('#prevButton', 'goPrev');
};
function nextPage() {
	console.log('next...');
	htmx.trigger('#nextButton', 'goNext');
};
