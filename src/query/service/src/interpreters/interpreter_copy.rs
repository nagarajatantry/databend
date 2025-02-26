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
use std::time::Instant;

use common_catalog::plan::StageTableInfo;
use common_catalog::table::AppendMode;
use common_exception::Result;
use common_expression::infer_table_schema;
use common_expression::types::Int32Type;
use common_expression::types::StringType;
use common_expression::BlockThresholds;
use common_expression::DataBlock;
use common_expression::DataField;
use common_expression::DataSchemaRef;
use common_expression::DataSchemaRefExt;
use common_expression::FromData;
use common_expression::FromOptData;
use common_expression::SendableDataBlockStream;
use common_meta_app::principal::StageInfo;
use common_pipeline_core::Pipeline;
use common_sql::executor::table_read_plan::ToReadDataSourcePlan;
use common_sql::executor::CopyIntoTable;
use common_sql::executor::CopyIntoTableSource;
use common_sql::executor::Exchange;
use common_sql::executor::FragmentKind;
use common_sql::executor::PhysicalPlan;
use common_sql::plans::CopyIntoTablePlan;
use common_storage::StageFileInfo;
use common_storage::StageFilesInfo;
use common_storages_stage::StageTable;
use log::debug;
use log::info;

use crate::interpreters::common::check_deduplicate_label;
use crate::interpreters::common::hook_compact;
use crate::interpreters::common::CompactHookTraceCtx;
use crate::interpreters::common::CompactTargetTableDescription;
use crate::interpreters::Interpreter;
use crate::interpreters::SelectInterpreter;
use crate::pipelines::builders::build_append2table_with_commit_pipeline;
use crate::pipelines::builders::build_commit_data_pipeline;
use crate::pipelines::PipelineBuildResult;
use crate::schedulers::build_query_pipeline_without_render_result_set;
use crate::sessions::QueryContext;
use crate::sessions::TableContext;
use crate::sql::plans::CopyPlan;
use crate::sql::plans::Plan;
use crate::stream::DataBlockStream;

pub struct CopyInterpreter {
    ctx: Arc<QueryContext>,
    plan: CopyPlan,
}

impl CopyInterpreter {
    /// Create a CopyInterpreter with context and [`CopyPlan`].
    pub fn try_create(ctx: Arc<QueryContext>, plan: CopyPlan) -> Result<Self> {
        Ok(CopyInterpreter { ctx, plan })
    }

    #[async_backtrace::framed]
    async fn build_query(&self, query: &Plan) -> Result<(SelectInterpreter, DataSchemaRef)> {
        let (s_expr, metadata, bind_context, formatted_ast) = match query {
            Plan::Query {
                s_expr,
                metadata,
                bind_context,
                formatted_ast,
                ..
            } => (s_expr, metadata, bind_context, formatted_ast),
            v => unreachable!("Input plan must be Query, but it's {}", v),
        };

        let select_interpreter = SelectInterpreter::try_create(
            self.ctx.clone(),
            *(bind_context.clone()),
            *s_expr.clone(),
            metadata.clone(),
            formatted_ast.clone(),
            false,
        )?;

        // Building data schema from bind_context columns
        // TODO(leiyskey): Extract the following logic as new API of BindContext.
        let fields = bind_context
            .columns
            .iter()
            .map(|column_binding| {
                DataField::new(
                    &column_binding.column_name,
                    *column_binding.data_type.clone(),
                )
            })
            .collect();
        let data_schema = DataSchemaRefExt::create(fields);

        Ok((select_interpreter, data_schema))
    }

