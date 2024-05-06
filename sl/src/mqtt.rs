use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use esp_idf_svc::mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration, QoS};
use esp_idf_svc::mqtt::client::EventPayload::{Connected, Published, Received, Subscribed};
use esp_idf_svc::sys::EspError;
use log::*;
use crate::MQTT_COMMAND_TOPIC;
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

pub fn handle_mqtt(
    mut mqtt_client: EspMqttClient,
    mqtt_conn: EspMqttConnection,
    state_tx: &mut Sender<State>
) {
    // Channel for sending event commands out of the MQTT thread
    let (event_tx, event_rx) = mpsc::channel::<String>();

    // Thread for handling different MQTT events
    spawn_event_thread(mqtt_conn, event_tx);

    mqtt_client.subscribe(MQTT_COMMAND_TOPIC, QoS::ExactlyOnce).unwrap();

    state_tx.send(State::CLOSED).unwrap();

    // Handle the different commands from the MQTT event thread
    for msg in event_rx { // Receive data from channel
        /*let cmd_arr = msg.split(":").collect::<Vec<&str>>();
        if cmd_arr.len() <= 1 {
            error!("Invalid command string {:?}", msg);
            continue;
        }*/
        match msg.as_str() {
            "open" => open_cmd(state_tx),
            "close" => close_cmd(state_tx),
            cmd => {
                error!("Unknown command {:?}", cmd);
                state_tx.send(State::ERROR).unwrap();
            }
        };
    }
}


// Simulate the lock opening
fn open_cmd(state_tx: &mut Sender<State>) {
    state_tx.send(State::OPENING).unwrap();
    thread::sleep(Duration::from_millis(1000));
    state_tx.send(State::OPEN).unwrap();
}

// Simulate the lock closing
fn close_cmd(state_tx: &mut Sender<State>) {
    state_tx.send(State::CLOSING).unwrap();
    thread::sleep(Duration::from_millis(1000));
    state_tx.send(State::CLOSED).unwrap();
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