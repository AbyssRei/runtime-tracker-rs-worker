-- D1 数据库初始化迁移
-- 应用统计表：按设备/日期/应用/小时粒度存储使用时长
CREATE TABLE IF NOT EXISTS daily_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    date TEXT NOT NULL,              -- 本地时区日期 YYYY-MM-DD
    app_name TEXT NOT NULL,
    package_name TEXT NOT NULL DEFAULT '',
    hourly_usage TEXT NOT NULL DEFAULT '[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]',
    UNIQUE(device_id, date, app_name, package_name)
);

CREATE INDEX IF NOT EXISTS idx_daily_stats_device_date
    ON daily_stats(device_id, date);

CREATE INDEX IF NOT EXISTS idx_daily_stats_date
    ON daily_stats(date);

-- 用眼时长表：按日期/小时粒度存储用眼时长
CREATE TABLE IF NOT EXISTS daily_eye_time (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,       -- 本地时区日期 YYYY-MM-DD
    hourly_usage TEXT NOT NULL DEFAULT '[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]'
);

CREATE INDEX IF NOT EXISTS idx_daily_eye_time_date
    ON daily_eye_time(date);
