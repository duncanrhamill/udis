#![doc = include_str!("../readme.md")]
#![warn(
    missing_docs,
    missing_copy_implementations,
    missing_debug_implementations,
    clippy::missing_safety_doc,
    clippy::missing_errors_doc
)]

use std::net::IpAddr;

use builder::Builder;
use serde::{Deserialize, Serialize};

/// Implementation of the async udis endpoint, __Requires the `tokio` feature__
#[cfg(feature = "tokio")]
pub mod async_tokio;

/// Builder struct for the [`Udis`] type
pub mod builder;

/// Defines errors that can occur
pub mod error;

mod net;

/// Implementation of the sync udis endpoint
pub mod sync;

/// The main interface to the udis system.
///
/// This type provides a builder which lets you define:
///  - the name of the udis endpoint
///  - which IP address the endpoint is accessible over
///  - any services the endpoint hosts
///  - and any services the endpoint is searching for
///
/// Once configured, build the endpoint with either [`Builder::build_sync()`] or
/// [`Builder::build_async()`], depending on which style of interface you want to use.
///
/// # Examples
///
/// Building an endpoint which wants to find a service called "hello"
///
/// ```no_run
/// let udis = udis::Udis::new("client")
///     .search("hello")
///     .build_sync()
///     .expect("Failed to build udis endpoint");
///
/// let service = udis.find_service().expect("Failed to find an endpoint with the `hello` service");
///
/// println!(
///     "Found `{}` service hosted by `{}` at {}:{}",
///     service.kind,
///     service.name,
///     service.addr,
///     service.port);
/// ```
///
/// Building an endpoint which advertises a `hello` service on port `4112`:
///
/// ```no_run
/// let udis = udis::Udis::new("server")
///     .host("hello", 4112)
///     .expect("Kind or port already hosted on endpoint")
///     .build_sync()
///     .expect("Failed to build udis endpoint");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Udis {
    name: String,
    addr: IpAddr,
    services: Vec<Service>,
}

/// Contains information on a single discovered service
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceInfo {
    /// The name of the udis endpoint hosting the service
    pub name: String,

    /// The kind of service being hosted
    pub kind: String,

    /// The address of the endpoint hosting the service
    pub addr: IpAddr,

    /// The port number the service is hosted on
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
enum Service {
    Host { kind: String, port: u16 },
    Search { kind: String },
}

impl Udis {
    /// Create a new udis discovery endpoint with the given name. This name will be advertised to
    /// the discovery network.
    #[expect(clippy::new_ret_no_self)]
    pub fn new<S: Into<String>>(name: S) -> Builder {
        Builder::new(name.into())
    }

    pub(crate) fn build(name: String, addr: IpAddr, services: Vec<Service>) -> Self {
        Self {
            name,
            addr,
            services,
        }
    }

    pub(crate) fn get_wanted_services<'a>(
        &'a self,
        peer: &'a Udis,
    ) -> impl Iterator<Item = &'a Service> {
        self.services
            .iter()
            .filter(|s| peer.services.iter().any(|p| s.wanted_by(p)))
    }
}

impl Service {
    fn wanted_by(&self, peer_service: &Service) -> bool {
        if let (
            Service::Host { kind, .. },
            Service::Search {
                kind: peer_wanted_kind,
            },
        ) = (self, peer_service)
        {
            kind == peer_wanted_kind
        } else {
            false
        }
    }
}
