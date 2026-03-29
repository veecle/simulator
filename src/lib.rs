use core::convert::Infallible;
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncReadExt, net::UnixSocket, select};
use veecle_os::runtime::{Reader, Storable, Writer};

#[derive(Debug, Clone, PartialEq, Default, Storable)]
pub struct Throttle(pub f32);

#[derive(Debug, Clone, PartialEq, Default, Storable)]
pub struct Brake(pub f32);

#[derive(Debug, Clone, PartialEq, Default, Storable)]
pub struct Steering(pub f32);

#[derive(Debug, Clone, PartialEq, Default, Storable)]
pub struct ParkingBrake(pub f32);

#[derive(Debug, Default, Deserialize, Serialize)]
struct SimulationControl {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    throttle: Option<f32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    brake: Option<f32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    steering: Option<f32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    parkingbrake: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Sensor {
    name: String,
    value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Default, Storable)]
pub struct Speed(pub f32);

#[veecle_os::runtime::actor]
#[veecle_os::telemetry::instrument]
pub async fn cruise_control_simulator(
    mut throttle_reader: Reader<'_, Throttle>,
    mut speed_writer: Writer<'_, Speed>,
) -> Infallible {
    let path = "/opt/simulation.sock";
    let socket = UnixSocket::new_stream().expect("opening runtime socket");
    let mut runtime_stream = socket
        .connect(path)
        .await
        .expect("connecting runtime socket");

    loop {
        veecle_os::debug!("Waiting for value");

        let mut sensor_buffer = [0; 1024];

        let throttle_future = throttle_reader.wait_for_update();
        let stream_future = runtime_stream.read(&mut sensor_buffer);

        select! {
            reader = throttle_future => reader.read(|value| {
                let mut json = serde_json::to_string(&SimulationControl {
                    throttle: Some(value.unwrap().0),
                    ..Default::default()
                }).unwrap();

                // We needa way to split the control data in the runtime, so we add a padding
                // character that doesn't appear in the sensor data natively.
                json.push('\n');

                if let Err(error) = runtime_stream.try_write(json.as_bytes()) {
                    veecle_os::error!("failed to write similuation control", error = error.to_string());
                }
            }),
            result = stream_future => {
                let Ok(bytes_read) = result else {
                    veecle_os::error!("failed to read from sensor stream");
                        std::process::exit(1);
                };

                if bytes_read == 0 {
                    veecle_os::error!("sensor stream disconnected");
                    std::process::exit(1);
                }

                let Ok(json) = std::str::from_utf8(&sensor_buffer[..bytes_read]) else {
                    veecle_os::error!("sensor data is not valid UTF8");
                    continue;
                };

                for sensor_json in json.split('\n').filter(|part| !part.is_empty()) {
                    let Ok(sensor_data) = serde_json::from_str::<Sensor>(sensor_json) else {
                        veecle_os::error!("sensor data is not valid");
                        continue;
                    };

                    veecle_os::info!("got sensor data", name = sensor_data.name.clone());

                    if sensor_data.name.as_str() == "speed" {
                        let Some(value) = sensor_data.value.as_number().and_then(|number| number.as_f64()) else {
                            veecle_os::error!("invalid speed value");
                            continue;
                        };

                        speed_writer.write(Speed(value as f32)).await;
                    }
                }
            }
        }
    }
}
