use std::time::{Duration, Instant};

use log::info;
use udis::Udis;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    // Build the udis endpoint.
    //
    // Here we setup what we want our endpoint to be called ("client"), and tell the builder that
    // we're interested in searching for services with the kind `hello`.
    //
    // There are two types of udis available, sync and async, which you choose with one of the
    // build functions. The sync version will spawn a new thread and communicate any found services
    // to this thread via channels, using the [`SyncUdis::find_service()`] or
    // [`SyncUdis::try_find_service()`] functions.
    //
    // The async version is shown in `client_async.rs`
    let udis = Udis::new("client").search("hello").build_sync()?;

    // Vector to collect our found services into
    let mut services = Vec::new();

    info!("Discovering services");

    // Wait for discovered services
    let now = Instant::now();
    let timeout = Duration::from_secs(5);
    while now.elapsed() < timeout {
        // The try_find_service function will look to see if any of our `search`ed services have
        // been found, if so it will return `Ok(Some(ServiceInfo))`. The service info struct
        // contains the service kind, the name of the endpoint hosting that service, and the socket
        // address that the endpoint wants you to use for that service
        if let Ok(Some(serv_info)) = udis.try_find_service() {
            info!(
                "Found service `{}` hosted by `{}` at {}:{}",
                serv_info.kind, serv_info.name, serv_info.addr, serv_info.port
            );

            services.push(serv_info);
        }

        // (avoid busy looping on the main thread)
        std::thread::sleep(Duration::from_millis(100));
    }

    if services.is_empty() {
        info!("Found no services");
    }

    // Gracefully shutdown the udis endpoint
    udis.shutdown()?;

    Ok(())
}
