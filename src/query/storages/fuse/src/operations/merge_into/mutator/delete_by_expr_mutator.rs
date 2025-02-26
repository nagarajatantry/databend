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

use std::ops::Not;

use common_exception::Result;
use common_expression::types::BooleanType;
use common_expression::types::DataType;
use common_expression::DataBlock;
use common_expression::Evaluator;
use common_expression::Expr;
use common_expression::FunctionContext;
use common_functions::BUILTIN_FUNCTIONS;
use common_sql::executor::cast_expr_to_non_null_boolean;

pub struct DeleteByExprMutator {
    expr: Option<Expr>,
    row_id_idx: usize,
    func_ctx: FunctionContext,
}

impl DeleteByExprMutator {
    pub fn create(expr: Option<Expr>, func_ctx: FunctionContext, row_id_idx: usize) -> Self {
        Self {
            expr,
            row_id_idx,
            func_ctx,
        }
    }

    // return block after delete, and the rowIds which are deleted
    pub fn delete_by_expr(&self, data_block: DataBlock) -> Result<(DataBlock, DataBlock)> {
        if self.expr.is_none() {
            Ok((
                DataBlock::empty(),
                DataBlock::new(
                    vec![data_block.get_by_offset(self.row_id_idx).clone()],
                    data_block.num_rows(),
                ),
            ))
        } else {
            let filter: Expr = cast_expr_to_non_null_boolean(self.expr.as_ref().unwrap().clone())?;
            assert_eq!(filter.data_type(), &DataType::Boolean);

            let evaluator = Evaluator::new(&data_block, &self.func_ctx, &BUILTIN_FUNCTIONS);

            let predicates = evaluator
                .run(&filter)
                .map_err(|e| e.add_message("eval filter failed:"))?
                .try_downcast::<BooleanType>()
                .unwrap();
            let filter = predicates.into_column().unwrap();
            let filtered_block = data_block.clone().filter_with_bitmap(&filter)?;

            Ok((
                data_block.filter_with_bitmap(&filter.not())?,
                DataBlock::new(
                    vec![filtered_block.get_by_offset(self.row_id_idx).clone()],
                    filtered_block.num_rows(),
                ),
            ))
        }
    }
}
