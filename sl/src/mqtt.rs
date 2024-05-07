use std::{sync::mpsc::{self, Sender}, thread, time::Duration};
use std::sync::{Arc, Mutex};
use esp_idf_svc::{
    mqtt::client::{
        EspMqttClient,
        EspMqttConnection,
        MqttClientConfiguration,
        QoS,
        EventPayload::{Connected, Published, Received, Subscribed}
    },
    sys::EspError
};
use log::*;
use crate::{MQTT_COMMAND_TOPIC, MQTT_RESPONSE_TOPIC};
use crate::lock::SmartLock;
use crate::state::State;

pub fn setup_mqtt(broker_addr: &str, client_id: &str)
                  -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let mqtt_cfg = MqttClientConfiguration {
        client_id: Some(client_id),
        ..Default::default()
    };

    let (mqtt_client, mqtt_conn) =
        EspMqttClient::new(broker_addr, &mqtt_cfg)?;
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

    mqtt_client.subscribe(MQTT_COMMAND_TOPIC, QoS::ExactlyOnce).unwrap();

    // Signal that the INITIALIZATION is done and the device is ready to receive commands
    state_tx.send(State::CLOSED).unwrap();

    thread::spawn(move || {
        loop {
            // Handle the different commands from the MQTT event thread
            match event_rx.try_recv() { // Receive data from channel
                Ok(msg) => {
                    match msg.as_str() {
                        "open" => {
                            state_tx.send(State::OPENING).unwrap();
                            thread::sleep(Duration::from_millis(1000));
                            state_tx.send(State::OPEN).unwrap();
                        },
                        "close" => {
                            state_tx.send(State::CLOSING).unwrap();
                            thread::sleep(Duration::from_millis(1000));
                            state_tx.send(State::CLOSED).unwrap();
                        },
                        cmd => {
                            error!("Unknown command {:?}", cmd);
                            state_tx.send(State::ERROR).unwrap();
                        }
                    };
                },
                Err(_) => continue,
            }
        }
    });

    loop {
        thread::sleep(Duration::from_millis(1000));
        mqtt_client.publish(
            MQTT_RESPONSE_TOPIC,
            QoS::ExactlyOnce,
            false,
            format!("SmartLock: {:?}", smart_lock.lock().unwrap().get_state()).as_bytes(),
        ).unwrap();
    }
}

fn spawn_event_thread(mut mqtt_conn: EspMqttConnection, event_tx: Sender<String>) {
    thread::spawn(move || {
        while let Ok(event) = mqtt_conn.next() {
            match event.payload() {
                Connected(_) => { info!("Connected"); },
                Subscribed(_) => { info!("Subscribed"); },
                Published(_) => { info!("Published"); },
                Received { data, .. } => {
                    if data != [] {
                        let msg = std::str::from_utf8(data).unwrap();
                        info!("Received data: {}", msg);
                        event_tx.send(msg.to_owned()).unwrap(); // Send data over channel
                    }
                }
                _ => warn!("{:?}", event.payload())
            };
        }
    });
}