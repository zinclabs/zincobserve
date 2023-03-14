// Copyright 2022 Zinc Labs Inc. and Contributors
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

use ahash::AHashMap;
use chrono::Duration;
use datafusion::arrow::datatypes::{DataType, Schema};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crate::common::str;
use crate::handler::grpc::cluster_rpc;
use crate::infra::config::CONFIG;
use crate::meta::sql::Sql as MetaSql;
use crate::meta::StreamType;
use crate::service::stream::get_stream_setting_fts_fields;
use crate::service::{db, file_list, logs};

const SQL_KEYWORDS: [&str; 32] = [
    "SELECT", "FROM", "WHERE", "TABLE", "LIMIT", "OFFSET", "AND", "OR", "NOT", "IN", "ANY", "IS",
    "NULL", "CASE", "AS", "HAVING", "GROUP", "BY", "ORDER", "ASC", "DESC", "BETWEEN", "LIKE",
    "ILIKE", "DISTINCT", "UNION", "JOIN", "INNER", "OUTER", "INDEX", "LEFT", "RIGHT",
];

const SQL_PUNCTUATION: [u8; 2] = [b'"', b'\''];
const SQL_DELIMITERS: [u8; 10] = [b' ', b'*', b'(', b')', b'<', b'>', b',', b';', b'=', b'!'];
const SQL_FULL_MODE_LIMIT: usize = 1000;

#[derive(Clone, Debug, Serialize)]
pub struct Sql {
    pub origin_sql: String,
    pub org_id: String,
    pub stream_name: String,
    pub meta: MetaSql,
    pub fulltext: Vec<(String, String)>,
    pub aggs: AHashMap<String, (String, MetaSql)>,
    pub fields: Vec<String>,
    pub sql_mode: SqlMode,
    pub schema: Schema,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SqlMode {
    Context,
    Full,
}

impl From<&str> for SqlMode {
    fn from(mode: &str) -> Self {
        match mode.to_lowercase().as_str() {
            "full" => SqlMode::Full,
            "context" => SqlMode::Context,
            _ => SqlMode::Context,
        }
    }
}

impl Display for SqlMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlMode::Context => write!(f, "context"),
            SqlMode::Full => write!(f, "full"),
        }
    }
}

