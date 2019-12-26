// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::common::MILLISECOND;
use crate::stats::MetricsSnapshot;

use logger::*;
use metrics::*;
use tiny_http::{Method, Response, Server};

use std::net::SocketAddr;

pub struct Http {
    snapshot: MetricsSnapshot,
    server: Server,
}

impl Http {
    pub fn new(
        address: SocketAddr,
        metrics: Metrics<AtomicU32>,
        count_label: Option<&str>,
    ) -> Self {
        let server = tiny_http::Server::http(address);
        if server.is_err() {
            fatal!("Failed to open {} for HTTP Stats listener", address);
        }
        Self {
            snapshot: MetricsSnapshot::new(metrics, count_label),
            server: server.unwrap(),
        }
    }

    pub fn run(&mut self) {
        let now = time::precise_time_ns();
        if now - self.snapshot.refreshed > 500 * MILLISECOND {
            self.snapshot.refresh();
        }
        if let Ok(Some(request)) = self.server.try_recv() {
            let url = request.url();
            let parts: Vec<&str> = url.split('?').collect();
            let url = parts[0];
            match request.method() {
                Method::Get => match url {
                    "/" => {
                        debug!("Serving GET on index");
                        let _ = request.respond(Response::from_string(format!(
                            "Welcome to {}\nVersion: {}\n",
                            crate::config::NAME,
                            crate::config::VERSION,
                        )));
                    }
                    "/metrics" => {
                        debug!("Serving Prometheus compatible stats");
                        let _ = request.respond(Response::from_string(self.snapshot.prometheus()));
                    }
                    "/metrics.json" | "/vars.json" | "/admin/metrics.json" => {
                        debug!("Serving machine readable stats");
                        let _ = request.respond(Response::from_string(self.snapshot.json(false)));
                    }
                    "/vars" => {
                        debug!("Serving human readable stats");
                        let _ = request.respond(Response::from_string(self.snapshot.human()));
                    }
                    url => {
                        debug!("GET on non-existent url: {}", url);
                        debug!("Serving machine readable stats");
                        let _ = request.respond(Response::from_string(self.snapshot.json(false)));
                    }
                },
                method => {
                    debug!("unsupported request method: {}", method);
                    let _ = request.respond(Response::empty(404));
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
