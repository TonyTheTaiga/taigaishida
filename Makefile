dev-css:
	npx tailwindcss -i ./taigaishida/static/src/input.css -o ./taigaishida/static/styles/2023.css --watch

build-css:
	npx tailwindcss -o taigaishida/static/styles/2023.css --minify

lint:
	black -l 120 .