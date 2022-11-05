FROM python:3

WORKDIR /app

COPY ./requirements.txt /app/requirements.txt
RUN pip install --trusted-host pypi.python.org -r requirements.txt
COPY . /app

ENV PORT=6001
CMD ["python3", "main.py"]
