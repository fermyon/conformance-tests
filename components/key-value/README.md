# Key Value

Tests the key/value interface.

## Expectations

This test component expects the following to be true:
* It is given permission to open a connection to an existing store named "default".
* It does not have permission to access a store named "forbidden".
* It has permission to access a store named "non-existent" but the store does not exist.
* The "default" store is empty
