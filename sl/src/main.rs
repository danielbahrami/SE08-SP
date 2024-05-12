mod lock;
mod mqtt;
mod state;
mod wifi;

use crate::lock::SmartLock;
use crate::state::State::*;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, nvs::EspDefaultNvsPartition,
};
use log::*;
use std::sync::{mpsc, Arc, Mutex};

// Wi-Fi
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

// MQTT
const MQTT_BROKER: &str = env!("MQTT_BROKER");
const MQTT_COMMAND_TOPIC: &str = env!("MQTT_COMMAND_TOPIC");
const MQTT_HEARTBEAT_TOPIC: &str = env!("MQTT_HEARTBEAT_TOPIC");
const MQTT_CLIENT_ID: &str = env!("MQTT_CLIENT_ID");
const MQTT_HEARTBEAT_FREQUENCY_MS: &str = env!("MQTT_HEARTBEAT_FREQUENCY_MS");

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Channel for sending events to smart lock
    let (event_tx, event_rx) = mpsc::channel::<String>();

    let peripherals = Peripherals::take().unwrap();
    let event_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut smart_lock = SmartLock::new();
    smart_lock.link_channel(event_tx.clone().into());
    // Configure the smart lock with states and transitions
    // Sim transitions, will simulate states which trigger a process
    smart_lock
        .add_transition(NONE, "init", INITIALIZING)
        .add_transition(INITIALIZING, "ready", LOCKED)
        .add_transition(INITIALIZING, "err-wifi", ERROR)
        .add_transition(INITIALIZING, "err-mqtt", ERROR)
        .add_sim_transition(LOCKED, "unlock", UNLOCKING, 1500, "unlock-success")
        .add_transition(UNLOCKING, "unlock-success", UNLOCKED)
        .add_transition(UNLOCKING, "unlock-failure", ERROR)
        .add_sim_transition(UNLOCKED, "lock", LOCKING, 1500, "lock-success")
        .add_transition(LOCKING, "lock-success", LOCKED)
        .add_transition(LOCKING, "lock-failure", ERROR)
        .add_sim_transition(ERROR, "reset", LOCKING, 1500, "lock-success");

    // Wrap smart lock for access between threads
    let smart_lock_ = Arc::new(Mutex::new(smart_lock));

    // Setup LED
    let (red_pin, green_pin, blue_pin) = match SmartLock::setup_led(
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
        peripherals.ledc.channel1,
        peripherals.ledc.channel2,
        peripherals.pins.gpio25,
        peripherals.pins.gpio32,
        peripherals.pins.gpio33,
    ) {
        Ok(values) => values,
        Err(e) => {
            error!("Failed to setup GPIO for LEDs\n{e}");
            return;
        }
    };

    // Run the smart lock. This will run the LED and listen for events to cause transitions
    SmartLock::run(smart_lock_.clone(), event_rx, red_pin, green_pin, blue_pin);

    // Notify the smart lock that initiation has begun
    event_tx.send("init".to_string()).unwrap();

    // Setup Wi-Fi connection
    let _wifi = match wifi::setup_wifi(WIFI_SSID, WIFI_PASSWORD, peripherals.modem, event_loop, nvs)
    {
        Ok(wifi) => wifi,
        Err(e) => {
            error!("Please check if Wi-Fi SSID and password are correct\n{e}");
            event_tx.send("err-wifi".to_string()).unwrap();
            return;
        }
    };

    // Setup MQTT connection
    let (mqtt_client, mqtt_conn) = match mqtt::setup_mqtt(MQTT_BROKER, MQTT_CLIENT_ID) {
        Ok(values) => values,
        Err(e) => {
            error!("Please check if address to MQTT broker is correct\n{e}");
            event_tx.send("err-mqtt".to_string()).unwrap();
            return;
        }
    };

    // Run and handle MQTT subscriptions and publications
    mqtt::handle_communication(mqtt_client, mqtt_conn, event_tx, smart_lock_);
}
