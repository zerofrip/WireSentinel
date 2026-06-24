use super::traits::{
    BandwidthRepository, DnsLogQuery, DnsLogRepository, DnsSortField, Result, SortOrder,
    TrafficLogQuery, TrafficLogRepository, TrafficSortField,
};
use async_trait::async_trait;
use shared_types::{
    AppIdentity, AppRecord, BandwidthStats, DNSQueryLog, Direction, Protocol, TopDomainEntry,
    TrafficEvent, TrafficRoute, Verdict, WireSentinelError,
};
use sqlx::SqlitePool;
use std::path::PathBuf;
use uuid::Uuid;

pub struct SqliteTrafficLogRepository {
    pool: SqlitePool,
}

impl SqliteTrafficLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TrafficLogRepository for SqliteTrafficLogRepository {
    async fn insert(&self, event: &TrafficEvent) -> Result<()> {
        let route_json = serde_json::to_string(&event.route).map_err(WireSentinelError::Serde)?;
        let verdict_json =
            serde_json::to_string(&event.verdict).map_err(WireSentinelError::Serde)?;
        let src_ip = event
            .source_ip
            .clone()
            .or_else(|| Some(event.local_addr.ip().to_string()));
        let dst_ip = event
            .destination_ip
            .clone()
            .or_else(|| Some(event.remote_addr.ip().to_string()));
        sqlx::query(
            r#"INSERT INTO traffic_logs (id, app_id, timestamp, protocol, local_addr, remote_addr, domain, route_json, bytes_in, bytes_out, verdict_json,
               process_id, source_ip, destination_ip, source_port, destination_port, bytes_sent, bytes_received)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(event.id.to_string())
        .bind(event.app.id().to_string())
        .bind(event.timestamp.to_rfc3339())
        .bind(format!("{:?}", event.protocol))
        .bind(event.local_addr.to_string())
        .bind(event.remote_addr.to_string())
        .bind(&event.remote_domain)
        .bind(route_json)
        .bind(event.bytes_in as i64)
        .bind(event.bytes_out as i64)
        .bind(verdict_json)
        .bind(event.process_id.map(|p| p as i64))
        .bind(src_ip)
        .bind(dst_ip)
        .bind(event.source_port.map(|p| p as i64).or_else(|| Some(event.local_addr.port() as i64)))
        .bind(event.destination_port.map(|p| p as i64).or_else(|| Some(event.remote_addr.port() as i64)))
        .bind(event.bytes_out as i64)
        .bind(event.bytes_in as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, query: TrafficLogQuery) -> Result<Vec<TrafficEvent>> {
        let order = match query.order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        let sort_col = match query.sort {
            TrafficSortField::Timestamp => "t.timestamp",
            TrafficSortField::Bytes => "(t.bytes_in + t.bytes_out)",
        };
        let sql = format!(
            "SELECT t.id, t.app_id, t.timestamp, t.protocol, t.local_addr, t.remote_addr, t.domain, t.route_json, t.bytes_in, t.bytes_out, t.verdict_json,
                    t.process_id, t.source_ip, t.destination_ip, t.source_port, t.destination_port,
                    a.display_name, a.exe_path
             FROM traffic_logs t
             LEFT JOIN apps a ON t.app_id = a.app_id
             WHERE (? IS NULL OR t.app_id = ?)
             ORDER BY {sort_col} {order}
             LIMIT ? OFFSET ?"
        );
        let app_id_str = query.app_id.map(|id| id.to_string());
        let rows = sqlx::query(&sql)
            .bind(app_id_str.as_deref())
            .bind(app_id_str.as_deref())
            .bind(query.limit as i64)
            .bind(query.offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                use sqlx::Row;
                parse_traffic_row((
                    row.get("id"),
                    row.get("app_id"),
                    row.get("timestamp"),
                    row.get("protocol"),
                    row.get("local_addr"),
                    row.get("remote_addr"),
                    row.get("domain"),
                    row.get("route_json"),
                    row.get("bytes_in"),
                    row.get("bytes_out"),
                    row.get("verdict_json"),
                    row.get("process_id"),
                    row.get("source_ip"),
                    row.get("destination_ip"),
                    row.get("source_port"),
                    row.get("destination_port"),
                    row.get("display_name"),
                    row.get("exe_path"),
                ))
            })
            .collect()
    }
}

fn parse_traffic_row(
    row: (
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
        i64,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    ),
) -> Result<TrafficEvent> {
    let (
        id,
        app_id,
        timestamp,
        protocol,
        local_addr,
        remote_addr,
        domain,
        route_json,
        bytes_in,
        bytes_out,
        verdict_json,
        process_id,
        source_ip,
        destination_ip,
        source_port,
        destination_port,
        display_name,
        exe_path,
    ) = row;

    let app_id = app_id
        .and_then(|s| Uuid::parse_str(&s).ok())
        .unwrap_or_else(Uuid::new_v4);
    let display_name = display_name.unwrap_or_else(|| "unknown".into());
    let exe_path = exe_path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("unknown"));
    let now = chrono::Utc::now();
    let record = AppRecord {
        app_id,
        display_name: display_name.clone(),
        exe_path,
        publisher: None,
        sha256: None,
        icon_path: None,
        first_seen: now,
        last_seen: now,
        default_route: None,
        exit_config: None,
    };
    let route: TrafficRoute = serde_json::from_str(&route_json).unwrap_or(TrafficRoute::Direct);
    let verdict: Verdict = verdict_json
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or(Verdict::allow("unknown"));

    Ok(TrafficEvent {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&chrono::Utc),
        app: AppIdentity::new(process_id.unwrap_or(0) as u32, record),
        direction: Direction::Outbound,
        protocol: parse_protocol(&protocol),
        local_addr: local_addr
            .parse()
            .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
        remote_addr: remote_addr
            .parse()
            .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap()),
        remote_domain: domain,
        bytes_in: bytes_in as u64,
        bytes_out: bytes_out as u64,
        route,
        verdict,
        process_id: process_id.map(|p| p as u32),
        source_ip,
        destination_ip,
        source_port: source_port.map(|p| p as u16),
        destination_port: destination_port.map(|p| p as u16),
    })
}

