// Copyright 2020 David Li
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Route guide client.

use futures::{Future, Sink, Stream};
use grpcio::{ChannelBuilder, Environment, WriteFlags};
use rand::{seq::SliceRandom, Rng};
use route::route_guide::*;
use route::route_guide_grpc::RouteGuideClient;
use route::util;
use std::sync::Arc;
use std::time::Duration;

fn new_point(latitude: i32, longitude: i32) -> Point {
    Point {
        latitude,
        longitude,
        ..Default::default()
    }
}

fn new_rectangle(
    lo_latitude: i32,
    lo_longitude: i32,
    hi_latitude: i32,
    hi_longitude: i32,
) -> Rectangle {
    let mut rect = Rectangle::default();
    rect.set_lo(new_point(lo_latitude, lo_longitude));
    rect.set_hi(new_point(hi_latitude, hi_longitude));
    rect
}

fn new_route_note<T: ToString>(latitude: i32, longitude: i32, message: T) -> RouteNote {
    let mut note = RouteNote::default();
    note.set_location(new_point(latitude, longitude));
    note.set_message(message.to_string());
    note
}

struct Client {
    client: RouteGuideClient,
}

impl Client {
    fn new<T: AsRef<str>>(addr: T) -> Self {
        let env = Arc::new(Environment::new(1));
        let channel = ChannelBuilder::new(env).connect(addr.as_ref());
        let client = RouteGuideClient::new(channel);
        Self { client }
    }

    fn get_feature(&self, point: &Point) {
        let feature = self
            .client
            .get_feature(point)
            .expect("Failed to get feature");

        if !feature.has_location() {
            eprintln!("Server returns incomplete feature.");
            return;
        }

        if feature.get_name().is_empty() {
            println!("No feature found at {}", util::format_point(point));
            return;
        }

        println!(
            "Found feature {} at {}",
            feature.get_name(),
            util::format_point(point)
        );
    }

    fn list_features(&self, rect: &Rectangle) {
        println!(
            "Searching features between {} and {}",
            util::format_point(rect.get_lo()),
            util::format_point(rect.get_hi()),
        );

        let mut features = self
            .client
            .list_features(rect)
            .expect("Failed to list features");
        loop {
            match features.into_future().wait() {
                Ok((Some(feature), s)) => {
                    features = s;
                    let location = feature.get_location();
                    println!(
                        "Found feature {} at {}",
                        feature.get_name(),
                        util::format_point(location)
                    );
                }
                Ok((None, _)) => break,
                Err((e, _)) => panic!("Failed to list features: {:?}", e),
            }
        }
        println!("List features successfully!");
    }

    fn record_route(&self) {
        let db = util::load_database();
        let mut rng = rand::thread_rng();
        let (mut sink, recv) = self.client.record_route().expect("Failed to record route");

        for _ in 0..10 {
            let feature = db.get_feature().choose(&mut rng).unwrap();
            let location = feature.get_location();
            println!("Visiting {}", util::format_point(&location));

            sink = sink
                .send((location.clone(), WriteFlags::default()))
                .wait()
                .unwrap();
            std::thread::sleep(Duration::from_millis(rng.gen_range(500, 1500)));
        }

        futures::future::poll_fn(|| sink.close()).wait().unwrap();

        let sum = recv.wait().unwrap();

        println!("Finished trip, route summary:");
        println!("\tVisited {} points", sum.get_point_count());
        println!("\tPassed {} features", sum.get_feature_count());
        println!("\tTravelled {} meters", sum.get_distance());
        println!("\tTook {} seconds", sum.get_elapsed_time());
    }

    fn route_chat(&self) {
        let (mut sink, mut recv) = self.client.route_chat().expect("Failed to route chat");

        let thread = std::thread::spawn(move || {
            let notes = vec![
                ("First message", 0, 0),
                ("Second message", 0, 1),
                ("Third message", 1, 0),
                ("Fourth message", 0, 0),
            ];

            for (msg, lat, lon) in notes {
                let note = new_route_note(lat, lon, msg);
                println!("Sending message {} at ({},{})", msg, lat, lon);
                sink = sink.send((note, WriteFlags::default())).wait().unwrap();
            }
            futures::future::poll_fn(|| sink.close()).wait().unwrap();
        });

        loop {
            match recv.into_future().wait() {
                Ok((Some(note), rx)) => {
                    let location = note.get_location();
                    println!(
                        "Got message {} at {}",
                        note.get_message(),
                        util::format_point(location)
                    );
                    recv = rx;
                }
                Ok((None, _)) => break,
                Err((e, _)) => panic!("Failed to route chat: {:?}", e),
            }
        }

        thread.join().unwrap();
    }
}

fn main() {
    let client = Client::new("127.0.0.1:8980");

    println!("Get Feature:");
    // Looking for a valid feature
    client.get_feature(&new_point(409146138, -746188906));
    // Feature missing.
    client.get_feature(&new_point(0, 0));

    println!();
    println!("List features:");
    client.list_features(&new_rectangle(400000000, -750000000, 420000000, -730000000));

    println!();
    println!("Record route:");
    client.record_route();

    println!();
    println!("Route chat:");
    client.route_chat();
}
