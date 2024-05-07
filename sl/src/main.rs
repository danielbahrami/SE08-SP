mod state;
mod wifi;
mod mqtt;
mod lock;

use std::sync::{Arc, mpsc, Mutex};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripherals::Peripherals,
    nvs::EspDefaultNvsPartition,
};
use log::*;
use crate::lock::SmartLock;
use crate::state::State;

// GREED = 32
// RED = 33
// BLUE = 25

// WiFi
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

// MQTT
const MQTT_BROKER: &str = env!("MQTT_BROKER");
const MQTT_COMMAND_TOPIC: &str = env!("MQTT_COMMAND_TOPIC");
const MQTT_RESPONSE_TOPIC: &str = env!("MQTT_RESPONSE_TOPIC");
const MQTT_CLIENT_ID: &str = "ESP32";


fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let (state_tx, state_rx) = mpsc::channel::<State>();

    let peripherals = Peripherals::take().unwrap();
    let event_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let smart_lock = Arc::new(Mutex::new(SmartLock::new()));
    let smart_lock_arc = smart_lock.clone();

    // Setup LEDs
    let (red_pin, green_pin, blue_pin) =
        match SmartLock::setup_leds(
            peripherals.ledc.timer0,
            peripherals.ledc.channel0,
            peripherals.ledc.channel1,
            peripherals.ledc.channel2,
            peripherals.pins.gpio25,
            peripherals.pins.gpio32,
            peripherals.pins.gpio33
        ) {
        Ok(values) => values,
        Err(e) => {
            error!("Failed to setup GPIO for LEDs\n{e}");
            return
        }
    };

    SmartLock::run(smart_lock, state_rx, red_pin, green_pin, blue_pin);

    state_tx.send(State::INITIALIZING).unwrap();

    // Setup WiFi connection
    let _wifi =
        match wifi::setup_wifi(WIFI_SSID, WIFI_PASSWORD, peripherals.modem, event_loop, nvs) {
        Ok(wifi) => wifi,
        Err(e) => {
            error!("Please check Wi-Fi ssid and password are correct\n{e}");
            state_tx.send(State::ERROR).unwrap();
            return
        }
    };

    // Setup MQTT connection
    let (mqtt_client, mqtt_conn) =
        match mqtt::setup_mqtt(MQTT_BROKER, MQTT_CLIENT_ID) {
        Ok(values) => values,
        Err(e) => {
            error!("Please check address to MQTT is correct\n{e}");
            state_tx.send(State::ERROR).unwrap();
            return
        }
    };

    // Run and handle MQTT subscriptions and publications
    mqtt::handle_communication(mqtt_client, mqtt_conn, state_tx, smart_lock_arc);
}