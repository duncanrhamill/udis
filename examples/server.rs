use std::time::Duration;

use udis::Udis;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    // Advertise ourselves using the udis endpoint.
    //
    // Here we build our endpoint by giving it a name ("server"), and telling it that we are
    // hosting a service, in this case one with the kind of "hello" which we will make available on
    // port 4112.
    let udis = Udis::new("server").host("hello", 4112)?.build_sync()?;

    // Wait for receipt
    std::thread::sleep(Duration::from_secs(10));

    // Shutdown the udis endpoint
    udis.shutdown()?;

    Ok(())
}
