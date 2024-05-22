# Test Manifest

## Schema

### `invocations`

A list of invocations of the application and their associated responses

* type: `list<invocation>`

### `invocation`

Represents a single invocation of the application. 
* type: `http-invocation` (TODO: in the future this will be extended with `| redis-invocation` and others)

### `http-invocation`

An invocation of a Spin application running an http based trigger

* type: `object`
* fields:
    * request: `http-request`
    * response `http-response` - the response required for the test to pass

### `http-request`

* type: `object`
* fields:
    * method: `http-method`
    * path: `string` (optional - default: `"/"`) - the path and query for the request
    * headers: `list<http-header>` (optional - default `[]`)
    * body: `option<http-body>` (optional - default `null`)

### `http-response`

* type: `object`
* fields:
    * status: `number` (optional - default `200`)
    * headers: `list<http-header>` (optional - default `[]`)
    * body: `option<http-body>` (optional - default `null`)

### `http-header`

* type: `object`
* fields:
    * name: `string`
    * value: `string` (optional - if not present only presence of header is checked)
    * optional: `bool` (optional - if true the header is allowed to be either present or not)

### `http-body`

* type: `list<u8>` | `list<list<u8>>`

### `http-method`

* type: `"get"` | `"post"` (TODO: add more)

