// Copyright 2021 Datafuse Labs
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

use metrics::counter;

pub fn metrics_incr_http_request_count(method: String, api: String, status: String) {
    let labels = [("method", method), ("api", api), ("status", status)];
    counter!("query_http_requests_count", 1, &labels);
}

pub fn metrics_incr_http_slow_request_count(method: String, api: String, status: String) {
    let labels = [("method", method), ("api", api), ("status", status)];
    counter!("query_http_slow_requests_count", 1, &labels);
}

pub fn metrics_incr_http_response_errors_count(err: String, code: u16) {
    let labels = [("err", err), ("code", code.to_string())];
    counter!("query_http_response_errors_count", 1, &labels);
}

pub fn metrics_incr_http_response_panics_count() {
    counter!("query_http_response_panics_count", 1);
}
