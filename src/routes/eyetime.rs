/// 用眼时长路由处理器
/// 对应原始项目的 EyeTime_Routes.js
use crate::services::eyetime;
use crate::utils;
use std::collections::HashMap;
use worker::*;

fn get_offset_hours(ctx: &RouteContext<()>) -> i32 {
    utils::get_env_or(&ctx.env, "DEFAULT_TIMEZONE_OFFSET", "8")
        .parse()
        .unwrap_or(8)
}

fn json_error(message: &str, status: u16) -> Result<Response> {
    let body = serde_json::json!({
        "error": message,
        "success": false
    });
    Ok(Response::from_json(&body)?.with_status(status))
}

/// GET /api/eyetime/daily - 某日用眼统计
pub async fn daily(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("DB")?;
    let offset_hours = get_offset_hours(&ctx);
    let now_ms = Date::now().as_millis();

    let url = req.url()?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let date_str = match params.get("date") {
        Some(d) => {
            if utils::parse_date(d).is_none() {
                return json_error("Invalid date format. Please use YYYY-MM-DD format.", 400);
            }
            d.clone()
        }
        None => {
            let tz = utils::TimezoneHelper::new(offset_hours);
            let today = tz.get_today(now_ms);
            utils::format_date(&today)
        }
    };

    let stats = eyetime::get_daily_minutes(&d1, &date_str, offset_hours).await?;
    Response::from_json(&stats)
}

/// GET /api/eyetime/weekly - 周用眼统计
pub async fn weekly(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("DB")?;
    let offset_hours = get_offset_hours(&ctx);
    let now_ms = Date::now().as_millis();

    let url = req.url()?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let week_offset = params
        .get("weekOffset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let stats = eyetime::get_weekly_minutes(&d1, week_offset, offset_hours, now_ms).await?;
    Response::from_json(&stats)
}

/// GET /api/eyetime/monthly - 月用眼统计
pub async fn monthly(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("DB")?;
    let offset_hours = get_offset_hours(&ctx);
    let now_ms = Date::now().as_millis();

    let url = req.url()?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let month_offset = params
        .get("monthOffset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let stats = eyetime::get_monthly_minutes(&d1, month_offset, offset_hours, now_ms).await?;
    Response::from_json(&stats)
}
