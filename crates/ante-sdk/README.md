# ante-sdk

Rust SDK for Ante.

A `Client` is one connection to an Ante host. It sends `Op`s and receives
`EventMsg`s; which session it drives is decided by the ops sent over it, never
by how it was opened.

```rust
let client = ante_sdk::connect("stdio".parse()?, ConnectOptions::default()).await?;
```

An `Endpoint` names where a host is reachable, never a session:

| Endpoint | The client | Host lifetime |
| --- | --- | --- |
| `stdio` | spawns `ante serve --stdio` as its own child | the connection's |
| `unix:<path>` | dials the socket file of an `ante serve --sock` host | someone else's |
| `ws://<addr>`, `wss://<addr>` | dials an `ante serve --ws` host, presenting `ConnectOptions::token` as its bearer | someone else's |

Plain `ws://` is dialed only to a loopback host; anywhere else must be
`wss://`, so the bearer token never crosses a network in clear. The WebSocket
connector is the `ws` feature, on by default; `wss://` and its TLS stack are
the opt-in `wss` feature.

A process that hosts sessions itself obtains the same `Client` type from its
host directly; the in-process channel carries the same wire types the remote
codecs serialize.

Part of [Ante](https://github.com/AntigmaLabs/ante), a self-contained agent runtime by [Antigma Labs](https://antigma.ai).
