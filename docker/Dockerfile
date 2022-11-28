FROM python:3.10

WORKDIR /app

COPY ./requirements.txt /app/requirements.txt
RUN pip install -r requirements.txt
COPY . /app
ENV PORT=6001
CMD python3 -m uvicorn taigaishida.taiga:app --port $PORT --host 0.0.0.0 --reload
