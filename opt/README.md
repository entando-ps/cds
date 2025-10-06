# CDS - Content Delivery Server

## Requirements ##

This guide requires the following tools:

- Cargo
- Postman or Postman CLI
- Docker


## Cargo Tests

Cargo tests can be executed running `cargo test` on the project root.

```bash
cd ..
cargo test
```

## Postman Tests- ##


### Docker Environment ###

The provided docker-compose.yml uses:
- keycloak
- cds: the current image


#### Port Requirements

The docker stack requires the following ports:
- 59080: for keycloak
- 58080: for keycloak
- 58080: for keycloak


#### Startup

To start the docker containers, use the command:
```bash
docker-compose up -d
```

After the first startup, it's necessary to update the KEYCLOAK_PUBLIC_KEY in docker-compose.yml:
- open the url [http://localhost:50090/auth/admin/master/console/#/realms/entando-dev/keys](http://localhost:50090/auth/admin/master/console/#/realms/entando-dev/keys)
- login with admin/admin
- retrieve the public key and paste it into the docker-compose in the cds section
- destroy and regenerate the containers using the following command:
```bash
docker compose up -d --no-deps --build cds
```


#### Stop
- Stop the docker containers using the command:
```bash
docker-compose down --rmi local
```


### Postman Tests ###

Update the following variables in [postman_collection.json](postman_collection.json) changing the ports in the parameters:
- cds-private-url
- cds-public-url
- keycloak-url


Import the file [postman_collection.json](postman_collection.json) in Postman and run the entire collection, 
or run it directly with Postman CLI using the following command:  

```bash
postman collection run postman_collection.json
```