    /// Build a pipeline for local copy into stage.
    #[async_backtrace::framed]
    async fn build_local_copy_into_stage_pipeline(
        &self,
        stage: &StageInfo,
        path: &str,
        query: &Plan,
    ) -> Result<PipelineBuildResult> {
        let (select_interpreter, data_schema) = self.build_query(query).await?;
        let plan = select_interpreter.build_physical_plan().await?;
        let mut build_res = select_interpreter.build_pipeline(plan).await?;
        let table_schema = infer_table_schema(&data_schema)?;
        let stage_table_info = StageTableInfo {
            schema: table_schema,
            stage_info: stage.clone(),
            files_info: StageFilesInfo {
                path: path.to_string(),
                files: None,
                pattern: None,
            },
            files_to_copy: None,
            is_select: false,
        };
        let to_table = StageTable::try_create(stage_table_info)?;
        build_append2table_with_commit_pipeline(
            self.ctx.clone(),
            &mut build_res.main_pipeline,
            to_table,
            data_schema,
            None,
            false,
            AppendMode::Normal,
        )?;
        Ok(build_res)
    }

    fn set_status(&self, status: &str) {
        self.ctx.set_status_info(status);
        info!("{}", status);
    }

    #[async_backtrace::framed]
    pub async fn build_physical_plan(
        &self,
        plan: &CopyIntoTablePlan,
    ) -> Result<(PhysicalPlan, Vec<StageFileInfo>)> {
        let to_table = self
            .ctx
            .get_table(
                plan.catalog_info.catalog_name(),
                &plan.database_name,
                &plan.table_name,
            )
            .await?;
        let files = plan.collect_files(self.ctx.as_ref()).await?;
        let source = if let Some(ref query) = plan.query {
            let (select_interpreter, query_source_schema) = self.build_query(query).await?;
            let plan_query = select_interpreter.build_physical_plan().await?;
            let result_columns = select_interpreter.get_result_columns();
            CopyIntoTableSource::Query(Box::new(common_sql::executor::QuerySource {
                plan: plan_query,
                ignore_result: select_interpreter.get_ignore_result(),
                result_columns,
                query_source_schema,
            }))
        } else {
            let stage_table_info = StageTableInfo {
                files_to_copy: Some(files.clone()),
                ..plan.stage_table_info.clone()
            };
            let stage_table = StageTable::try_create(stage_table_info)?;
            let read_source_plan = Box::new(
                stage_table
                    .read_plan_with_catalog(
                        self.ctx.clone(),
                        plan.catalog_info.catalog_name().to_string(),
                        None,
                        None,
                        false,
                    )
                    .await?,
            );
            CopyIntoTableSource::Stage(read_source_plan)
        };

        let mut root = PhysicalPlan::CopyIntoTable(Box::new(CopyIntoTable {
            catalog_info: plan.catalog_info.clone(),
            required_values_schema: plan.required_values_schema.clone(),
            values_consts: plan.values_consts.clone(),
            required_source_schema: plan.required_source_schema.clone(),
            stage_table_info: plan.stage_table_info.clone(),
            table_info: to_table.get_table_info().clone(),
            force: plan.force,
            write_mode: plan.write_mode,
            validation_mode: plan.validation_mode.clone(),

            files: files.clone(),
            source,
        }));
        if plan.enable_distributed {
            root = PhysicalPlan::Exchange(Exchange {
                plan_id: 0,
                input: Box::new(root),
                kind: FragmentKind::Merge,
                keys: Vec::new(),
                ignore_exchange: false,
            });
        }
        Ok((root, files))
    }

    #[async_backtrace::framed]
    async fn build_read_stage_table_data_pipeline(
        &self,
        pipeline: &mut Pipeline,
        plan: &CopyIntoTablePlan,
        block_thresholds: BlockThresholds,
        files: Vec<StageFileInfo>,
    ) -> Result<()> {
        let ctx = self.ctx.clone();
        let table_ctx: Arc<dyn TableContext> = ctx.clone();

        let mut stage_table_info = plan.stage_table_info.clone();
        stage_table_info.files_to_copy = Some(files.clone());
        let stage_table = StageTable::try_create(stage_table_info.clone())?;
        let read_source_plan = {
            stage_table
                .read_plan_with_catalog(
                    ctx.clone(),
                    plan.catalog_info.catalog_name().to_string(),
                    None,
                    None,
                    false,
                )
                .await?
        };

        stage_table.set_block_thresholds(block_thresholds);
        stage_table.read_data(table_ctx, &read_source_plan, pipeline)?;

        Ok(())
    }

