# BRC20 indexer API
This document defines a REST API specification for querying the BRC20S indexer.


See sepc source:
- [openapi.json](./openapi.json)

## Sanity checks
Double check that the spec looks good by running server and then visit http://127.0.0.1/api/v1/api-docs/openapi.json.


How to running indexer server
```bash
ord --bitcoin-rpc-user foo --bitcoin-rpc-pass bar server
```