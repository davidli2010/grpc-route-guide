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

//! Route guide server.

use futures::sync::oneshot;
use futures::{stream, Stream};
use futures::{Future, Sink};
use grpcio::*;
use route::route_guide::*;
use route::route_guide_grpc::{create_route_guide, RouteGuide};
use route::util;
use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
struct RouteGuideService {
    features: Arc<FeatureDatabase>,
}

impl RouteGuideService {
    fn new(features: FeatureDatabase) -> Self {
        Self {
            features: Arc::new(features),
        }
    }
}

impl RouteGuide for RouteGuideService {
    fn get_feature(&mut self, ctx: RpcContext, point: Point, sink: UnarySink<Feature>) {
        let feature =
            util::check_feature(self.features.get_feature(), &point).unwrap_or_else(|| {
                let mut f = Feature::default();
                f.set_location(point);
                f
            });

        let f = sink
            .success(feature)
            .map_err(move |err| eprintln!("Failed to get feature: {:?}", err));

        ctx.spawn(f);
    }

    fn list_features(
        &mut self,
        ctx: RpcContext,
        rect: Rectangle,
        sink: ServerStreamingSink<Feature>,
    ) {
        let features: Vec<_> = self
            .features
            .get_feature()
            .iter()
            .filter(|&f| util::exists(f) && util::in_range(f.get_location(), &rect))
            .map(|f| (f.clone(), WriteFlags::default()))
            .collect();

        let f = sink
            .send_all(stream::iter_ok::<_, Error>(features))
            .map(|_| {})
            .map_err(|e| eprintln!("Failed to list features: {:?}", e));

        ctx.spawn(f);
    }

    fn record_route(
        &mut self,
        ctx: RpcContext,
        stream: RequestStream<Point>,
        sink: ClientStreamingSink<RouteSummary>,
    ) {
        let features = self.features.clone();
        let timer = Instant::now();
        let f = stream
            .fold(
                (None, RouteSummary::default()),
                move |(prev, mut sum), point| {
                    let point_count = sum.get_point_count();
                    sum.set_point_count(point_count + 1);

                    let feature = util::check_feature(features.get_feature(), &point)
                        .unwrap_or_else(Feature::default);
                    if util::exists(&feature) {
                        let feature_count = sum.get_feature_count();
                        sum.set_feature_count(feature_count + 1);
                    }

                    if let Some(ref prev_point) = prev {
                        let distance = util::calc_distance(prev_point, &point);
                        let total_distance = sum.get_distance();
                        sum.set_distance(total_distance + distance);
                    }

                    Ok((Some(point), sum)) as Result<_>
                },
            )
            .and_then(move |(_, mut sum)| {
                let duration = timer.elapsed();
                sum.set_elapsed_time(duration.as_secs() as i32);
                sink.success(sum)
            })
            .map_err(|e| eprintln!("Failed to record route: {:?}", e));

        ctx.spawn(f);
    }

    fn route_chat(
        &mut self,
        ctx: RpcContext,
        stream: RequestStream<RouteNote>,
        sink: DuplexSink<RouteNote>,
    ) {
        let mut buffer: Vec<RouteNote> = Vec::new();

        let send = stream
            .map(move |note| {
                let send_notes: Vec<_> = buffer
                    .iter()
                    .filter_map(|n| {
                        if util::point_eq(n.get_location(), note.get_location()) {
                            Some((n.clone(), WriteFlags::default()))
                        } else {
                            None
                        }
                    })
                    .collect();
                buffer.push(note);
                stream::iter_ok::<_, Error>(send_notes)
            })
            .flatten();

        let f = sink
            .send_all(send)
            .map(|_| {})
            .map_err(|e| eprintln!("Failed to route chat: {:?}", e));

        ctx.spawn(f);
    }
}

struct RouteGuideServer {
    _port: u16,
    server: Server,
}

impl RouteGuideServer {
    /// Create a RouteGuide server listening on `port`.
    fn new(port: u16) -> Self {
        let env = Arc::new(Environment::new(1));

        let features = util::load_database();
        let route_guide = create_route_guide(RouteGuideService::new(features));

        let server = ServerBuilder::new(env)
            .register_service(route_guide)
            .bind("127.0.0.1", port)
            .build()
            .unwrap();

        Self {
            _port: port,
            server,
        }
    }

    /// Start serving requests.
    fn start(&mut self) {
        self.server.start();

        for &(ref host, port) in self.server.bind_addrs() {
            println!("listening on {}:{}", host, port);
        }
    }

    /// Await termination on the main thread since the grpc library uses daemon threads.
    fn block_until_shutdown(&mut self) {
        let (tx, rx) = oneshot::channel();
        std::thread::spawn(move || {
            println!("Press ENTER to exit...");
            let _ = std::io::stdin().read(&mut [0]).unwrap();
            let _ = tx.send(());
        });
        let _ = rx.wait();
        let _ = self.server.shutdown().wait();
    }
}

fn main() {
    let mut server = RouteGuideServer::new(8980);
    server.start();
    server.block_until_shutdown();
}
