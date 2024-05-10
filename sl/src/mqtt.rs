use crate::lock::SmartLock;
use crate::state::State;
use crate::{MQTT_COMMAND_TOPIC, MQTT_HEARTBEAT_FREQUENCY_MS, MQTT_HEARTBEAT_TOPIC};
use esp_idf_svc::{
    mqtt::client::{
        EspMqttClient, EspMqttConnection,
        EventPayload::{Connected, Published, Received, Subscribed},
        MqttClientConfiguration, QoS,
    },
    sys::EspError,
};
use log::*;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

pub fn setup_mqtt(
    broker_addr: &str,
    client_id: &str,
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let mqtt_cfg = MqttClientConfiguration {
        client_id: Some(client_id),
        ..Default::default()
    };

    let (mqtt_client, mqtt_conn) = EspMqttClient::new(broker_addr, &mqtt_cfg)?;
    Ok((mqtt_client, mqtt_conn))
}

pub fn handle_communication(
    mut mqtt_client: EspMqttClient,
    mqtt_conn: EspMqttConnection,
    state_tx: Sender<State>,
    smart_lock: Arc<Mutex<SmartLock>>,
) {
    // Channel for sending event commands out of the MQTT thread
    let (event_tx, event_rx) = mpsc::channel::<String>();

    // Thread for handling different MQTT events
    spawn_event_thread(mqtt_conn, event_tx);

    mqtt_client
        .subscribe(MQTT_COMMAND_TOPIC, QoS::ExactlyOnce)
        .unwrap();

    // Signal that the INITIALIZATION is done and the device is ready to receive commands
    state_tx.send(State::CLOSED).unwrap();

   // MQTT event thread
thread::spawn(move || {
    let mut lock_state = State::CLOSED; // Initialize lock state as closed

    for msg in event_rx {
        match msg.as_str() {
            "open" => {
                if lock_state == State::CLOSED {
                    state_tx.send(State::OPENING).unwrap();
                    thread::sleep(Duration::from_millis(3000));
                    state_tx.send(State::OPEN).unwrap();
                    lock_state = State::OPEN;
                } else if lock_state == State::OPEN {
                    error!("Lock is already open");
                    state_tx.send(State::ERROR).unwrap();
                    thread::sleep(Duration::from_millis(3000));
                    state_tx.send(State::OPEN).unwrap();
                }
            }
            "close" => {
                if lock_state == State::OPEN {
                    state_tx.send(State::CLOSING).unwrap();
                    thread::sleep(Duration::from_millis(3000));
                    state_tx.send(State::CLOSED).unwrap();
                    lock_state = State::CLOSED;
                } else if lock_state == State::CLOSED {
                    error!("Lock is already closed");
                    state_tx.send(State::ERROR).unwrap();
                    thread::sleep(Duration::from_millis(3000));
                    state_tx.send(State::CLOSED).unwrap();
                }
            }
            cmd => {
                error!("Unknown command {:?}", cmd);
                state_tx.send(State::ERROR).unwrap();
            }
        };
    }
});

    // parse heartbeat frequency or use 1000ms as default
    let heartbeat_freq = MQTT_HEARTBEAT_FREQUENCY_MS
        .parse::<u64>()
        .unwrap_or_else(|_| 1000);
    // Heartbeat loop
    loop {
        thread::sleep(Duration::from_millis(heartbeat_freq));
        mqtt_client
            .publish(
                MQTT_HEARTBEAT_TOPIC,
                QoS::ExactlyOnce,
                false,
                format!(
                    "SmartLock: {:?}, {:?}",
                    smart_lock.lock().unwrap().get_state(),
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis(),
                )
                .as_bytes(),
            )
            .unwrap();
    }
}

fn spawn_event_thread(mut mqtt_conn: EspMqttConnection, event_tx: Sender<String>) {
    thread::spawn(move || {
        while let Ok(event) = mqtt_conn.next() {
            match event.payload() {
                Connected(_) => {
                    info!("Connected");
                }
                Subscribed(_) => {
                    info!("Subscribed");
                }
                Published(_) => {
                    info!("Published");
                }
                Received { data, .. } => {
                    if data != [] {
                        let msg = std::str::from_utf8(data).unwrap();
                        info!("Received data: {}", msg);
                        event_tx.send(msg.to_owned()).unwrap(); // Send data over channel
                    }
                }
                _ => warn!("{:?}", event.payload()),
            };
        }
    });
}