    fn get_copy_into_table_result(&self) -> Result<Vec<DataBlock>> {
        let cs = self.ctx.get_copy_status();

        let mut results = cs.files.iter().collect::<Vec<_>>();
        results.sort_by(|a, b| a.key().cmp(b.key()));

        let n = cs.files.len();
        let mut files = Vec::with_capacity(n);
        let mut rows_loaded = Vec::with_capacity(n);
        let mut errors_seen = Vec::with_capacity(n);
        let mut first_error = Vec::with_capacity(n);
        let mut first_error_line = Vec::with_capacity(n);

        for entry in results {
            let status = entry.value();
            files.push(entry.key().as_bytes().to_vec());
            rows_loaded.push(status.num_rows_loaded as i32);
            if let Some(err) = &status.error {
                errors_seen.push(err.num_errors as i32);
                first_error.push(Some(err.first_error.message.as_bytes().to_vec()));
                first_error_line.push(Some(err.first_error.line as i32 + 1));
            } else {
                errors_seen.push(0);
                first_error.push(None);
                first_error_line.push(None);
            }
        }
        let blocks = vec![DataBlock::new_from_columns(vec![
            StringType::from_data(files),
            Int32Type::from_data(rows_loaded),
            Int32Type::from_data(errors_seen),
            StringType::from_opt_data(first_error),
            Int32Type::from_opt_data(first_error_line),
        ])];
        Ok(blocks)
    }
}

#[async_trait::async_trait]
impl Interpreter for CopyInterpreter {
    fn name(&self) -> &str {
        "CopyInterpreterV2"
    }

    #[minitrace::trace(name = "copy_interpreter_execute_v2")]
    #[async_backtrace::framed]
    async fn execute2(&self) -> Result<PipelineBuildResult> {
        debug!("ctx.id" = self.ctx.get_id().as_str(); "copy_interpreter_execute_v2");

        let start = Instant::now();

        if check_deduplicate_label(self.ctx.clone()).await? {
            return Ok(PipelineBuildResult::create());
        }

        match &self.plan {
            CopyPlan::IntoTable(plan) => {
                let (physical_plan, files) = self.build_physical_plan(plan).await?;
                let mut build_res = build_query_pipeline_without_render_result_set(
                    &self.ctx,
                    &physical_plan,
                    false,
                )
                .await?;
                build_commit_data_pipeline(&self.ctx, &mut build_res.main_pipeline, plan, &files)
                    .await?;

                let compact_target = CompactTargetTableDescription {
                    catalog: plan.catalog_info.name_ident.catalog_name.clone(),
                    database: plan.database_name.clone(),
                    table: plan.table_name.clone(),
                };

                let trace_ctx = CompactHookTraceCtx {
                    start,
                    operation_name: "copy_into".to_owned(),
                };

                hook_compact(
                    self.ctx.clone(),
                    &mut build_res.main_pipeline,
                    compact_target,
                    trace_ctx,
                )
                .await;
                Ok(build_res)
            }
            CopyPlan::IntoStage {
                stage, from, path, ..
            } => {
                self.build_local_copy_into_stage_pipeline(stage, path, from)
                    .await
            }
            CopyPlan::NoFileToCopy => Ok(PipelineBuildResult::create()),
        }
    }

    fn inject_result(&self) -> Result<SendableDataBlockStream> {
        let blocks = match &self.plan {
            CopyPlan::NoFileToCopy => {
                vec![DataBlock::empty_with_schema(self.plan.schema())]
            }
            CopyPlan::IntoTable(p) if !p.from_attachment => self.get_copy_into_table_result()?,
            _ => {
                vec![]
            }
        };
        Ok(Box::pin(DataBlockStream::create(None, blocks)))
    }
}
