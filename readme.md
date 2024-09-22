# `udis` - a tiny local service descovery system

[![Crates.io Version](https://img.shields.io/crates/v/udis)](https://crates.io/crates/udis/0.1.0) [![docs.rs](https://img.shields.io/docsrs/udis)](https://docs.rs/udis/latest/udis/)

There are many service discovery options available out there, but when it comes
to local service discovery we mainly reach for mDNS. In most cases this is
great, but sometimes it's nice just to have a really simple solution that isn't
managed by the OS in any way.

`udis` (_micro discovery_) is a dead simple system for providing local service
discovery with minimal fuss. The main use case is:

> To find servers on the local network that host a kind of service on a
> particular port.

To achieve this, `udis` uses UDP multicast to send notifications to a
"discovery network" - basically anyone who's listening - which include some
very basic information:

- the name of the endpoint, which is just some description of the application,
  like "server" or "client",
- the IP address of the machine that the endpoint is hosted on
- and a list of services, where each service is either:
  - one the endpoint is searching for (i.e. "I want to find a service")
  - or one the endpoint is actively hosting (i.e. "I can offer you a service")

Each endpoint in the network will actively listen for notifications, and if it
receives one it's interested in (e.g. a server receives a notification from a
client looking for one of its services) the endpoint will re-notify the network
of its presence. This means the order in which endpoints join the network
doesn't matter.

Once an endpoint has found a service its interested in it's the users job to
continue setting up communications, e.g. using the IP address and port number
to connect their own socket.

## Example usage

Create a server which hosts a "hello" service on port 4112:

```rust
let udis = udis::Udis::new("server").host("hello", 4112)?.build_sync()?;
```

And then create a client which searches for the "hello" service:

```rust
let udis = udis::Udis::new("client")
    .search("hello")
    .build_sync()
    .expect("Failed to build udis endpoint");

let service = udis.find_service().expect("Failed to find an endpoint with the `hello` service");

// prints: Found `hello` service hosted by `server` at 192.168.0.1:4112
println!(
    "Found `{}` service hosted by `{}` at {}:{}",
    service.kind,
    service.name,
    service.addr,
    service.port);
```

Only the `client` side receives notifications of services its interested in, so
the job of opening communications goes to the side that searched for the
service, not the service hoster.

For a complete example see the [`examples/client.rs`](examples/client.rs) and
[`examples/server.rs`](examples/server.rs`) example files.

## Under the hood

### Sync

Under the hood the `SyncUdis` endpoint type is implemented with a background
thread that communicates to the main thread with channels.

### Async

`udis` supports async with the `tokio` runtime, which can be enabled with the
`tokio` feature. When enabled you can use the `build_async` function on the
`udis::builder::Builder` struct which creates a tokio task rather than a thread.

## Discovery notification packets

The udis notification packet is a simple JSON one, for example a server hosting
a service of the kind "hello" on port 4112 might send this packet out:

```json
{
    "name": "server",
    "addr": "192.168.0.1",
    "services": [
        {
            "Host": {
                "kind": "hello",
                "port": 4112
            }
        }
    ]
}
```
