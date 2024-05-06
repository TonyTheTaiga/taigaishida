from google.cloud import datastore

# Create a client to interact with the Datastore service
client = datastore.Client()

# Define the kind for which you want to delete all entities
kind = "Image"

# Create a query to retrieve all entities of the specified kind
query = client.query(kind=kind)

# Retrieve all entities matching the query
entities = list(query.fetch())

# Delete all entities
for entity in entities:
    client.delete(entity.key)

print(f"Deleted {len(entities)} entities of kind {kind}")
