use std::collections::HashSet;

use crate::{error::Error, net::build_multicast_socket, Service, ServiceInfo, Udis};
use log::{error, trace};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

/// An asynchronous udis endpoint.
///
/// This endpoint works by starting a background tokio task that handles the udis network logic,
/// and communicates discovered services to the main task with channels.
///
/// To retrieve services found by this endpoint use the [`AsyncUdis::find_service`] function.
///
/// When finished using the endpoint be sure to call [`AsyncUdis::shutdown`] to close the background
/// task.
#[derive(Debug)]
pub struct AsyncUdis {
    _udis: Udis,

    // Task join handle
    bg_task_jh: JoinHandle<Result<(), Error>>,

    // Sender for commands
    cmd_tx: UnboundedSender<Cmd>,

    // Receiver for getting service infos from the udis task
    serv_info_rx: UnboundedReceiver<ServiceInfo>,
}

enum Cmd {
    Shutdown,
}

impl AsyncUdis {
    pub(crate) fn build(udis: Udis) -> Self {
        let (cmd_tx, cmd_rx) = unbounded_channel();
        let (serv_info_tx, serv_info_rx) = unbounded_channel();

        let udis_bg = udis.clone();

        let bg_task_jh =
            tokio::task::spawn(async move { async_task(udis_bg, cmd_rx, serv_info_tx).await });

        Self {
            _udis: udis,
            bg_task_jh,
            cmd_tx,
            serv_info_rx,
        }
    }

    /// Find the next service discovered by this udis endpoint.
    ///
    /// # Errors
    ///
    /// This function may return an error if the background task has closed for any reason.
    pub async fn find_service(&mut self) -> Result<ServiceInfo, Error> {
        if let Some(serv_info) = self.serv_info_rx.recv().await {
            Ok(serv_info)
        } else {
            Err(Error::ServiceInfoChannelClosed)
        }
    }

    /// Shutdown this endpoint
    ///
    /// # Errors
    ///
    /// This function may return an error if the background task has closed for any reason.
    pub async fn shutdown(self) -> Result<(), Error> {
        self.cmd_tx
            .send(Cmd::Shutdown)
            .map_err(|_| Error::FailedToShutdownUdisTask)?;

        self.bg_task_jh.await??;

        Ok(())
    }
}

async fn async_task(
    udis: Udis,
    mut cmd_rx: UnboundedReceiver<Cmd>,
    serv_info_tx: UnboundedSender<ServiceInfo>,
) -> Result<(), Error> {
    // Build the multicast socket
    let (disc_addr, socket) = build_multicast_socket()?;
    trace!("joined udis notify network on {disc_addr}");

    for service in &udis.services {
        match service {
            Service::Host { kind, port } => {
                trace!("hosting service `{}` on port {}", kind, port);
            }
            Service::Search { kind } => {
                trace!("searching for service `{}`", kind);
            }
        }
    }

    // Conver the socket to a tokio one
    let socket: tokio::net::UdpSocket = tokio::net::UdpSocket::from_std(socket.into())?;

    // Build the registry of udis peers
    let mut registry = HashSet::<Udis>::new();

    // Build the notify message
    let notify_message = serde_json::to_vec(&udis).map_err(Error::FailedToSerialiseNotifyMsg)?;

    // Send our notify message as we're joining the network
    socket.send_to(&notify_message[..], &disc_addr).await?;

    // Buffer
    let mut buf = [0; 1024];

    // Main loop
    loop {
        // Either receive some data on the socket or a command from the main task
        tokio::select! {
            // On command receipt handle it
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(cmd) => match cmd {
                        Cmd::Shutdown => break,
                    }
                    None => break,
                }
            },

            // On some data from the socket process it
            recv_res = socket.recv(&mut buf) => {
                let recieved = match recv_res {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Error while receving udis notify messages (will continue): {e}");
                        continue;
                    }
                };

                // Decode into a udis struct
                let peer: Udis =
                serde_json::from_slice(&buf[..recieved])
                    .map_err(Error::FailedToDeserialiseNotifyMsg)?;

                // If its our own notify message ignore it
                if peer == udis {
                    continue;
                }

                // If its already in the registry ignore it
                if registry.contains(&peer) {
                    continue;
                }

                // Add the peer to the registry
                registry.insert(peer.clone());

                // If the peer is interested in one of the services we're offering notify it
                if udis.get_wanted_services(&peer).count() > 0 {
                    trace!(
                        "notified of peer `{}` that wants one of our services",
                        peer.name
                    );

                    socket.send_to(&notify_message[..], &disc_addr).await?;
                }

                // If the peer has one of the services we're interested in
                for service in peer.get_wanted_services(&udis) {
                    let Service::Host { kind, port } = service else {
                        trace!("Non-host service returned by get_watned_services, skipping");
                        continue;
                    };

                    trace!(
                        "found peer `{}` that hosts a service we want `{}` at {}:{}",
                        peer.name,
                        kind,
                        peer.addr,
                        port
                    );

                    // Build service info struct
                    let serv_info = ServiceInfo {
                        name: peer.name.clone(),
                        kind: kind.clone(),
                        addr: peer.addr,
                        port: *port,
                    };

                    // Send to the main thread
                    serv_info_tx.send(serv_info)?;
                }
            }
        }
    }

    trace!("udis background task shutting down");

    Ok(())
}
