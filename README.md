# IPDB Server

A simple IPDB server with low memory footprint and high performance.

## Setup

```plaintext
Usage: ipdb-server [OPTIONS]

Options:
  -a, --addr <ADDR>              The address to listen on [env: IPDB_SERVER_ADDR=] [default:
                                 localhost]
  -p, --port <PORT>              The port to listen on [env: IPDB_SERVER_PORT=] [default:
                                 26583]
  -4, --v4-path <V4_PATH>        IPDB IPv4 path [env: IPDB_SERVER_V4=]
  -6, --v6-path <V6_PATH>        IPDB IPv6 path [env: IPDB_SERVER_V4=]
  -t, --token <TOKEN>            [env: IPDB_SERVER_TOKEN=]
  -l, --log-config <LOG_CONFIG>  log config, default None, use default config, control level
                                 by following options [env: IPDB_SERVER_LOG_CONFIG=]
  -v, --verbose...               More output per occurrence
  -q, --quiet...                 Less output per occurrence
  -h, --help                     Print help
  -V, --version                  Print version
```

## API

- `/`: Root, show ipdb server version.
- `/ip`: `POST` with `application/json` or `application/x-www-form-urlencoded`.
  - Parameters:
    - `ip`: required, `String`, IPv4 or IPv6 String
    - `language`: required, `String`, e.g. "CN" "EN"
    - `token`: optional if server not set, `String`, if server set but client do not send, respond 401 Unauthorized.
  - Response:
    - `ok`: True if ok, false if there is an error.
    - `error`: null if not `ok`, `String`, error message.
    - `fields`: null if `ok`, `Map`, IPDB main info
