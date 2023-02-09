use super::search::Query;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub column: String,
    pub operator: AllOperator,
    #[serde(default)]
    pub ignore_case: Option<bool>,
    pub value: serde_json::Value,
    pub is_numeric: Option<bool>,
}

impl Evaluate for Condition {
    fn evaluate(&self, row: Map<String, Value>) -> bool {
        if !row.contains_key(&self.column) {
            return false;
        };

        let evaluate_numeric = if self.is_numeric.is_some() {
            self.is_numeric.unwrap()
        } else {
            matches!(row.get(&self.column).unwrap(), serde_json::Value::Number(_))
        };

        if evaluate_numeric {
            let number = match row.get(&self.column).expect("column exists") {
                serde_json::Value::Number(number) => number,
                _ => unreachable!("please make sure right value for is_numeric is set trigger"),
            };

            match self.operator {
                AllOperator::EqualTo => number.as_f64().unwrap() == self.value.as_f64().unwrap(),
                AllOperator::NotEqualTo => number.as_f64().unwrap() != self.value.as_f64().unwrap(),
                AllOperator::GreaterThan => number.as_f64().unwrap() > self.value.as_f64().unwrap(),
                AllOperator::GreaterThanEquals => {
                    number.as_f64().unwrap() >= self.value.as_f64().unwrap()
                }
                AllOperator::LessThan => number.as_f64().unwrap() < self.value.as_f64().unwrap(),
                AllOperator::LessThanEquals => {
                    number.as_f64().unwrap() <= self.value.as_f64().unwrap()
                }
                _ => false,
            }
        } else {
            let string = match row.get(&self.column).expect("column exists") {
                serde_json::Value::String(s) => s,
                _ => unreachable!("please make sure right value for is_numeric is set trigger"),
            };

            if self.ignore_case.unwrap_or_default() {
                match self.operator {
                    AllOperator::EqualTo => {
                        string.eq_ignore_ascii_case(self.value.as_str().unwrap())
                    }
                    AllOperator::NotEqualTo => {
                        !string.eq_ignore_ascii_case(self.value.as_str().unwrap())
                    }
                    AllOperator::Contains => string
                        .to_ascii_lowercase()
                        .contains(&self.value.as_str().unwrap().to_ascii_lowercase()),
                    AllOperator::NotContains => !string
                        .to_ascii_lowercase()
                        .contains(&self.value.as_str().unwrap().to_ascii_lowercase()),
                    _ => false,
                }
            } else {
                match self.operator {
                    AllOperator::EqualTo => string.eq(self.value.as_str().unwrap()),
                    AllOperator::NotEqualTo => !string.eq(self.value.as_str().unwrap()),
                    AllOperator::Contains => string.contains(self.value.as_str().unwrap()),
                    AllOperator::NotContains => !string.contains(self.value.as_str().unwrap()),
                    _ => false,
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllOperator {
    #[serde(alias = "=")]
    EqualTo,
    #[serde(alias = "!=")]
    NotEqualTo,
    #[serde(alias = ">")]
    GreaterThan,
    #[serde(alias = ">=")]
    GreaterThanEquals,
    #[serde(alias = "<")]
    LessThan,
    #[serde(alias = "<=")]
    LessThanEquals,
    Contains,
    NotContains,
}

impl Default for AllOperator {
    fn default() -> Self {
        Self::EqualTo
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub stream: String,
    pub query: Option<Query>,
    pub condition: Condition,
    pub duration: i64,
    pub frequency: i64,
    pub time_between_alerts: i64,
    pub destination: String,
    #[serde(default)]
    pub is_real_time: bool,
}

impl PartialEq for Alert {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.stream == other.stream
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertList {
    pub list: Vec<Alert>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trigger {
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub is_valid: bool,
    #[serde(default)]
    pub alert_name: String,
    #[serde(default)]
    pub stream: String,
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub last_sent_at: i64,
    #[serde(default)]
    pub count: i64,
    #[serde(default)]
    pub is_ingest_time: bool,
}

impl Default for Trigger {
    fn default() -> Self {
        Trigger {
            timestamp: 0,
            is_valid: true,
            alert_name: String::new(),
            stream: String::new(),
            org: String::new(),
            last_sent_at: 0,
            count: 0,
            is_ingest_time: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerTimer {
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub expires_at: i64,
}

pub trait Evaluate {
    fn evaluate(&self, row: Map<String, Value>) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate() {
        let condition = Condition {
            column: "occurance".to_owned(),
            operator: AllOperator::GreaterThanEquals,
            ignore_case: None,
            value: serde_json::json!(5),
            is_numeric: None,
        };

        let eq_condition = Condition {
            column: "Country".to_owned(),
            operator: AllOperator::EqualTo,
            ignore_case: None,
            value: serde_json::json!("USA"),
            is_numeric: None,
        };

        let row = serde_json::json!({"Country":"USA","occurance": 10});
        assert_eq!(condition.evaluate(row.as_object().unwrap().clone()), true);
        assert_eq!(
            eq_condition.evaluate(row.as_object().unwrap().clone()),
            true
        );
    }
}
