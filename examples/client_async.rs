use std::time::Duration;

use log::{error, info};
use udis::Udis;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    // Build the udis endpoint.
    //
    // Here we setup what we want our endpoint to be called ("client"), and tell the builder that
    // we're interested in searching for services with the kind `hello`.
    //
    // There are two types of udis available, sync and async, which you choose with one of the
    // build functions. The async version will spawn a new tokio task and communicate any found
    // services to this task via channels, using the [`AsyncUdis::find_service()`] function.
    //
    // The sync version is shown in `client.rs`
    let mut udis = Udis::new("client").search("hello").build_async()?;

    // Vector to collect our found services into
    let mut services = Vec::new();

    info!("Discovering services");

    // Look for services until we reach the given timeout
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match udis.find_service().await {
                Ok(serv_info) => {
                    info!(
                        "Found service `{}` hosted by `{}` at {}:{}",
                        serv_info.kind, serv_info.name, serv_info.addr, serv_info.port
                    );

                    services.push(serv_info);
                }
                Err(e) => {
                    error!("err: {e}");
                    break;
                }
            }
        }
    })
    .await
    .ok();

    if services.is_empty() {
        info!("Found no services");
    }

    // Gracefully shutdown the udis endpoint
    udis.shutdown().await?;

    Ok(())
}