fn parse_protocol(s: &str) -> Protocol {
    match s {
        "Tcp" => Protocol::Tcp,
        "Udp" => Protocol::Udp,
        "Icmp" => Protocol::Icmp,
        _ => Protocol::Other,
    }
}

pub struct SqliteDnsLogRepository {
    pool: SqlitePool,
}

impl SqliteDnsLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DnsLogRepository for SqliteDnsLogRepository {
    async fn insert(&self, log: &DNSQueryLog) -> Result<()> {
        let answers = serde_json::to_string(&log.answers).map_err(WireSentinelError::Serde)?;
        let response = log
            .response
            .clone()
            .or_else(|| Some(log.answers.join(", ")));
        sqlx::query(
            r#"INSERT INTO dns_logs (id, app_id, pid, timestamp, qname, qtype, upstream, blocked, latency_ms, answers_json, response, correlation_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(log.id.to_string())
        .bind(log.app_id.map(|id| id.to_string()))
        .bind(log.pid.map(|p| p as i64))
        .bind(log.timestamp.to_rfc3339())
        .bind(&log.qname)
        .bind(&log.qtype)
        .bind(&log.upstream)
        .bind(log.blocked as i32)
        .bind(log.latency_ms as i64)
        .bind(answers)
        .bind(response)
        .bind(log.correlation_id.map(|id| id.to_string()))
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, query: DnsLogQuery) -> Result<Vec<DNSQueryLog>> {
        let order = match query.order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        let sort_col = match query.sort {
            DnsSortField::Timestamp => "timestamp",
        };
        let mut sql = "SELECT id, app_id, pid, timestamp, qname, qtype, upstream, blocked, latency_ms, answers_json, response, correlation_id
             FROM dns_logs WHERE 1=1".to_string();
        if query.qname.is_some() {
            sql.push_str(" AND qname LIKE ?");
        }
        if query.blocked.is_some() {
            sql.push_str(" AND blocked = ?");
        }
        sql.push_str(&format!(" ORDER BY {sort_col} {order} LIMIT ? OFFSET ?"));

        let mut q = sqlx::query_as::<
            _,
            (
                String,
                Option<String>,
                Option<i64>,
                String,
                String,
                String,
                String,
                i32,
                i64,
                String,
                Option<String>,
                Option<String>,
            ),
        >(&sql);
        if let Some(ref name) = query.qname {
            q = q.bind(format!("%{name}%"));
        }
        if let Some(blocked) = query.blocked {
            q = q.bind(blocked as i32);
        }
        q = q.bind(query.limit as i64).bind(query.offset as i64);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        rows.into_iter().map(parse_dns_row).collect()
    }

    async fn top_domains(&self, limit: u32) -> Result<Vec<TopDomainEntry>> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            r#"SELECT qname, COUNT(*) as total, SUM(CASE WHEN blocked != 0 THEN 1 ELSE 0 END) as blocked
               FROM dns_logs GROUP BY qname ORDER BY total DESC LIMIT ?"#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(domain, total, blocked)| TopDomainEntry {
                domain,
                query_count: total as u64,
                blocked_count: blocked as u64,
            })
            .collect())
    }

    async fn count(&self) -> Result<u64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dns_logs")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(row.0 as u64)
    }
}

fn parse_dns_row(
    (
        id,
        app_id,
        pid,
        timestamp,
        qname,
        qtype,
        upstream,
        blocked,
        latency_ms,
        answers_json,
        response,
        correlation_id,
    ): (
        String,
        Option<String>,
        Option<i64>,
        String,
        String,
        String,
        String,
        i32,
        i64,
        String,
        Option<String>,
        Option<String>,
    ),
) -> Result<DNSQueryLog> {
    let answers: Vec<String> = serde_json::from_str(&answers_json).unwrap_or_default();
    Ok(DNSQueryLog {
        id: Uuid::parse_str(&id).map_err(|e| WireSentinelError::Config(e.to_string()))?,
        timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp)
            .map_err(|e| WireSentinelError::Config(e.to_string()))?
            .with_timezone(&chrono::Utc),
        app_id: app_id.and_then(|s| Uuid::parse_str(&s).ok()),
        pid: pid.map(|p| p as u32),
        qname,
        qtype,
        upstream,
        blocked: blocked != 0,
        latency_ms: latency_ms as u64,
        answers,
        response,
        correlation_id: correlation_id.and_then(|s| Uuid::parse_str(&s).ok()),
    })
}

pub struct SqliteBandwidthRepository {
    pool: SqlitePool,
}

impl SqliteBandwidthRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BandwidthRepository for SqliteBandwidthRepository {
    async fn insert(&self, stats: &BandwidthStats) -> Result<()> {
        sqlx::query(
            "INSERT INTO bandwidth_stats (app_id, interval_start, interval_end, bytes_in, bytes_out, peak_bps) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(stats.app_id.to_string())
        .bind(stats.interval_start.to_rfc3339())
        .bind(stats.interval_end.to_rfc3339())
        .bind(stats.bytes_in as i64)
        .bind(stats.bytes_out as i64)
        .bind(stats.peak_bps as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        Ok(())
    }

    async fn latest_per_app(&self, _limit: u32) -> Result<Vec<BandwidthStats>> {
        Ok(Vec::new())
    }
}
