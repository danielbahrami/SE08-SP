use crate::lock::SmartLock;
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
use std::{sync::mpsc::Sender, thread, time::Duration};

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
    event_tx: Sender<String>,
    smart_lock: Arc<Mutex<SmartLock>>,
) {
    // Thread for handling different MQTT events
    spawn_event_thread(mqtt_conn, event_tx);

    mqtt_client
        .subscribe(MQTT_COMMAND_TOPIC, QoS::ExactlyOnce)
        .unwrap();

    // Parse heartbeat frequency or use 3000ms as default
    let heartbeat_freq = MQTT_HEARTBEAT_FREQUENCY_MS
        .parse::<u64>()
        .unwrap_or_else(|_| 3000);

    // Heartbeat loop
    loop {
        thread::sleep(Duration::from_millis(heartbeat_freq));
        mqtt_client
            .publish(
                MQTT_HEARTBEAT_TOPIC,
                QoS::ExactlyOnce,
                false,
                format!("Smart Lock: {:?}", smart_lock.lock().unwrap().get_state(),).as_bytes(),
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
                    event_tx.send("ready".to_string()).unwrap(); // Signal "ready" to smart lock
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
