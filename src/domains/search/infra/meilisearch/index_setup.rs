//! `MeiliSearch` index setup: creates the index and configures searchable /
//! filterable attributes and embedder settings.

use serde_json::{json, Value as JsonValue};

use super::meilisearch_repo::MeiliSearchRepo;
use crate::domains::search::constants::{FILTERABLE_ATTRIBUTES, SEARCHABLE_ATTRIBUTES};

const DEFAULT_EMBEDDER_NAME: &str = "default";
const USER_PROVIDED_EMBEDDER_SOURCE: &str = "userProvided";
const BGE_M3_EMBEDDING_DIMENSIONS: u64 = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SettingsUpdatePlan {
    update_searchable_attributes: bool,
    update_filterable_attributes: bool,
    update_embedder: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmbedderSetupMode {
    HybridReady,
    KeywordOnly,
}

fn desired_searchable_attributes() -> [&'static str; 8] {
    SEARCHABLE_ATTRIBUTES
}

fn desired_filterable_attributes() -> [&'static str; 10] {
    FILTERABLE_ATTRIBUTES
}

fn desired_embedder_settings() -> JsonValue {
    json!({
        "source": USER_PROVIDED_EMBEDDER_SOURCE,
        "dimensions": BGE_M3_EMBEDDING_DIMENSIONS
    })
}

fn desired_embedder_patch_body() -> JsonValue {
    json!({
        "embedders": {
            DEFAULT_EMBEDDER_NAME: desired_embedder_settings()
        }
    })
}

fn json_string_array_matches(value: Option<&JsonValue>, expected: &[&str]) -> bool {
    let Some(items) = value.and_then(JsonValue::as_array) else {
        return false;
    };

    items.len() == expected.len()
        && items
            .iter()
            .zip(expected.iter())
            .all(|(item, expected_value)| item.as_str() == Some(*expected_value))
}

fn embedder_settings_match(settings: &JsonValue) -> bool {
    let Some(embedder) = settings
        .get("embedders")
        .and_then(|value| value.get(DEFAULT_EMBEDDER_NAME))
    else {
        return false;
    };

    embedder.get("source").and_then(JsonValue::as_str) == Some(USER_PROVIDED_EMBEDDER_SOURCE)
        && embedder.get("dimensions").and_then(JsonValue::as_u64)
            == Some(BGE_M3_EMBEDDING_DIMENSIONS)
}

fn settings_update_plan(settings: &JsonValue) -> SettingsUpdatePlan {
    SettingsUpdatePlan {
        update_searchable_attributes: !json_string_array_matches(
            settings.get("searchableAttributes"),
            &SEARCHABLE_ATTRIBUTES,
        ),
        update_filterable_attributes: !json_string_array_matches(
            settings.get("filterableAttributes"),
            &FILTERABLE_ATTRIBUTES,
        ),
        update_embedder: !embedder_settings_match(settings),
    }
}

async fn fetch_current_settings(
    repo: &MeiliSearchRepo,
) -> Result<JsonValue, Box<dyn std::error::Error + Send + Sync>> {
    let settings_url = format!(
        "{}/indexes/{}/settings",
        repo.client.client.get_host().trim_end_matches('/'),
        repo.client.index_name
    );
    let resp = repo
        .http
        .get(settings_url)
        .header(
            "Authorization",
            format!(
                "Bearer {}",
                repo.client.client.get_api_key().unwrap_or_default()
            ),
        )
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch MeiliSearch settings {status}: {body}").into());
    }

    Ok(resp.json().await?)
}

fn format_embedder_patch_error(status: reqwest::StatusCode, body: &str) -> String {
    format!("Failed to set MeiliSearch embedder config ({status}): {body}")
}

fn keyword_only_embedder_message(reason: &str) -> String {
    format!("Semantic search unavailable; continuing with Meili keyword-only search: {reason}")
}

fn handle_embedder_patch_failure(status: reqwest::StatusCode, body: &str) -> EmbedderSetupMode {
    tracing::error!(
        "{}",
        keyword_only_embedder_message(&format_embedder_patch_error(status, body))
    );
    EmbedderSetupMode::KeywordOnly
}

fn handle_embedder_task_failure(error: &str) -> EmbedderSetupMode {
    tracing::error!(
        "{}",
        keyword_only_embedder_message(&format!("embedder setup task failed: {error}"))
    );
    EmbedderSetupMode::KeywordOnly
}

/// Creates the index if it does not already exist, then configures searchable
/// attributes, filterable attributes, and the `userProvided` embedder for
/// vector search.
pub(super) async fn init_index(
    repo: &MeiliSearchRepo,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Tolerate an existing index — create if missing, ignore if present.
    let index_exists = repo
        .client
        .client
        .get_index(&repo.client.index_name)
        .await
        .is_ok();
    if !index_exists {
        let task = repo
            .client
            .client
            .create_index(&repo.client.index_name, None)
            .await?;
        repo.wait_for_task_with_debug(task.get_task_uid(), "create_index")
            .await
            .ok();
    }

    let index = repo.client.index();
    let current_settings = fetch_current_settings(repo).await?;
    let update_plan = settings_update_plan(&current_settings);
    let mut embedder_setup_mode = EmbedderSetupMode::HybridReady;

    if update_plan.update_searchable_attributes {
        let task = index
            .set_searchable_attributes(desired_searchable_attributes())
            .await?;
        repo.wait_for_task_with_debug(task.get_task_uid(), "set_searchable_attributes")
            .await?;
    }

    if update_plan.update_filterable_attributes {
        let task = index
            .set_filterable_attributes(desired_filterable_attributes())
            .await?;
        repo.wait_for_task_with_debug(task.get_task_uid(), "set_filterable_attributes")
            .await?;
    }

    if update_plan.update_embedder {
        embedder_setup_mode = setup_embedder_with_normalization(repo, embedder_setup_mode).await?;
    }

    match embedder_setup_mode {
        EmbedderSetupMode::HybridReady => {
            tracing::info!("MeiliSearch index '{}' initialized", repo.client.index_name);
        }
        EmbedderSetupMode::KeywordOnly => {
            tracing::warn!(
                "MeiliSearch index '{}' initialized without semantic search; keyword search remains available",
                repo.client.index_name
            );
        }
    }
    Ok(())
}

