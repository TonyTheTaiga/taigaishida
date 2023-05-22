dev-css:
	npx tailwindcss -i ./taigaishida/static/src/input.css -o ./taigaishida/static/styles/2023.css --watch

build-css:
	npx tailwindcss -i ./taigaishida/static/src/input.css -o taigaishida/static/styles/2023.css --minify

lint:
	black -l 120 .

update-cors:
	gcloud storage buckets update gs://taiga-ishida-public --cors-file=./misc/cors-settings.json

clear-cors:
	gcloud storage buckets update gs://taiga-ishida-public --clear-cors
