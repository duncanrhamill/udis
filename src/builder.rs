use std::net::IpAddr;

use crate::{error::Error, sync::SyncUdis, Service, Udis};

#[cfg(feature = "tokio")]
use crate::async_tokio::AsyncUdis;

/// A builder struct for a udis endpoint.
///
/// This struct allows you to configure the udis endpoint, see [`Udis`] for the configuration
/// options, or see the functions defined on this type.
#[derive(Debug, Clone)]
pub struct Builder {
    name: String,
    addr: Option<IpAddr>,
    services: Vec<Service>,
}

impl Builder {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            addr: None,
            services: Vec::new(),
        }
    }

    /// Set the IP address that this discovery endpoint will be visible on.
    ///
    /// If not set the current machine's IP address (as determined by
    /// [`local_ip_address::local_ip()`] will be used, which is always IPv4).
    pub fn addr<I>(mut self, ip: I) -> Self
    where
        I: Into<IpAddr>,
    {
        self.addr = Some(ip.into());
        self
    }

    /// Make a service available on this endpoint, i.e. say that we are hosting a service.
    ///
    /// `kind` is the name for the service type, which is hosted on this machine on the given
    /// `port`.
    ///
    /// # Errors
    ///
    /// Can fail if the given `kind` or `port` are already hosted on this endpoint.
    pub fn host<S: Into<String>>(mut self, kind: S, port: u16) -> Result<Self, Error> {
        let kind = kind.into();

        if self.services.iter().any(|s| {
            if let Service::Host { kind: k, port: p } = s {
                *k == kind || *p == port
            } else {
                false
            }
        }) {
            Err(Error::DuplicateService { kind, port })
        } else {
            self.services.push(Service::Host { kind, port });
            Ok(self)
        }
    }

    /// Search for a service kind with this endpoint.
    pub fn search<S: Into<String>>(mut self, kind: S) -> Self {
        self.services.push(Service::Search { kind: kind.into() });
        self
    }

    /// Build a sync udis endpoint
    ///
    /// # Errors
    ///
    /// This function will return an error if you did not specify an address using
    /// [`Builder::addr`] and the local IP address of this machine can't be determined.
    pub fn build_sync(self) -> Result<SyncUdis, Error> {
        // If there is no addr use the local one
        let addr = match self.addr {
            Some(addr) => addr,
            None => local_ip_address::local_ip()?,
        };

        Ok(SyncUdis::build(Udis::build(self.name, addr, self.services)))
    }

    /// Build an async udis endpoint
    ///
    /// __Requires the `tokio` feature.__
    ///
    /// # Errors
    ///
    /// This function will return an error if you did not specify an address using
    /// [`Builder::addr`] and the local IP address of this machine can't be determined.
    #[cfg(feature = "tokio")]
    pub fn build_async(self) -> Result<AsyncUdis, Error> {
        // If there is no addr use the local one
        let addr = match self.addr {
            Some(addr) => addr,
            None => local_ip_address::local_ip()?,
        };

        Ok(AsyncUdis::build(Udis::build(
            self.name,
            addr,
            self.services,
        )))
    }
}
