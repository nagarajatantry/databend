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

use std::sync::Arc;

use common_catalog::plan::DataSourcePlan;
use common_catalog::table_context::TableContext;
use common_exception::Result;
use common_expression::TableSchemaRef;
use common_pipeline_core::Pipeline;

use super::ParquetRSTable;
use crate::parquet_rs::source::ParquetSource;
use crate::utils::calc_parallelism;
use crate::ParquetPart;
use crate::ParquetRSReader;

impl ParquetRSTable {
    #[inline]
    pub(super) fn do_read_data(
        &self,
        ctx: Arc<dyn TableContext>,
        plan: &DataSourcePlan,
        pipeline: &mut Pipeline,
    ) -> Result<()> {
        let table_schema: TableSchemaRef = self.table_info.schema();
        // If there is a `ParquetFilesPart`, we should create pruner for it.
        // `ParquetFilesPart`s are always staying at the end of `parts`.
        let need_pruner = matches!(
            plan.parts
                .partitions
                .last()
                .map(|p| p.as_any().downcast_ref::<ParquetPart>().unwrap()),
            Some(ParquetPart::ParquetFiles(_)),
        );

        let reader = Arc::new(ParquetRSReader::create_with_parquet_schema(
            ctx.clone(),
            self.operator.clone(),
            table_schema,
            self.leaf_fields.clone(),
            &self.schema_descr,
            plan,
            self.read_options,
            need_pruner,
        )?);

        let num_threads = calc_parallelism(&ctx, plan)?;

        // TODO(parquet):
        // - introduce Top-K optimization.
        // - pass `self.parquet_metas` to `ParquetSource` to avoid duplicate meta processing for small files.
        pipeline.add_source(
            |output| ParquetSource::create(ctx.clone(), output, reader.clone()),
            num_threads,
        )
    }
}
