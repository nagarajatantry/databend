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

#![allow(clippy::uninlined_format_args)]

pub mod counter;
mod dump;
pub mod histogram;
mod recorder;
pub mod registry;
mod reset;

pub use dump::dump_metric_samples;
pub use dump::HistogramCount;
pub use dump::MetricSample;
pub use dump::MetricValue;
pub use dump::SummaryCount;
pub use metrics_exporter_prometheus::PrometheusHandle;
pub use prometheus_client::metrics::counter::Counter;
pub use prometheus_client::metrics::family::Family;
pub use prometheus_client::metrics::gauge::Gauge;
pub use prometheus_client::metrics::histogram::Histogram;
pub use recorder::init_default_metrics_recorder;
pub use recorder::label_counter;
pub use recorder::label_counter_with_val;
pub use recorder::label_counter_with_val_and_labels;
pub use recorder::label_decrement_gauge_with_val_and_labels;
pub use recorder::label_gauge;
pub use recorder::label_gauge_with_val_and_labels;
pub use recorder::label_histogram_with_val;
pub use recorder::label_increment_gauge_with_val_and_labels;
pub use recorder::try_handle;
pub use recorder::LABEL_KEY_CLUSTER;
pub use recorder::LABEL_KEY_TENANT;
pub use registry::load_global_prometheus_registry;
pub use registry::register_counter;
pub use registry::register_counter_family;
pub use registry::register_gauge;
pub use registry::register_gauge_family;
pub use registry::register_histogram_family_in_milliseconds;
pub use registry::register_histogram_family_in_seconds;
pub use registry::register_histogram_in_milliseconds;
pub use registry::register_histogram_in_seconds;
pub use reset::reset_metrics;
