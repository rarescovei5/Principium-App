use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(sqlx::Type, Serialize, Deserialize, Debug )]
#[sqlx(type_name = "subscription_plan", rename_all = "lowercase")]
pub enum SubscriptionPlan {
    Free,
    Pro
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "subscription_status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    Incomplete,
    PastDue,
    Unpaid
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SubscriptionData {
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub ends_at: Option<DateTime<Utc>>,
}