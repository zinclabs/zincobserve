// Copyright 2022 Zinc Labs Inc. and Contributors
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

use opentelemetry::global;
use std::time::Instant;
use tonic::{Request, Response, Status};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::handler::grpc::cluster_rpc::search_server::Search;
use crate::handler::grpc::cluster_rpc::SearchRequest;
use crate::handler::grpc::cluster_rpc::SearchResponse;
use crate::infra::metrics;
use crate::meta::StreamType;
use crate::service::search as SearchService;

#[derive(Default)]
pub struct Searcher {}

#[tonic::async_trait]
impl Search for Searcher {
    #[tracing::instrument(name = "grpc:search:enter", skip(self, req))]
    async fn search(
        &self,
        req: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let start = Instant::now();
        let parent_cx = global::get_text_map_propagator(|prop| {
            prop.extract(&super::MetadataMap(req.metadata()))
        });
        tracing::Span::current().set_parent(parent_cx);

        let req = req.get_ref();
        let org_id = req.org_id.clone();
        let stream_type: StreamType = StreamType::from(req.stream_type.as_str());
        let result = match SearchService::exec::search(req).await {
            Ok(res) => res,
            Err(err) => {
                let time = start.elapsed().as_secs_f64();
                metrics::GRPC_RESPONSE_TIME
                    .with_label_values(&[
                        "/_search",
                        "500",
                        &org_id,
                        "",
                        stream_type.to_string().as_str(),
                    ])
                    .observe(time);
                metrics::GRPC_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/_search",
                        "500",
                        &org_id,
                        "",
                        stream_type.to_string().as_str(),
                    ])
                    .inc();
                return Err(Status::internal(err.to_string()));
            }
        };

        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&[
                "/_search",
                "200",
                &org_id,
                "",
                stream_type.to_string().as_str(),
            ])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&[
                "/_search",
                "200",
                &org_id,
                "",
                stream_type.to_string().as_str(),
            ])
            .inc();

        Ok(Response::new(result))
    }
}
