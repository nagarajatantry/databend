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

use common_exception::Result;
use common_expression::FunctionContext;

use super::rewrite::RuleCommuteJoin;
use super::rewrite::RuleEliminateEvalScalar;
use super::rewrite::RuleFoldCountAggregate;
use super::rewrite::RuleInferFilter;
use super::rewrite::RuleNormalizeDisjunctiveFilter;
use super::rewrite::RuleNormalizeScalarFilter;
use super::rewrite::RulePushDownFilterAggregate;
use super::rewrite::RulePushDownFilterEvalScalar;
use super::rewrite::RulePushDownFilterJoin;
use super::rewrite::RulePushDownLimitAggregate;
use super::rewrite::RulePushDownLimitExpression;
use super::rewrite::RulePushDownPrewhere;
use super::rewrite::RuleTryApplyAggIndex;
use crate::optimizer::rule::rewrite::RuleEliminateFilter;
use crate::optimizer::rule::rewrite::RuleMergeEvalScalar;
use crate::optimizer::rule::rewrite::RuleMergeFilter;
use crate::optimizer::rule::rewrite::RulePushDownFilterProjectSet;
use crate::optimizer::rule::rewrite::RulePushDownFilterScan;
use crate::optimizer::rule::rewrite::RulePushDownFilterSort;
use crate::optimizer::rule::rewrite::RulePushDownFilterUnion;
use crate::optimizer::rule::rewrite::RulePushDownLimitOuterJoin;
use crate::optimizer::rule::rewrite::RulePushDownLimitScan;
use crate::optimizer::rule::rewrite::RulePushDownLimitSort;
use crate::optimizer::rule::rewrite::RulePushDownLimitUnion;
use crate::optimizer::rule::rewrite::RulePushDownSortScan;
use crate::optimizer::rule::rewrite::RuleSplitAggregate;
use crate::optimizer::rule::transform::RuleCommuteJoinBaseTable;
use crate::optimizer::rule::transform::RuleEagerAggregation;
use crate::optimizer::rule::transform::RuleLeftExchangeJoin;
use crate::optimizer::rule::RuleID;
use crate::optimizer::rule::RulePtr;
use crate::MetadataRef;

pub struct RuleFactory;

impl RuleFactory {
    pub fn create_rule(
        id: RuleID,
        metadata: MetadataRef,
        _func_ctx: FunctionContext,
    ) -> Result<RulePtr> {
        match id {
            RuleID::EliminateEvalScalar => Ok(Box::new(RuleEliminateEvalScalar::new())),
            RuleID::PushDownFilterUnion => Ok(Box::new(RulePushDownFilterUnion::new())),
            RuleID::PushDownFilterEvalScalar => {
                Ok(Box::new(RulePushDownFilterEvalScalar::new(metadata)))
            }
            RuleID::PushDownFilterJoin => Ok(Box::new(RulePushDownFilterJoin::new(metadata))),
            RuleID::PushDownFilterScan => Ok(Box::new(RulePushDownFilterScan::new(metadata))),
            RuleID::PushDownFilterSort => Ok(Box::new(RulePushDownFilterSort::new())),
            RuleID::PushDownFilterProjectSet => Ok(Box::new(RulePushDownFilterProjectSet::new())),
            RuleID::PushDownLimitUnion => Ok(Box::new(RulePushDownLimitUnion::new())),
            RuleID::PushDownLimitScan => Ok(Box::new(RulePushDownLimitScan::new())),
            RuleID::PushDownSortScan => Ok(Box::new(RulePushDownSortScan::new())),
            RuleID::PushDownLimitOuterJoin => Ok(Box::new(RulePushDownLimitOuterJoin::new())),
            RuleID::PushDownLimitExpression => Ok(Box::new(RulePushDownLimitExpression::new())),
            RuleID::PushDownLimitSort => Ok(Box::new(RulePushDownLimitSort::new())),
            RuleID::PushDownLimitAggregate => Ok(Box::new(RulePushDownLimitAggregate::new())),
            RuleID::PushDownFilterAggregate => Ok(Box::new(RulePushDownFilterAggregate::new())),
            RuleID::EliminateFilter => Ok(Box::new(RuleEliminateFilter::new())),
            RuleID::MergeEvalScalar => Ok(Box::new(RuleMergeEvalScalar::new())),
            RuleID::MergeFilter => Ok(Box::new(RuleMergeFilter::new())),
            RuleID::NormalizeScalarFilter => Ok(Box::new(RuleNormalizeScalarFilter::new())),
            RuleID::SplitAggregate => Ok(Box::new(RuleSplitAggregate::new())),
            RuleID::FoldCountAggregate => Ok(Box::new(RuleFoldCountAggregate::new())),
            RuleID::NormalizeDisjunctiveFilter => {
                Ok(Box::new(RuleNormalizeDisjunctiveFilter::new()))
            }
            RuleID::InferFilter => Ok(Box::new(RuleInferFilter::new())),
            RuleID::CommuteJoin => Ok(Box::new(RuleCommuteJoin::new())),
            RuleID::CommuteJoinBaseTable => Ok(Box::new(RuleCommuteJoinBaseTable::new())),
            RuleID::LeftExchangeJoin => Ok(Box::new(RuleLeftExchangeJoin::new())),
            RuleID::EagerAggregation => Ok(Box::new(RuleEagerAggregation::new(metadata))),
            RuleID::PushDownPrewhere => Ok(Box::new(RulePushDownPrewhere::new(metadata))),
            RuleID::TryApplyAggIndex => Ok(Box::new(RuleTryApplyAggIndex::new(metadata))),
        }
    }
}
