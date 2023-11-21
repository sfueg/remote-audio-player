use clap::Parser;

use rodio::Sink;

use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::BufReader;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("localhost"))]
    server: String,

    #[arg(short, long, default_value_t = 1883)]
    port: u16,

    #[arg(short, long, default_value_t = String::from("remoteaudio/commands"))]
    topic: String,

    #[arg(short, long, default_value_t = String::from("remoteaudio"))]
    client: String,

    #[arg(short, long, default_value_t = true)]
    debug: bool,
}

struct State {
    sounds: HashMap<String, Sink>,
    fades: HashMap<String, Fade>,
}

struct Fade {
    from_volume: f32,
    to_volume: f32,
    start_time: Instant,
    duration: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MQTTPlaySoundEvent {
    id: String,
    path: String,
    is_loop: Option<bool>,
    overwrite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MQTTStopSoundEvent {
    id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MQTTSetVolumeEvent {
    id: String,
    volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MQTTFadeToVolumeEvent {
    id: String,
    volume: f32,
    time_in_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum MQTTEvent {
    PlaySound(MQTTPlaySoundEvent),
    StopSound(MQTTStopSoundEvent),
    SetVolume(MQTTSetVolumeEvent),
    FadeToVolume(MQTTFadeToVolumeEvent),
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let (tx, rx) = mpsc::channel::<MQTTEvent>();

    thread::spawn(move || {
        let mut state = State {
            sounds: HashMap::new(),
            fades: HashMap::new(),
        };

        loop {
            let mut remove_fades: Vec<String> = Vec::new();

            let now = Instant::now();
            for (id, fade) in &state.fades {
                let duration = now.duration_since(fade.start_time);
                let duration_ms = duration.as_millis() as f32;

                let sink = state.sounds.get(id);

                if fade.duration < duration_ms {
                    if let Some(sink) = sink {
                        sink.set_volume(fade.to_volume);
                    }

                    remove_fades.push(id.to_string());
                } else {
                    let progress = duration_ms / fade.duration;

                    let current_volume =
                        fade.to_volume * progress + fade.from_volume * (1.0 - progress);

                    if let Some(sink) = sink {
                        sink.set_volume(current_volume);
                    }
                }
            }

            for id in remove_fades {
                state.fades.remove(&id);
            }

            while let Ok(message) = rx.try_recv() {
                match message {
                    MQTTEvent::PlaySound(e) => {
                        if e.overwrite.unwrap_or(true) == false && state.sounds.contains_key(&e.id)
                        {
                            continue;
                        }

                        let file = std::fs::File::open(e.path);

                        match file {
                            Ok(file) => {
                                let sink = if e.is_loop.unwrap_or(false) {
                                    let sink =
                                        stream_handle.play_once(BufReader::new(file)).unwrap();
                                    sink
                                } else {
                                    let source =
                                        rodio::Decoder::new_looped(BufReader::new(file)).unwrap();

                                    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
                                    sink.append(source);
                                    sink
                                };

                                state.sounds.insert(e.id, sink);
                            }
                            Err(e) => {
                                println!("Error reading file: {:?}", e);
                            }
                        }
                    }

                    MQTTEvent::StopSound(e) => {
                        state.sounds.remove(&e.id);
                    }

                    MQTTEvent::SetVolume(e) => {
                        let sink = state.sounds.get(&e.id);

                        if let Some(sink) = sink {
                            sink.set_volume(e.volume);
                        }

                        state.fades.remove(&e.id);
                    }

                    MQTTEvent::FadeToVolume(e) => {
                        let sink = state.sounds.get(&e.id);

                        if let Some(sink) = sink {
                            state.fades.insert(
                                e.id,
                                Fade {
                                    from_volume: sink.volume(),
                                    to_volume: e.volume,
                                    start_time: Instant::now(),
                                    duration: e.time_in_ms,
                                },
                            );
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(1));
        }
    });

    if args.debug {
        println!("Running in Debugmode");
    }

    println!(
        "Connecting to \"{}:{}\" as \"{}\"",
        args.server, args.port, args.client
    );

    let mut mqttoptions = MqttOptions::new(args.client, args.server, args.port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    println!("Subscribing to \"{}\"", args.topic);
    client.subscribe(args.topic, QoS::AtMostOnce).await.unwrap();

    while let Ok(notification) = eventloop.poll().await {
        match notification {
            rumqttc::Event::Incoming(packet) => match packet {
                rumqttc::Packet::Publish(publish) => {
                    let str = std::str::from_utf8(&publish.payload).unwrap();
                    let e = serde_json::from_str::<MQTTEvent>(str);

                    match e {
                        Ok(e) => {
                            if args.debug {
                                println!("{:?}", e);
                            }
                            tx.send(e).unwrap();
                        }
                        Err(e) => {
                            println!("{:?}", e);
                        }
                    }
                }
                _ => {}
            },
            rumqttc::Event::Outgoing(_) => {}
        }
    }
}
