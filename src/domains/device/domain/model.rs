//! Domain model definitions for device-related entities.
//! This includes enums for device status and OS, as well as the core `Device` struct.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use utoipa::ToSchema;

use crate::common::error::AppError;

/// Enum representing the possible statuses of a device in the system.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum DeviceStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "decommissioned")]
    Decommissioned,
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Pending => "pending",
            Self::Blocked => "blocked",
            Self::Decommissioned => "decommissioned",
        };
        write!(f, "{s}")
    }
}

impl FromStr for DeviceStatus {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "pending" => Ok(Self::Pending),
            "blocked" => Ok(Self::Blocked),
            "decommissioned" => Ok(Self::Decommissioned),
            _ => Err(AppError::ValidationError(format!("Invalid status: {s}"))),
        }
    }
}

impl From<String> for DeviceStatus {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap_or_else(|_| panic!("Invalid device status: {s}"))
    }
}

impl From<&DeviceStatus> for String {
    fn from(val: &DeviceStatus) -> Self {
        val.to_string()
    }
}

/// Enum representing the supported operating systems of a device.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum DeviceOS {
    #[serde(rename = "Android")]
    Android,
    #[serde(rename = "iOS")]
    IOS,
}

impl fmt::Display for DeviceOS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Android => "Android",
            Self::IOS => "iOS",
        };
        write!(f, "{s}")
    }
}

impl FromStr for DeviceOS {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Android" => Ok(Self::Android),
            "iOS" => Ok(Self::IOS),
            _ => Err(AppError::ValidationError(format!("Invalid device_os: {s}"))),
        }
    }
}

impl From<String> for DeviceOS {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap_or_else(|_| panic!("Invalid device OS: {s}"))
    }
}

impl From<&DeviceOS> for String {
    fn from(val: &DeviceOS) -> Self {
        val.to_string()
    }
}

/// Domain model representing a device entity.
#[derive(Debug, Clone)]
pub struct Device {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub device_os: DeviceOS,
    pub status: DeviceStatus,
    pub registered_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_by: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
}
