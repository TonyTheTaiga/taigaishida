dev-css:
	npx tailwindcss -i input.css -o ./taigaishida/static/styles/2024.css --watch

build-css:
	npx tailwindcss -i input.css -o taigaishida/static/styles/2024.css --minify

lint:
	black -l 120 .

update-cors:
	gcloud storage buckets update gs://taiga-ishida-public --cors-file=./misc/cors-settings.json
	gcloud storage buckets update gs://taiga-ishida-private --cors-file=./misc/cors-settings.json

clear-cors:
	gcloud storage buckets update gs://taiga-ishida-public --clear-cors
	gcloud storage buckets update gs://taiga-ishida-private --clear-cors

start-server:
	uvicorn --factory taigaishida.taiga:build_app --reload
