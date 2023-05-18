# CDS - Content Delivery Server

## Scope

The scope of this service is to add a new infrastructure layer to serve static resources to Entando's components (de-app, appbuilder),
in particular when we need to scale-out the de-app or in multitenancy configurations.

The CDS service exposes two ports:

- Internal Port: 8080, everything under `/api/v1` path and needs authorization (bearertoken)
- Public Port: 8081, which needs to be exposed by an Ingress object


## Documentation

To see the documentation:
1. Clone the project
2. Run `cargo doc --open`

## Notes

The order of the parameters is very important for the `api/v1/upload/` api:
1. path
2. protected
3. filename
4. file

```bash
curl --location --request POST 'https://cds.domain.com/api/v1/upload/' \
--header 'Authorization: Bearer eyJhbG...K3unA' \
--form 'path="test"' \
--form 'protected="true"' \
--form 'filename="file.png"' \
--form 'file=@"/home/user/Images/Screenshot_20220526_143107.png"'
```

## Environment Varibles

To be able to start the CDS server we must define these env vars:

- **KEYCLOAK_PUBLIC_KEY**="-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAngLylJCK3Z5F7kwt0yJkud8dgfMZJsabGH7dnCYvwO4zwhSQnKczUcNoqH9iOTSX+kA6/xmUp7IxIUKDV3bIrk9k9Qu80c+k/PtPeEkgeAtRc3Z2oErGgI2UBd6qhxeUb1yd8cLh7FY1xEUOK/eFaUTwIDAQAB\n-----END PUBLIC KEY-----\n"
- **RUST_LOG**="actix_web=trace,actix_server=trace,actix_web_middleware_keycloak_auth=trace"
- **CORS_ALLOWED_ORIGIN**=https://host.domain.com (or All)
- **CORS_ALLOWED_ORIGIN_END_WITH**=your-domain.com 

### Environment variables for the init container

| Name      | Default Value                                                  | Description                                                 |
|-----------|----------------------------------------------------------------|-------------------------------------------------------------|
| REPO_PATH | https://raw.github.com/entando-ps/cds/entando720/entando-data/ | The path of the GitHub repository used to store the archive |
| ARCHIVE_NAME | entando720.tar.gz | The name of the archive containing all the Entando static resources |
| FORCE_CDS | false | Needed to force the overwriting of the volume | 

