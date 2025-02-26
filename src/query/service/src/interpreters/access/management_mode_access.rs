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

use common_catalog::table_context::TableContext;
use common_config::GlobalConfig;
use common_exception::ErrorCode;
use common_exception::Result;
use common_storages_view::view_table::VIEW_ENGINE;

use crate::interpreters::access::AccessChecker;
use crate::sessions::QueryContext;
use crate::sql::plans::Plan;

pub struct ManagementModeAccess {}

impl ManagementModeAccess {
    pub fn create() -> Box<dyn AccessChecker> {
        Box::new(ManagementModeAccess {})
    }
}

#[async_trait::async_trait]
impl AccessChecker for ManagementModeAccess {
    // Check what we can do if in management mode.
    #[async_backtrace::framed]
    async fn check(&self, ctx: &Arc<QueryContext>, plan: &Plan) -> Result<()> {
        // Allows for management-mode.
        if GlobalConfig::instance().query.management_mode {
            let ok = match plan {
                Plan::Query {rewrite_kind, .. } => {
                    use common_sql::plans::RewriteKind;
                        match rewrite_kind  {
                            Some(ref v) => matches!(v,
                            RewriteKind::ShowDatabases
                            | RewriteKind::ShowTables(_)
                            | RewriteKind::ShowColumns(_, _)
                            | RewriteKind::ShowEngines
                            | RewriteKind::ShowSettings
                            | RewriteKind::ShowFunctions
                            | RewriteKind::ShowTableFunctions
                            | RewriteKind::ShowUsers
                            | RewriteKind::ShowStages
                            | RewriteKind::DescribeStage
                            | RewriteKind::ListStage
                            | RewriteKind::Call
                            | RewriteKind::ShowRoles),
                            _ => false
                        }
                },
                // Show.
                Plan::ShowCreateDatabase(_)
                | Plan::ShowCreateTable(_)
                | Plan::ShowGrants(_)

                // Set
                | Plan::SetVariable(_)

                // Database.
                | Plan::CreateDatabase(_)
                | Plan::DropDatabase(_)

                // Table.
                | Plan::CreateTable(_)
                | Plan::DropTable(_)
                | Plan::DropView(_)
                | Plan::CreateView(_)

                // User.
                | Plan::AlterUser(_)
                | Plan::CreateUser(_)
                | Plan::DropUser(_)

                // Roles.
                | Plan::ShowRoles(_)
                | Plan::CreateRole(_)
                | Plan::DropRole(_)

                // Privilege.
                | Plan::GrantPriv(_)
                | Plan::RevokePriv(_)
                | Plan::GrantRole(_)
                | Plan::RevokeRole(_)
                // Stage.
                | Plan::CreateStage(_)
                | Plan::DropStage(_)
                // Network policy.
                | Plan::CreateNetworkPolicy(_)
                | Plan::AlterNetworkPolicy(_)
                | Plan::DropNetworkPolicy(_)

                // UDF
                | Plan::CreateUDF(_)
                | Plan::AlterUDF(_)
                | Plan::DropUDF(_)
                | Plan::UseDatabase(_) => true,
                Plan::DescribeTable(plan) => {
                    let catalog = &plan.catalog;
                    let database = &plan.database;
                    let table = &plan.table;
                    let table = ctx.get_table(catalog, database, table).await?;
                    table.get_table_info().engine() != VIEW_ENGINE
                },
                _ => false,
            };

            if !ok {
                return Err(ErrorCode::ManagementModePermissionDenied(format!(
                    "Access denied for operation:{:?} in management-mode",
                    plan.format_indent()
                )));
            }
        };

        Ok(())
    }
}
