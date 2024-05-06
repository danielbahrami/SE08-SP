use esp_idf_svc::eventloop::{EspEventLoop, System};
use esp_idf_svc::hal::modem;
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::nvs::{EspNvsPartition, NvsDefault};
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use log::info;

pub fn setup_wifi(
    ssid: &str,
    password: &str,
    modem: impl Peripheral<P = modem::Modem> + 'static,
    event_loop: EspEventLoop<System>,
    nvs: EspNvsPartition<NvsDefault>
) -> Result<BlockingWifi<EspWifi<'static>>, EspError> {
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, event_loop.clone(), Some(nvs)).unwrap(),
        event_loop,
    )?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        password: password.try_into().unwrap(),
        auth_method: AuthMethod::None,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;
    info!("Connected");
    Ok(wifi)
}