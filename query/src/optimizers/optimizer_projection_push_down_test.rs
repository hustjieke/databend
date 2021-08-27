// Copyright 2020 Datafuse Labs.
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

use std::mem::size_of;
use std::sync::Arc;

use common_datavalues::prelude::*;
use common_exception::Result;
use common_planners::*;
use pretty_assertions::assert_eq;

use crate::optimizers::optimizer_test::*;
use crate::optimizers::*;
use crate::sql::*;

#[test]
fn test_projection_push_down_optimizer_1() -> Result<()> {
    let ctx = crate::tests::try_create_context()?;

    let schema = DataSchemaRefExt::create(vec![
        DataField::new("a", DataType::Utf8, false),
        DataField::new("b", DataType::Utf8, false),
        DataField::new("c", DataType::Utf8, false),
        DataField::new("d", DataType::Utf8, false),
    ]);

    let output_schema = DataSchemaRefExt::create(vec![
        DataField::new("a", DataType::Utf8, false),
        DataField::new("b", DataType::Utf8, false),
        DataField::new("c", DataType::Utf8, false),
    ]);

    let plan = PlanNode::Projection(ProjectionPlan {
        expr: vec![col("a"), col("b"), col("c")],
        schema: output_schema,
        input: Arc::from(
            PlanBuilder::from(&PlanNode::Empty(EmptyPlan::create_with_schema(schema))).build()?,
        ),
    });

    let mut projection_push_down = ProjectionPushDownOptimizer::create(ctx);
    let optimized = projection_push_down.optimize(&plan)?;

    let expect = "\
        Projection: a:Utf8, b:Utf8, c:Utf8";

    let actual = format!("{:?}", optimized);
    assert_eq!(expect, actual);

    Ok(())
}

#[test]
fn test_projection_push_down_optimizer_group_by() -> Result<()> {
    let ctx = crate::tests::try_create_context()?;

    let plan = PlanParser::create(ctx.clone())
        .build_from_sql("select max(value) as c1, name as c2 from system.settings group by c2")?;

    let mut project_push_down = ProjectionPushDownOptimizer::create(ctx);
    let optimized = project_push_down.optimize(&plan)?;

    let expect = "\
        Projection: max(value) as c1:Utf8, name as c2:Utf8\
        \n  AggregatorFinal: groupBy=[[name]], aggr=[[max(value)]]\
        \n    AggregatorPartial: groupBy=[[name]], aggr=[[max(value)]]\
        \n      ReadDataSource: scan partitions: [1], scan schema: [name:Utf8, value:Utf8], statistics: [read_rows: 0, read_bytes: 0]";

    let actual = format!("{:?}", optimized);
    assert_eq!(expect, actual);
    Ok(())
}

#[test]
fn test_projection_push_down_optimizer_2() -> Result<()> {
    let ctx = crate::tests::try_create_context()?;

    let total = ctx.get_settings().get_max_block_size()? as u64;
    let statistics =
        Statistics::new_exact(total as usize, ((total) * size_of::<u64>() as u64) as usize);
    ctx.try_set_statistics(&statistics)?;
    let source_plan = PlanNode::ReadSource(ReadDataSourcePlan {
        db: "system".to_string(),
        table: "test".to_string(),
        table_id: 0,
        table_version: None,
        schema: DataSchemaRefExt::create(vec![
            DataField::new("a", DataType::Utf8, false),
            DataField::new("b", DataType::Utf8, false),
            DataField::new("c", DataType::Utf8, false),
        ]),
        parts: generate_partitions(8, total as u64),
        statistics: statistics.clone(),
        description: format!(
            "(Read from system.{} table, Read Rows:{}, Read Bytes:{})",
            "test".to_string(),
            statistics.read_rows,
            statistics.read_bytes
        ),
        scan_plan: Arc::new(ScanPlan::empty()),
        remote: false,
    });

    let filter_plan = PlanBuilder::from(&source_plan)
        .filter(col("a").gt(lit(6)).and(col("b").lt_eq(lit(10))))?
        .build()?;

    let plan = PlanNode::Projection(ProjectionPlan {
        expr: vec![col("a")],
        schema: DataSchemaRefExt::create(vec![DataField::new("a", DataType::Utf8, false)]),
        input: Arc::from(filter_plan),
    });

    let mut projection_push_down = ProjectionPushDownOptimizer::create(ctx);
    let optimized = projection_push_down.optimize(&plan)?;

    let expect = "\
        Projection: a:Utf8\
        \n  Filter: ((a > 6) and (b <= 10))\
        \n    ReadDataSource: scan partitions: [8], scan schema: [a:Utf8, b:Utf8], statistics: [read_rows: 10000, read_bytes: 80000]";
    let actual = format!("{:?}", optimized);
    assert_eq!(expect, actual);

    Ok(())
}

