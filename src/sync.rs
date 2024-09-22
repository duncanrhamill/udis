use std::{
    collections::HashSet,
    io::ErrorKind,
    sync::mpsc::{channel, Receiver, RecvError, Sender, TryRecvError},
    thread::JoinHandle,
    time::Duration,
};

use log::{error, trace};

use crate::{error::Error, net::build_multicast_socket, Service, ServiceInfo, Udis};

/// A synchronous udis endpoint.
///
/// This endpoint works by starting a background thread that runs the udis network logic, and will
/// communicate any observed services back to the main thread via channels.
///
/// To retrieve services found by this endpoint use the [`SyncUdis::find_service`] or
/// [`SyncUdis::try_find_service`] functions.
///
/// When finished using the endpoint be sure to call [`SyncUdis::shutdown`] to close the background
/// thread.
#[derive(Debug)]
pub struct SyncUdis {
    /// The common udis info
    _udis: Udis,

    /// Join handle for the background thread
    bg_thread_jh: JoinHandle<Result<(), Error>>,

    /// Channel for sending commands to the bg thread
    cmd_tx: Sender<Cmd>,

    /// Service info receive channel, the BG thread will send discovered services over this channel
    /// back to the [`SyncUdis`] endpoint
    serv_info_rx: Receiver<ServiceInfo>,
}

enum Cmd {
    Shutdown,
}

impl SyncUdis {
    pub(crate) fn build(udis: Udis) -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (serv_info_tx, serv_info_rx) = channel();

        let udis_bg = udis.clone();

        let bg_thread_jh =
            std::thread::spawn(move || sync_bg_thread(udis_bg, cmd_rx, serv_info_tx));

        Self {
            _udis: udis,
            bg_thread_jh,
            cmd_tx,
            serv_info_rx,
        }
    }

    /// Find the next service discovered by this udis endpoint.
    ///
    /// This function will block until a service is found.
    ///
    /// # Errors
    ///
    /// This function can return an error if the background thread closes for an unexpected reason.
    pub fn find_service(&self) -> Result<ServiceInfo, Error> {
        if self.bg_thread_jh.is_finished() {
            return Err(Error::BackgroundThreadShutdown);
        }

        let serv_info = self.serv_info_rx.recv()?;

        Ok(serv_info)
    }

    /// Try to find the next service discovered by the udis endpoint.
    ///
    /// This function will not block, if no service is found `Ok(None)` will be returned.
    ///
    /// # Errors
    ///
    /// This function can return an error if the background thread closes for an unexpected reason.
    pub fn try_find_service(&self) -> Result<Option<ServiceInfo>, Error> {
        if self.bg_thread_jh.is_finished() {
            return Err(Error::BackgroundThreadShutdown);
        }

        match self.serv_info_rx.try_recv() {
            Ok(serv_info) => Ok(Some(serv_info)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(Error::ServiceInfoRecvError(RecvError)),
        }
    }

    /// Shutdown this endpoint
    ///
    /// # Errors
    ///
    /// This function can return an error if the background thread closes for an unexpected reason.
    pub fn shutdown(self) -> Result<(), Error> {
        self.cmd_tx
            .send(Cmd::Shutdown)
            .map_err(|_| Error::FailedToShutdownUdisThread)?;

        self.bg_thread_jh
            .join()
            .map_err(|_| Error::FailedToShutdownUdisThread)??;

        Ok(())
    }
}

/// Background thread for the [`SyncUdis`] endpoint
fn sync_bg_thread(
    udis: Udis,
    cmd_rx: Receiver<Cmd>,
    serv_info_tx: Sender<ServiceInfo>,
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

    // Build the registry of udis peers
    let mut registry = HashSet::<Udis>::new();

    // Build the notify message
    let notify_message = serde_json::to_vec(&udis).map_err(Error::FailedToSerialiseNotifyMsg)?;

    // Send our notify message as we're joining the network
    socket.send_to(&notify_message[..], &disc_addr.into())?;

    // Receive buffer
    let mut buf = Vec::with_capacity(1024);

    // Main loop
    loop {
        // Check if there's a command
        match cmd_rx.try_recv() {
            Ok(cmd) => match cmd {
                Cmd::Shutdown => break,
            },
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break,
        }

        // Wait so we're not busy blocking the thread
        std::thread::sleep(Duration::from_millis(100));

        // Try to receive a packet on the discovery socket
        let received = match socket.recv(buf.spare_capacity_mut()) {
            Ok(a) => a,
            Err(e) => {
                match e.kind() {
                    ErrorKind::TimedOut | ErrorKind::WouldBlock => (),
                    k => error!(
                        "Error while receiving udis notify messages (will continue): ({k:?}) {e}"
                    ),
                }
                continue;
            }
        };
        // SAFETY: just received into the `buffer`.
        unsafe {
            buf.set_len(received);
        }

        // Decode into a udis struct
        let peer: Udis =
            serde_json::from_slice(&buf[..]).map_err(Error::FailedToDeserialiseNotifyMsg)?;

        // Clear the buffer
        buf.clear();

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

        // If the peer is interested in one of the services we're offering notify it directly
        if udis.get_wanted_services(&peer).count() > 0 {
            trace!(
                "notified of peer `{}` that wants one of our services",
                peer.name
            );

            socket.send_to(&notify_message[..], &disc_addr.into())?;
        }

        // If the peer has one of the services we're interested in
        for service in peer.get_wanted_services(&udis) {
            let Service::Host { kind, port } = service else {
                trace!("Non-host service returned by get_wanted_services, skipping");
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

    trace!("udis background task shutting down");

    Ok(())
}
