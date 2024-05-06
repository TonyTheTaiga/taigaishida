FROM python:3.11

WORKDIR /app

COPY ./requirements.txt /app/requirements.txt
RUN pip install -r requirements.txt
COPY . /app
ENV PORT=6001
CMD python3 -m uvicorn --factory taigaishida.taiga:build_app --port $PORT --host 0.0.0.0 --reload
