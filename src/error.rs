use crate::ServiceInfo;

/// Enum of errors that might occur in udis usage
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    FmtError(#[from] std::fmt::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("The service `{kind}` on port {port} is a duplicate service, either the kind or port are already in use on this endpoint")]
    DuplicateService { kind: String, port: u16 },

    #[error("Could not get the local IP address")]
    LocalAddrError(#[from] local_ip_address::Error),

    #[error("Action cannot be performed, the background udis thread shutdown")]
    BackgroundThreadShutdown,

    #[error("Failed to receive information about any services")]
    ServiceInfoRecvError(#[from] std::sync::mpsc::RecvError),

    #[error("Failed to serialise udis notify message")]
    FailedToSerialiseNotifyMsg(#[source] serde_json::Error),

    #[error("Failed to deserialise udis notify message")]
    FailedToDeserialiseNotifyMsg(#[source] serde_json::Error),

    #[error("Failed to send service information to the main thread")]
    FailedToSendServiceInfo(#[from] std::sync::mpsc::SendError<ServiceInfo>),

    #[cfg(feature = "tokio")]
    #[error("Failed to send service information to the main thread")]
    FailedToSendServiceInfoTokio(#[from] tokio::sync::mpsc::error::SendError<ServiceInfo>),

    #[error("Failed to shutdown the udis background thread")]
    FailedToShutdownUdisThread,

    #[cfg(feature = "tokio")]
    #[error("Failed to shutdown the udis background tokio task")]
    FailedToShutdownUdisTask,

    #[cfg(feature = "tokio")]
    #[error("Failed to join the udis background tokio task")]
    FailedToJoinUdisTask(#[from] tokio::task::JoinError),

    #[error("Service info channel closed, the udis task has stopped")]
    ServiceInfoChannelClosed,
}