/// Normalize existing documents for the `userProvided` embedder, then PATCH
/// embedder settings via raw HTTP. On normalization failure, degrades to
/// keyword-only mode rather than failing the entire index initialization.
async fn setup_embedder_with_normalization(
    repo: &MeiliSearchRepo,
    mut embedder_setup_mode: EmbedderSetupMode,
) -> Result<EmbedderSetupMode, Box<dyn std::error::Error + Send + Sync>> {
    match repo.normalize_documents_for_user_provided_embedder().await {
        Ok(normalized_count) => {
            if normalized_count > 0 {
                tracing::info!(
                    normalized_count,
                    "Normalized existing MeiliSearch documents for userProvided embedder"
                );
            }
        }
        Err(e) => {
            tracing::error!(
                "{}",
                keyword_only_embedder_message(&format!(
                    "document normalization for userProvided embedder failed: {e}"
                ))
            );
            return Ok(EmbedderSetupMode::KeywordOnly);
        }
    }

    // PATCH embedder settings via raw HTTP — the SDK does not expose embedder config.
    let settings_url = format!(
        "{}/indexes/{}/settings",
        repo.client.client.get_host().trim_end_matches('/'),
        repo.client.index_name
    );
    let embedder_body = desired_embedder_patch_body();
    let resp = repo
        .http
        .patch(&settings_url)
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!(
                "Bearer {}",
                repo.client.client.get_api_key().unwrap_or_default()
            ),
        )
        .json(&embedder_body)
        .send()
        .await?;

    if resp.status().is_success() {
        let task_info: JsonValue = resp.json().await?;
        if let Some(task_uid) = task_info.get("taskUid").and_then(|v| v.as_u64()) {
            if let Err(error) = repo
                .wait_for_task_with_debug(task_uid as u32, "set_embedder")
                .await
            {
                embedder_setup_mode = handle_embedder_task_failure(&error.to_string());
            }
        }
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        embedder_setup_mode = handle_embedder_patch_failure(status, &body);
    }

    Ok(embedder_setup_mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_update_plan_detects_no_drift() {
        let settings = json!({
            "searchableAttributes": desired_searchable_attributes(),
            "filterableAttributes": desired_filterable_attributes(),
            "embedders": {
                "default": {
                    "source": "userProvided",
                    "dimensions": 1024
                }
            }
        });

        let plan = settings_update_plan(&settings);

        assert!(!plan.update_searchable_attributes);
        assert!(!plan.update_filterable_attributes);
        assert!(!plan.update_embedder);
    }

    #[test]
    fn test_settings_update_plan_detects_drift() {
        let settings = json!({
            "searchableAttributes": ["title"],
            "filterableAttributes": ["entity_type"],
            "embedders": {
                "default": {
                    "source": "openAi"
                }
            }
        });

        let plan = settings_update_plan(&settings);

        assert!(plan.update_searchable_attributes);
        assert!(plan.update_filterable_attributes);
        assert!(plan.update_embedder);
    }

    #[test]
    fn test_settings_update_plan_detects_drift_when_embedder_dimensions_missing() {
        let settings = json!({
            "searchableAttributes": desired_searchable_attributes(),
            "filterableAttributes": desired_filterable_attributes(),
            "embedders": {
                "default": {
                    "source": "userProvided"
                }
            }
        });

        let plan = settings_update_plan(&settings);

        assert!(!plan.update_searchable_attributes);
        assert!(!plan.update_filterable_attributes);
        assert!(plan.update_embedder);
    }

    #[test]
    fn test_desired_embedder_patch_body_includes_dimensions() {
        assert_eq!(
            desired_embedder_patch_body(),
            json!({
                "embedders": {
                    "default": {
                        "source": "userProvided",
                        "dimensions": 1024
                    }
                }
            })
        );
    }

    #[test]
    fn test_format_embedder_patch_error_includes_status_and_body() {
        let message =
            format_embedder_patch_error(reqwest::StatusCode::BAD_REQUEST, "missing required field");

        assert!(message.contains("400 Bad Request"));
        assert!(message.contains("missing required field"));
    }

    #[test]
    fn test_handle_embedder_patch_failure_degrades_to_keyword_only() {
        let mode = handle_embedder_patch_failure(
            reqwest::StatusCode::BAD_REQUEST,
            "missing required field",
        );

        assert_eq!(mode, EmbedderSetupMode::KeywordOnly);
    }

    #[test]
    fn test_handle_embedder_task_failure_degrades_to_keyword_only() {
        let mode = handle_embedder_task_failure("task failed");

        assert_eq!(mode, EmbedderSetupMode::KeywordOnly);
    }

    #[test]
    fn test_keyword_only_embedder_message_mentions_keyword_search() {
        let message = keyword_only_embedder_message("embedder patch failed");

        assert!(message.contains("keyword-only"));
        assert!(message.contains("embedder patch failed"));
    }
}
