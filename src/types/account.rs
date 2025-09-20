use serde::{Deserialize, Serialize};
use serde_json::{Value};
use chrono::DateTime;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    active_cons: Value,
    allowed_output_formats: Value,
    auth: Value,
    created_at: Value,
    exp_date: Value,
    is_trial: Value,
    max_connections: Value,
    message: String,
    password: String,
    pub status: String,
    username: String,
}

pub trait UserTrait {
    fn active_cons(&self) -> u64;
    fn exp_date(&self) -> String;
    fn created_at(&self) -> String;
    fn is_trial(&self) -> bool;
    fn max_connections(&self) -> u64;
}

impl UserTrait for UserInfo {
    fn active_cons(&self) -> u64 {
        self.active_cons.as_u64().unwrap_or(0)
    }
    fn exp_date(&self) -> String {
        let exp_ts = match self.exp_date.as_str() {
            Some(s) => s.parse().unwrap(),
            _ => self.exp_date.as_i64().unwrap_or_default(),
        };
        DateTime::from_timestamp(exp_ts, 0)
            .unwrap_or_default()
            .to_string()
    }
    fn created_at(&self) -> String {
        let created_ts = match self.created_at.as_str() {
            Some(s) => s.parse().unwrap(),
            _ => self.created_at.as_i64().unwrap_or_default(),
        };
        DateTime::from_timestamp(created_ts, 0)
            .unwrap_or_default()
            .to_string()
    }
    fn is_trial(&self) -> bool {
        match self.is_trial.is_boolean() {
            true => self.is_trial.as_bool().unwrap(),
            false => matches!(self.is_trial.as_str(), Some("1")),
        }
    }
    fn max_connections(&self) -> u64 {
        match self.max_connections.as_str() {
            Some(s) => s.parse().unwrap(),
            _ => self.max_connections
                .as_u64()
                .unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    server_info: Value,
    pub user_info: UserInfo,
}