#[test]
fn test_projection_push_down_optimizer_3() -> Result<()> {
    let ctx = crate::tests::try_create_context()?;

    let total = ctx.get_settings().get_max_block_size()? as u64;
    let statistics =
        Statistics::new_exact(total as usize, ((total) * size_of::<u64>() as u64) as usize);
    ctx.try_set_statistics(&statistics)?;
    let source_plan = PlanNode::ReadSource(ReadDataSourcePlan {
        db: "system".to_string(),
        table: "test".to_string(),
        table_id: 0,
        table_version: None,
        schema: DataSchemaRefExt::create(vec![
            DataField::new("a", DataType::Utf8, false),
            DataField::new("b", DataType::Utf8, false),
            DataField::new("c", DataType::Utf8, false),
            DataField::new("d", DataType::Utf8, false),
            DataField::new("e", DataType::Utf8, false),
            DataField::new("f", DataType::Utf8, false),
            DataField::new("g", DataType::Utf8, false),
        ]),
        parts: generate_partitions(8, total as u64),
        statistics: statistics.clone(),
        description: format!(
            "(Read from system.{} table, Read Rows:{}, Read Bytes:{})",
            "test".to_string(),
            statistics.read_rows,
            statistics.read_bytes
        ),
        scan_plan: Arc::new(ScanPlan::empty()),
        remote: false,
    });

    let group_exprs = &[col("a"), col("c")];

    // SELECT a FROM table WHERE b = 10 GROUP BY a, c having a < 10 order by c LIMIT 10;
    let plan = PlanBuilder::from(&source_plan)
        .filter(col("b").eq(lit(10)))?
        .aggregate_partial(&[], group_exprs)?
        .aggregate_final(source_plan.schema(), &[], group_exprs)?
        .having(col("a").lt(lit(10)))?
        .sort(&[col("c")])?
        .limit(10)?
        .project(&[col("a")])?
        .build()?;

    let mut projection_push_down = ProjectionPushDownOptimizer::create(ctx);
    let optimized = projection_push_down.optimize(&plan)?;

    let expect = "\
    Projection: a:Utf8\
    \n  Limit: 10\
    \n    Sort: c:Utf8\
    \n      Having: (a < 10)\
    \n        AggregatorFinal: groupBy=[[a, c]], aggr=[[]]\
    \n          AggregatorPartial: groupBy=[[a, c]], aggr=[[]]\
    \n            Filter: (b = 10)\
    \n              ReadDataSource: scan partitions: [8], scan schema: [a:Utf8, b:Utf8, c:Utf8], statistics: [read_rows: 10000, read_bytes: 80000]";

    let actual = format!("{:?}", optimized);
    assert_eq!(expect, actual);

    Ok(())
}

#[test]
fn test_projection_push_down_optimizer_4() -> Result<()> {
    let ctx = crate::tests::try_create_context()?;

    let plan = PlanParser::create(ctx.clone())
        .build_from_sql("select substring(value from 1 for 3)  as c1 from system.settings")?;

    let mut project_push_down = ProjectionPushDownOptimizer::create(ctx);
    let optimized = project_push_down.optimize(&plan)?;

    let expect = "Projection: substring(value, 1, 3) as c1:Utf8\
                        \n  Expression: substring(value, 1, 3):Utf8 (Before Projection)\
                        \n    ReadDataSource: scan partitions: [1], scan schema: [value:Utf8], statistics: [read_rows: 0, read_bytes: 0]";

    let actual = format!("{:?}", optimized);
    assert_eq!(expect, actual);
    Ok(())
}