mod state;
mod wifi;
mod mqtt;
mod lock;

use std::{
    thread,
    sync::mpsc,
    time::{Duration, SystemTime}
};
use std::cmp::PartialEq;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, RecvError, Sender};
use esp_idf_svc::{
    eventloop::{EspEventLoop, System, EspSystemEventLoop},
    hal::{
        adc::{attenuation, AdcChannelDriver, AdcDriver, config::Config, ADC1},
        peripherals::Peripherals, modem, peripheral::Peripheral, gpio::Gpio34
    },
    nvs::{EspDefaultNvsPartition, EspNvsPartition, NvsDefault},
    wifi::{Configuration, EspWifi, ClientConfiguration, AuthMethod, BlockingWifi},
    mqtt::client::{EspMqttClient, MqttClientConfiguration, EspMqttConnection, QoS,
                   EventPayload::{Connected, Published, Received, Subscribed}
    },
    sys::EspError
};
use esp_idf_svc::hal::gpio::{OutputPin, Pin, PinDriver};
use esp_idf_svc::hal::ledc::{config, LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver};
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::prelude::FromValueType;
use esp_idf_svc::sys::sleep;
use log::*;
use crate::state::{make_blue, make_green, make_orange, make_red, make_yellow, State};

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

    let (mut state_tx, state_rx) = mpsc::channel::<State>();

    let peripherals = Peripherals::take().unwrap();
    let event_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    // Setup LEDs
    let (red_pin, green_pin, blue_pin) =
        match setup_leds(
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


    lock::run_leds(state_rx, red_pin, green_pin, blue_pin);

    state_tx.send(State::INITIALIZING).unwrap();

    // Setup WiFi connection
    let _wifi =
        match wifi::setup_wifi(WIFI_SSID, WIFI_PASSWORD, peripherals.modem, event_loop, nvs) {
        Ok(wifi) => wifi,
        Err(e) => {
            error!("Please check Wi-Fi ssid and password are correct\n{e}");
            return
        }
    };

    // Setup MQTT connection
    let (mqtt_client, mqtt_conn) =
        match mqtt::setup_mqtt(MQTT_BROKER, MQTT_CLIENT_ID) {
        Ok(values) => values,
        Err(e) => {
            error!("Please check address to MQTT is correct\n{e}");
            return
        }
    };

    // Run and handle MQTT subscriptions and publications
    mqtt::handle_mqtt(mqtt_client, mqtt_conn, &mut state_tx);
}

fn setup_leds(
    timer0: impl Peripheral<P = impl LedcTimer> + 'static,
    channel0: impl Peripheral<P = impl LedcChannel> + 'static,
    channel1: impl Peripheral<P = impl LedcChannel> + 'static,
    channel2: impl Peripheral<P = impl LedcChannel> + 'static,
    pin25: impl Peripheral<P = impl OutputPin> + 'static,
    pin32: impl Peripheral<P = impl OutputPin> + 'static,
    pin33: impl Peripheral<P = impl OutputPin> + 'static
) -> Result<(LedcDriver<'static>, LedcDriver<'static>, LedcDriver<'static>), EspError> {
    let led_timer = Arc::new(LedcTimerDriver::new(
        timer0,
        &TimerConfig::default().frequency(25.kHz().into())
    )?);
    let red_pin = LedcDriver::new(
        channel0,
        led_timer.clone(),
        pin25
    )?;
    let green_pin = LedcDriver::new(
        channel1,
        led_timer.clone(),
        pin32
    )?;
    let blue_pin = LedcDriver::new(
        channel2,
        led_timer.clone(),
        pin33
    )?;
    Ok((red_pin, green_pin, blue_pin))
}
