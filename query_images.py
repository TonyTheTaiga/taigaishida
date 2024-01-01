from google.cloud import datastore

client = datastore.Client(database="")
query = client.query(kind="Image")
rows = list(query.fetch())
# print(len(rows))
for row in rows:
    print(row["latlong"])
    break
