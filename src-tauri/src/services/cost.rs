//! CostService：按每条事件的时间和上下文长度匹配价格规则，再按模型与周期汇总。

use std::collections::{BTreeMap, HashMap};

use crate::database::{dao, Database};
use crate::models::{ModelCost, ModelPricing, PricingRule};

/// 计算指定数据源所有模型的费用。规则只在 advanced_enabled=true 时启用，
/// 多条规则自上而下取第一条匹配；没有匹配则使用模型默认单价。
pub fn calculate(
    db: &Database,
    source: dao::SpeedSource,
    pricing: &HashMap<String, ModelPricing>,
) -> Result<Vec<ModelCost>, String> {
    db.with_conn(|conn| {
        let (today_start, week_start, month_start) = dao::local_period_bounds(conn)?;
        let mut costs: BTreeMap<String, ModelCost> = BTreeMap::new();
        for event in dao::cost_events(conn, source)? {
            let entry = costs.entry(event.model.clone()).or_insert_with(|| ModelCost {
                model: event.model.clone(),
                ..ModelCost::default()
            });
            let Some(model_pricing) = pricing.get(&event.model) else {
                continue;
            };
            let prices = active_prices(model_pricing, &event);
            let cost = (event.input as f64 * prices.0
                + event.output as f64 * prices.1
                + event.cache_read as f64 * prices.2
                + event.cache_creation as f64 * prices.3)
                / 1_000_000.0;
            entry.total += cost;
            if event.ts >= today_start {
                entry.today += cost;
            }
            if event.ts >= week_start {
                entry.this_week += cost;
            }
            if event.ts >= month_start {
                entry.this_month += cost;
            }
        }
        Ok(costs.into_values().collect())
    })
}

/// 返回（输入、输出、缓存读、缓存写）单价，单位为元/百万 tokens。
fn active_prices(model: &ModelPricing, event: &dao::CostEventRow) -> (f64, f64, f64, f64) {
    if model.advanced_enabled.unwrap_or(false) {
        if let Some(rule) = model
            .rules
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|rule| rule_matches(rule, event))
        {
            return (
                rule.input,
                rule.output,
                rule.cache,
                rule.cache_write.unwrap_or(0.0),
            );
        }
    }
    (
        model.input,
        model.output,
        model.cache,
        model.cache_write.unwrap_or(0.0),
    )
}

fn rule_matches(rule: &PricingRule, event: &dao::CostEventRow) -> bool {
    let context = event.input + event.cache_read + event.cache_creation;
    if rule.min_context.is_some_and(|min| context < min)
        || rule.max_context.is_some_and(|max| context >= max)
    {
        return false;
    }
    time_matches(rule, &event.local_time)
}

/// 开始时间含、结束时间不含；开始晚于结束表示跨零点时段。
fn time_matches(rule: &PricingRule, local_time: &str) -> bool {
    let start = rule.start_time.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let end = rule.end_time.as_deref().map(str::trim).filter(|s| !s.is_empty());
    match (start, end) {
        (None, None) => true,
        (Some(start), None) => local_time >= start,
        (None, Some(end)) => local_time < end,
        (Some(start), Some(end)) if start > end => local_time >= start || local_time < end,
        (Some(start), Some(end)) => local_time >= start && local_time < end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pricing(enabled: bool, rules: Vec<PricingRule>) -> ModelPricing {
        ModelPricing {
            input: 1.0,
            output: 2.0,
            cache: 0.1,
            cache_write: Some(0.2),
            advanced_enabled: Some(enabled),
            rules: Some(rules),
        }
    }

    fn rule(min: Option<i64>, max: Option<i64>, start: &str, end: &str, output: f64) -> PricingRule {
        PricingRule {
            start_time: Some(start.to_string()),
            end_time: Some(end.to_string()),
            min_context: min,
            max_context: max,
            input: 1.0,
            output,
            cache: 0.1,
            cache_write: Some(0.2),
        }
    }

    fn event(context: i64, time: &str) -> dao::CostEventRow {
        dao::CostEventRow {
            ts: 0,
            model: "m".to_string(),
            input: context,
            output: 10,
            cache_read: 0,
            cache_creation: 0,
            local_time: time.to_string(),
        }
    }

    #[test]
    fn disabled_rules_use_default_price() {
        let p = pricing(false, vec![rule(None, None, "", "", 99.0)]);
        assert_eq!(active_prices(&p, &event(1_000, "12:00")).1, 2.0);
    }

    #[test]
    fn context_range_is_min_inclusive_and_max_exclusive() {
        let p = pricing(true, vec![rule(Some(128_000), Some(256_000), "", "", 8.0)]);
        assert_eq!(active_prices(&p, &event(127_999, "12:00")).1, 2.0);
        assert_eq!(active_prices(&p, &event(128_000, "12:00")).1, 8.0);
        assert_eq!(active_prices(&p, &event(255_999, "12:00")).1, 8.0);
        assert_eq!(active_prices(&p, &event(256_000, "12:00")).1, 2.0);
    }

    #[test]
    fn daily_time_range_including_overnight() {
        let p = pricing(true, vec![rule(None, None, "22:00", "06:00", 5.0)]);
        assert_eq!(active_prices(&p, &event(0, "23:59")).1, 5.0);
        assert_eq!(active_prices(&p, &event(0, "05:59")).1, 5.0);
        assert_eq!(active_prices(&p, &event(0, "06:00")).1, 2.0);
        assert_eq!(active_prices(&p, &event(0, "12:00")).1, 2.0);
    }

    #[test]
    fn first_matching_rule_wins() {
        let p = pricing(
            true,
            vec![
                rule(Some(100_000), None, "", "", 6.0),
                rule(Some(100_000), None, "00:00", "08:00", 3.0),
            ],
        );
        assert_eq!(active_prices(&p, &event(120_000, "02:00")).1, 6.0);
    }
}