impl Sql {
    #[tracing::instrument(name = "service:search:sql:new", skip(req))]
    pub async fn new(req: &cluster_rpc::SearchRequest) -> Result<Sql, anyhow::Error> {
        let req_query = req.query.as_ref().unwrap();
        let req_time_range = (req_query.start_time, req_query.end_time);
        let org_id = req.org_id.clone();
        let stream_type: StreamType = StreamType::from(req.stream_type.as_str());

        let mut origin_sql = req_query.sql.clone();

        // quick check SQL
        if origin_sql.is_empty() {
            return Err(anyhow::anyhow!("Query SQL is empty"));
        }
        if !origin_sql.to_lowercase().contains(" from ") {
            return Err(anyhow::anyhow!(
                "Query SQL should likes [select * from table]"
            ));
        }

        // Hack for quote
        origin_sql = add_quote_for_sql(&origin_sql);
        // log::info!("[TRACE] origin_sql: {:?}", origin_sql);

        // check sql_mode
        let sql_mode: SqlMode = req_query.sql_mode.as_str().into();
        let mut track_total_hits = if sql_mode.eq(&SqlMode::Full) {
            false
        } else {
            req_query.track_total_hits
        };

        // check SQL
        let re1 = Regex::new(r"(?i) (limit|offset|group|having|join|union) ").unwrap();
        let re2 = Regex::new(r"(?i) (join|union) ").unwrap();
        if sql_mode.eq(&SqlMode::Context) && re1.is_match(&origin_sql) {
            return Err(anyhow::anyhow!(
                "sql_mode=context, Query SQL is not supported [limit|offset|group by|having|join|union]",
            ));
        }
        if sql_mode.eq(&SqlMode::Full) && re2.is_match(&origin_sql) {
            return Err(anyhow::anyhow!(
                "sql_mode=full, Query SQL is not supported [join|union]",
            ));
        }

        // check time_range
        if req_time_range.0 > 0
            && req_time_range.0 < Duration::seconds(1).num_microseconds().unwrap()
        {
            return Err(anyhow::anyhow!(
                "Query SQL time_range start_time should be microseconds",
            ));
        }
        if req_time_range.1 > 0
            && req_time_range.1 < Duration::seconds(1).num_microseconds().unwrap()
        {
            return Err(anyhow::anyhow!(
                "Query SQL time_range start_time should be microseconds",
            ));
        }
        if req_time_range.0 > 0 && req_time_range.1 > 0 && req_time_range.1 < req_time_range.0 {
            return Err(anyhow::anyhow!(
                "Query SQL time_range start_time should be less than end_time",
            ));
        }

        // check Agg SQL
        // 1. must from query
        // 2. disallow select *
        // 3. must select group by field
        let re1 = Regex::new(r"(?i) from[ ]+query").unwrap();
        let re2 = Regex::new(r"(?i)select \*").unwrap();
        let re3 = Regex::new(r#"(?i) group[ ]+by[ ]+([a-zA-Z0-9'"._-]+)"#).unwrap();
        let re4 = Regex::new(r"(?i)select (.*) from[ ]+query").unwrap();
        let mut req_aggs = HashMap::new();
        for agg in req.aggs.iter() {
            req_aggs.insert(agg.name.to_string(), agg.sql.to_string());
        }
        if sql_mode.eq(&SqlMode::Full) && !req_aggs.is_empty() {
            return Err(anyhow::anyhow!("sql_mode=full, Query not supported aggs"));
        }

        // check aggs
        if !req_aggs.is_empty() {
            track_total_hits = true;
        }
        for sql in req_aggs.values() {
            if !re1.is_match(sql.as_str()) {
                return Err(anyhow::anyhow!(
                    "Aggregation SQL only support 'from query' as context",
                ));
            }
            if re2.is_match(sql.as_str()) {
                return Err(anyhow::anyhow!(
                    "Aggregation SQL is not supported 'select *' please specify the fields",
                ));
            }
            if re3.is_match(sql.as_str()) {
                let caps = re3.captures(sql.as_str()).unwrap();
                let group_by = caps
                    .get(1)
                    .unwrap()
                    .as_str()
                    .trim_matches(|v| v == '\'' || v == '"');
                let select_caps = match re4.captures(sql.as_str()) {
                    Some(caps) => caps,
                    None => {
                        return Err(anyhow::anyhow!(
                            "Aggregation SQL should start with 'select'",
                        ))
                    }
                };
                if !select_caps.get(1).unwrap().as_str().contains(group_by) {
                    return Err(anyhow::anyhow!(
                        "Aggregation SQL used [group by] you should select the field [{}]",
                        group_by
                    ));
                }
            }
        }

        // parse sql
        let meta = MetaSql::new(&origin_sql);
        if meta.is_err() {
            return Err(anyhow::anyhow!(meta.err().unwrap()));
        }
        let mut meta = meta.unwrap();

        // fetch schema
        let schema = match db::schema::get(&org_id, &meta.source, Some(stream_type)).await {
            Ok(schema) => schema,
            Err(_) => Schema::empty(),
        };
        let schema_fields = schema.fields().to_vec();

        // Hack for DataFusion
        // DataFusion disallow use `k8s-logs-2022.09.11` as table name
        let stream_name = meta.source.clone();
        let re = Regex::new(&format!(r#"(?i) from[ '"]+{}[ '"]?"#, stream_name)).unwrap();
        let caps = match re.captures(origin_sql.as_str()) {
            Some(caps) => caps,
            None => return Err(anyhow::anyhow!("SQL should likes [select * from table]")),
        };
        origin_sql = origin_sql.replace(caps.get(0).unwrap().as_str(), " FROM tbl ");

        // Hack time range
        if req_time_range.0 > 0 || req_time_range.1 > 0 {
            meta.time_range = Some(req_time_range);
        }
        if req.partition.is_some() {
            let partition = req.partition.as_ref().unwrap();
            meta.time_range = Some((partition.time_min, partition.time_max));
        }
        if let Some(time_range) = meta.time_range {
            let time_range_sql = if time_range.0 > 0 && time_range.1 > 0 {
                format!(
                    "({} >= {} AND {} < {})",
                    CONFIG.common.time_stamp_col,
                    time_range.0,
                    CONFIG.common.time_stamp_col,
                    time_range.1
                )
            } else if time_range.0 > 0 {
                format!("{} >= {}", CONFIG.common.time_stamp_col, time_range.0)
            } else if time_range.1 > 0 {
                format!("{} < {}", CONFIG.common.time_stamp_col, time_range.1)
            } else {
                "".to_string()
            };
            if !time_range_sql.is_empty() {
                let re = Regex::new(r"(?i) WHERE (.*)").unwrap();
                match re.captures(origin_sql.as_str()) {
                    Some(caps) => {
                        let mut where_str = caps.get(1).unwrap().as_str().to_string();
                        if where_str.to_lowercase().contains(" order ") {
                            where_str = where_str
                                [0..where_str.to_lowercase().find(" order ").unwrap()]
                                .to_string();
                        }
                        if !where_str.contains(&CONFIG.common.time_stamp_col) {
                            origin_sql = origin_sql.replace(
                                where_str.as_str(),
                                &format!("{} AND {}", time_range_sql, where_str),
                            );
                        }
                    }
                    None => {
                        origin_sql = origin_sql
                            .replace(" FROM tbl", &format!(" FROM tbl WHERE {}", time_range_sql));
                    }
                };
            }
        }

        // check full sql_mode limit
        if sql_mode.eq(&SqlMode::Full) {
            let origin_sql = origin_sql.to_lowercase();
            if !origin_sql.contains(" limit ")
                && !origin_sql.contains(" group ")
                && !origin_sql.contains(" count(")
                && !origin_sql.contains(&CONFIG.common.time_stamp_col)
            {
                return Err(anyhow::anyhow!("sql_mode=full, Query SQL must limit rows"));
            }
        }

        // Hack offset limit
        meta.offset = req_query.from as usize;
        meta.limit = req_query.size as usize;
        if !sql_mode.eq(&SqlMode::Full) && !origin_sql.to_lowercase().contains(" limit ") {
            origin_sql = if meta.order_by.is_empty() {
                format!(
                    "{} ORDER BY {} DESC LIMIT {}",
                    origin_sql,
                    CONFIG.common.time_stamp_col,
                    meta.offset + meta.limit
                )
            } else {
                format!("{} LIMIT {}", origin_sql, meta.offset + meta.limit)
            };
        }
        // for full sql_mode
        if sql_mode.eq(&SqlMode::Full) && !origin_sql.to_lowercase().contains(" limit ") {
            origin_sql = format!("{} LIMIT {}", origin_sql, SQL_FULL_MODE_LIMIT);
        }

        // HACK full text search
        let mut fulltext = Vec::new();
        let re1 = Regex::new(r"(?i)match_all\('([^']*)'\)").unwrap();
        let re2 = Regex::new(r"(?i)match_all_ignore_case\('([^']*)'\)").unwrap();
        for cap in re1.captures_iter(&origin_sql) {
            // println!("match_all: {}, {}", &cap[0], &cap[1]);
            fulltext.push((cap[0].to_string(), cap[1].to_string()));
        }
        for cap in re2.captures_iter(&origin_sql) {
            // println!("match_all_ignore_case: {}, {}", &cap[0], &cap[1]);
            fulltext.push((cap[0].to_string(), cap[1].to_lowercase()));
        }
        // fetch fts fields
        let fts_fiels = get_stream_setting_fts_fields(&schema).unwrap();
        let match_all_fields = if !fts_fiels.is_empty() {
            fts_fiels.iter().map(|v| v.to_lowercase()).collect()
        } else {
            crate::common::stream::SQL_FULL_TEXT_SEARCH_FIELDS
                .iter()
                .map(|v| v.to_string())
                .collect::<String>()
        };
        for item in fulltext.iter() {
            let mut fulltext_search = Vec::new();
            for field in &schema_fields {
                if !CONFIG.common.feature_fulltext_on_all_fields
                    && !match_all_fields.contains(&field.name().to_lowercase())
                {
                    continue;
                }
                if !field.data_type().eq(&DataType::Utf8) || field.name().starts_with('@') {
                    continue;
                }
                let mut func = "LIKE";
                if item.0.to_lowercase().contains("_ignore_case") {
                    func = "ILIKE";
                }
                fulltext_search.push(format!("\"{}\" {} '%{}%'", field.name(), func, item.1));
            }
            if fulltext_search.is_empty() {
                return Err(anyhow::anyhow!("No full text search field found"));
            }
            let fulltext_search = format!("({})", fulltext_search.join(" OR "));
            origin_sql = origin_sql.replace(item.0.as_str(), &fulltext_search);
        }
        // Hack: str_match
        for key in [
            "match",
            "match_ignore_case",
            "str_match",
            // "str_match_ignore_case", use UDF will get better result
        ] {
            let re_str_match = Regex::new(&format!(r"(?i)\b{}\b\(([^\)]*)\)", key)).unwrap();
            let re_fn = if key == "match" || key == "str_match" {
                "LIKE"
            } else {
                "ILIKE"
            };
            for cap in re_str_match.captures_iter(origin_sql.clone().as_str()) {
                let attrs = cap
                    .get(1)
                    .unwrap()
                    .as_str()
                    .split(',')
                    .map(|v| v.trim().trim_matches(|v| v == '\'' || v == '"'))
                    .collect::<Vec<&str>>();
                let field = attrs.first().unwrap();
                let value = attrs.get(1).unwrap();
                origin_sql = origin_sql.replace(
                    cap.get(0).unwrap().as_str(),
                    format!("\"{}\" {} '%{}%'", field, re_fn, value,).as_str(),
                );
            }
        }

        // query support histogram
        let re_histogram = Regex::new(r"(?i)histogram\(([^\)]*)\)").unwrap();
        for cap in re_histogram.captures_iter(origin_sql.clone().as_str()) {
            let attrs = cap
                .get(1)
                .unwrap()
                .as_str()
                .split(',')
                .map(|v| v.trim().trim_matches(|v| v == '\'' || v == '"'))
                .collect::<Vec<&str>>();
            let field = attrs.first().unwrap();
            let interval = attrs.get(1).unwrap();
            origin_sql = origin_sql.replace(
                cap.get(0).unwrap().as_str(),
                format!(
                    "date_bin(interval '{}', to_timestamp_micros(\"{}\"), to_timestamp('2001-01-01T00:00:00'))",
                    interval,
                   field,
                )
                .as_str(),
            );
        }

        // pickup where
        let re_where = Regex::new(r"(?i) where (.*)").unwrap();
        let mut where_str = match re_where.captures(&origin_sql) {
            Some(caps) => caps[1].to_string(),
            None => "".to_string(),
        };
        if !where_str.is_empty() {
            let mut where_str_lower = where_str.to_lowercase();
            for key in [" order by", " group by", " offset", " limit"].iter() {
                if let Some(pos) = where_str_lower.find(key) {
                    where_str = where_str[..pos].to_string();
                    where_str_lower = where_str.to_lowercase();
                }
            }
        }

        // Hack aggregation
        if track_total_hits {
            req_aggs.insert(
                "_count".to_string(),
                String::from("SELECT COUNT(*) as num from query"),
            );
        }
        let mut aggs = AHashMap::new();
        let re_where = Regex::new(r"(?i) where ").unwrap();
        let re_from = Regex::new(r"(?i) from[ ]+query").unwrap();
        for (key, sql) in &req_aggs {
            let mut sql = sql.to_string();
            if let Some(caps) = re_from.captures(&sql) {
                sql = sql.replace(&caps[0].to_string(), " FROM tbl ");
            }
            if !where_str.is_empty() {
                match re_where.captures(&sql) {
                    Some(caps) => {
                        sql = sql.replace(
                            &caps[0].to_string(),
                            &format!(" WHERE ({}) AND ", where_str),
                        );
                    }
                    None => {
                        sql = sql.replace(
                            &" FROM tbl ".to_string(),
                            &format!(" FROM tbl WHERE ({}) ", where_str),
                        );
                    }
                }
            }
            let sql_meta = MetaSql::new(sql.clone().as_str());
            if sql_meta.is_err() {
                return Err(sql_meta.err().unwrap());
            }
            for cap in re_histogram.captures_iter(sql.clone().as_str()) {
                let attrs = cap
                    .get(1)
                    .unwrap()
                    .as_str()
                    .split(',')
                    .map(|v| v.trim().trim_matches(|v| v == '\'' || v == '"'))
                    .collect::<Vec<&str>>();
                let field = attrs.first().unwrap();
                let interval = attrs.get(1).unwrap();
                sql = sql.replace(
                    cap.get(0).unwrap().as_str(),
                    format!(
                        "date_bin(interval '{}', to_timestamp_micros(\"{}\"), to_timestamp('2001-01-01T00:00:00'))",
                        interval,
                       field,
                    )
                    .as_str(),
                );
            }

            // Hack for quote
            sql = add_quote_for_sql(sql.as_str());

            aggs.insert(key.clone(), (sql, sql_meta.unwrap()));
        }

        let mut sql = Sql {
            origin_sql,
            org_id,
            stream_name,
            meta,
            fulltext,
            aggs,
            fields: vec![],
            sql_mode,
            schema,
        };

        // calculate all needs fields
        for field in schema_fields {
            if check_field_in_use(&sql, field.name()) {
                sql.fields.push(field.name().to_string());
            }
        }

        log::info!(
            "[TRACE] sqlparser: stream_name -> {:?}, fields -> {:?}, partition_key -> {:?}, full_text -> {:?}, time_range -> {:?}, order_by -> {:?}, limit -> {:?},{:?}", 
            sql.stream_name,
            sql.meta.fields,
            sql.meta.quick_text,
            sql.fulltext,
            sql.meta.time_range,
            sql.meta.order_by,
            sql.meta.offset,
            sql.meta.limit,
        );

        Ok(sql)
    }

    /// match a source is a valid file or not
    pub async fn match_source(
        &self,
        source: &str,
        match_min_ts_only: bool,
        stream_type: StreamType,
    ) -> bool {
        // match org_id & table
        if !source.starts_with(
            format!(
                "files/{}/{}/{}/",
                &self.org_id, stream_type, &self.stream_name
            )
            .as_str(),
        ) {
            return false;
        }

        // check partition key
        if !self.filter_source_by_partition_key(source).await {
            return false;
        }

        // check time range
        let file_meta = file_list::get_file_meta(source).await.unwrap_or_default();
        if file_meta.min_ts == 0 || file_meta.max_ts == 0 {
            return true;
        }
        log::trace!(
            "time range: {:?}, file time: {}-{}, {}",
            self.meta.time_range,
            file_meta.min_ts,
            file_meta.max_ts,
            source
        );

        // match partation clause
        if self.meta.time_range.is_some() {
            let (time_min, time_max) = self.meta.time_range.unwrap();
            if match_min_ts_only && time_min > 0 {
                return file_meta.min_ts >= time_min && file_meta.min_ts < time_max;
            }
            if time_min > 0 && time_min > file_meta.max_ts {
                return false;
            }
            if time_max > 0 && time_max < file_meta.min_ts {
                return false;
            }
        }
        true
    }

    /// filter source by partition key
    pub async fn filter_source_by_partition_key(&self, source: &str) -> bool {
        // check partition key
        for key in &self.meta.quick_text {
            let field = logs::get_partition_key_str(format!("{}=", key.0).as_str());
            let value = logs::get_partition_key_str(format!("{}={}", key.0, key.1).as_str());
            if str::find(source, format!("/{}", field).as_str())
                && !str::find(source, format!("/{}/", value).as_str())
            {
                return false;
            }
            // println!("source: {}", source);
            // println!("field: {}, value: {}", field, value);
        }
        true
    }
}

// Hack for double quote
fn add_quote_for_sql(text: &str) -> String {
    let mut tokens = split_token(text);
    add_quote_for_tokens(&mut tokens);
    tokens.join("")
}

fn add_quote_for_tokens(tokens: &mut [String]) {
    let re_lower = Regex::new(r#"^[a-z0-9_]+$"#).unwrap();
    for cap in tokens.iter_mut() {
        let cap_str = cap.as_str();
        let cap_str_upper = cap_str.to_uppercase();
        let cap_str_upper = cap_str_upper.as_str();
        if cap_str.len() == 1 && SQL_DELIMITERS.contains(cap_str.as_bytes().first().unwrap()) {
            continue;
        }
        if SQL_KEYWORDS.contains(&cap_str_upper) {
            continue;
        }
        if SQL_PUNCTUATION.contains(cap_str.as_bytes().first().unwrap()) {
            continue;
        }
        if cap_str.as_bytes().last().unwrap().eq(&b')') {
            // function
            let first_bracket = cap_str.find('(').unwrap();
            let mut tokens = split_token(&cap_str[first_bracket + 1..cap_str.len() - 1]);
            add_quote_for_tokens(&mut tokens);
            *cap = format!("{}({})", &cap_str[0..first_bracket], tokens.join(""));
            continue;
        }
        // level is a special field (keyword for SQL: SET TRANSACTION ISOLATION LEVEL)
        if cap_str_upper != "LEVEL" && re_lower.is_match(cap_str) {
            continue;
        }
        if cap_str.parse::<f64>().is_ok() && cap_str.contains('.') {
            // float number
            *cap = format!("'{}'", cap_str);
            continue;
        }
        *cap = format!("\"{}\"", cap_str);
    }
}

fn split_token(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let text_chars = text.chars().collect::<Vec<char>>();
    let text_chars_len = text_chars.len();
    let mut start_pos = 0;
    let mut in_word = false;
    let mut bracket = 0;
    let mut in_quote = false;
    let mut quote = ' ';
    for i in 0..text_chars_len {
        let c = text_chars.get(i).unwrap();
        if *c == '(' {
            bracket += 1;
            continue;
        }
        if *c == ')' {
            bracket -= 1;
            continue;
        }
        if *c == '\'' || *c == '"' {
            if in_quote {
                if quote == *c {
                    in_quote = false;
                }
            } else {
                in_quote = true;
                quote = *c;
            }
        }
        if SQL_DELIMITERS.contains(&(*c as u8)) {
            if bracket > 0 || in_quote {
                continue;
            }
            if in_word {
                let token = text_chars[start_pos..i].iter().collect::<String>();
                tokens.push(token);
            }
            tokens.push(String::from_utf8(vec![*c as u8]).unwrap());
            in_word = false;
            start_pos = i + 1;
            continue;
        }
        if in_word {
            continue;
        }
        in_word = true;
    }
    if start_pos != text_chars_len {
        let token = text_chars[start_pos..text_chars_len]
            .iter()
            .collect::<String>();
        tokens.push(token);
    }

    // println!("tokens: {:?}", tokens);
    tokens
}

fn check_field_in_use(sql: &Sql, field: &str) -> bool {
    let re = Regex::new(&format!(r"\b{}\b", field)).unwrap();
    if str::find(sql.origin_sql.as_str(), field) && re.is_match(sql.origin_sql.as_str()) {
        return true;
    }
    for (_, sql) in sql.aggs.iter() {
        if str::find(sql.0.as_str(), field) && re.is_match(sql.0.as_str()) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn sql_works() {
        let org_id = "test_org";
        let col = "_timestamp";
        let table = "default";
        let query = crate::meta::search::Query {
            sql: format!("select {} from {} ", col, table),
            from: 0,
            size: 100,
            sql_mode: "full".to_owned(),
            start_time: 1667978895416,
            end_time: 1667978900217,
            track_total_hits: false,
        };

        let req: crate::meta::search::Request = crate::meta::search::Request {
            query,
            aggs: HashMap::new(),
            encoding: crate::meta::search::RequestEncoding::Empty,
        };

        let mut rpc_req: cluster_rpc::SearchRequest = req.to_owned().into();
        rpc_req.org_id = org_id.to_string();

        let resp = Sql::new(&rpc_req).await.unwrap();
        assert_eq!(resp.stream_name, table);
        assert_eq!(resp.org_id, org_id);
        assert!(check_field_in_use(&resp, col));
    }

    #[actix_web::test]
    async fn test_add_quote_for_sql() {
        let samples = [
            (
                "select * from table",
                "select * from table",
            ),
            (
                "select ab.c from table ",
                r#"select "ab.c" from table "#,
            ),
            (
                "select * from table where a = 1",
                "select * from table where a = 1",
            ),
            (
                "select * from table where a=1",
                "select * from table where a=1",
            ),
            (
                "select * from table where County='中文'",
                r#"select * from table where "County"='中文'"#,
            ),
            (
                "select * from table where match_all(123)",
                "select * from table where match_all(123)",
            ),
            (
                "select * from table where match_all('123')",
                "select * from table where match_all('123')",
            ),
            (
                "select * from table where match_all('中文')",
                "select * from table where match_all('中文')",
            ),
            (
                "select count(*) from table where match_all('中文')",
                "select count(*) from table where match_all('中文')",
            ),
            (
                "select count(*) as abc from table where match_all('中文')",
                "select count(*) as abc from table where match_all('中文')",
            ),
            (
                "select count(abc) as abc from table where match_all('中文')",
                "select count(abc) as abc from table where match_all('中文')",
            ),
            (
                "select count(Abc) as abc from table where match_all('中文')",
                r#"select count("Abc") as abc from table where match_all('中文')"#,
            ),
            (
                "select count(_p,stream) as newcol from table",
                "select count(_p,stream) as newcol from table",
            ),
            (
                "select count(_p,Stream) as newcol from table",
                r#"select count(_p,"Stream") as newcol from table"#,
            ),
            (
                "select count(_P,Stream) as newcol from table",
                r#"select count("_P","Stream") as newcol from table"#,
            ),
            (
                "select * from table where str_match(log,'中文')",
                "select * from table where str_match(log,'中文')",
            ),
            (
                "select * from table where str_match(log, '中文')",
                "select * from table where str_match(log, '中文')",
            ),
            (
                "select * from table where str_match(log, 'a=b')",
                "select * from table where str_match(log, 'a=b')",
            ),
            (
                "select * from table wherkubernetes.pod='dc03-eed1-be27'",
                r#"select * from table "wherkubernetes.pod"='dc03-eed1-be27'"#,
            ),
            (
                "select * from table where str_match(log, '中文') AND match_all('abc') AND time_range(_timestamp, '2020-01-01 00:00:00', '2020-01-01 00:00:00')",
                "select * from table where str_match(log, '中文') AND match_all('abc') AND time_range(_timestamp, '2020-01-01 00:00:00', '2020-01-01 00:00:00')",
            ),
            (
                "select * from table where str_match(log, '中文') AND match_all('abc') AND time_range(_timestamp, '2020-01-01 00:00:00', '2020-01-01 00:00:00') ORDER BY _timestamp DESC limit 10",
                "select * from table where str_match(log, '中文') AND match_all('abc') AND time_range(_timestamp, '2020-01-01 00:00:00', '2020-01-01 00:00:00') ORDER BY _timestamp DESC limit 10",
            ),
            (
                "select * from table where str_match(log, '中文') AND match_all('abc') AND time_range(Timestamp, '2020-01-01 00:00:00', '2020-01-01 00:00:00') ORDER BY _timestamp DESC limit 10",
                r#"select * from table where str_match(log, '中文') AND match_all('abc') AND time_range("Timestamp", '2020-01-01 00:00:00', '2020-01-01 00:00:00') ORDER BY _timestamp DESC limit 10"#,
            ),
            (
                "select * from table where str_match(log, 'Sql: select * from table')",
                "select * from table where str_match(log, 'Sql: select * from table')",
            ),
            (
                r#"select * from table where str_match(log, 'Sql: select * from table where "a" = 1')"#,
                r#"select * from table where str_match(log, 'Sql: select * from table where "a" = 1')"#,
            ),
            (
                r#"select * from table where str_match(log, 'Sql: select * from table where "a" = \'1\'')"#,
                r#"select * from table where str_match(log, 'Sql: select * from table where "a" = \'1\'')"#,
            ),
            (
                "select * from table WHERE remote_addr='110.6.45.247' and request_uri='GET /api/mp-weixin/rxjh-checkline/check_line_status/?region_id=6' and body_bytes_sent=4500 and request_time = 0.004",
                "select * from table WHERE remote_addr='110.6.45.247' and request_uri='GET /api/mp-weixin/rxjh-checkline/check_line_status/?region_id=6' and body_bytes_sent=4500 and request_time = '0.004'",
            ),
            (
                r#"select * from table WHERE log='10.2.69.251 - prabhat@zinclabs.io [10/Mar/2023:12:43:53 +0000] "POST /api/demo_org1_n976k98gUMT17m3/_bulk HTTP/2.0" 200 111 "-" "go-resty/2.7.0 (https://github.com/go-resty/resty)" 633 0.005 [zinc-cp1-zinc-cp-4082] [] 10.2.34.102:4082 127 0.004 200 ef85bcdfc57709b6f9f4a3a117a22c55'"#,
                r#"select * from table WHERE log='10.2.69.251 - prabhat@zinclabs.io [10/Mar/2023:12:43:53 +0000] "POST /api/demo_org1_n976k98gUMT17m3/_bulk HTTP/2.0" 200 111 "-" "go-resty/2.7.0 (https://github.com/go-resty/resty)" 633 0.005 [zinc-cp1-zinc-cp-4082] [] 10.2.34.102:4082 127 0.004 200 ef85bcdfc57709b6f9f4a3a117a22c55'"#,
            ),
            (
                "select * from table where kubernetes.labels.pod-template-hash='7d8765890' and time<10 and time>10 and time>=29 and kubernetes.container_name!='controller'",
                r#"select * from table where "kubernetes.labels.pod-template-hash"='7d8765890' and time<10 and time>10 and time>=29 and "kubernetes.container_name"!='controller'"#,
            ),
        ];

        for (i, (sql, quoted_sql)) in samples.into_iter().enumerate() {
            assert_eq!(add_quote_for_sql(sql), quoted_sql, "sample #{i}");
        }
    }
}
