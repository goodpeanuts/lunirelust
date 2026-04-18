//! `MeiliSearch` index setup: creates the index and configures searchable /
//! filterable attributes and embedder settings.

use serde_json::{json, Value as JsonValue};

use super::meilisearch_repo::MeiliSearchRepo;

/// Creates the index if it does not already exist, then configures searchable
/// attributes, filterable attributes, and the `userProvided` embedder for
/// vector search.
#[expect(clippy::too_many_lines)]
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
        repo.client
            .client
            .wait_for_task(task, None, None)
            .await
            .ok();
    }

    let index = repo.client.index();

    let task = index
        .set_searchable_attributes([
            "title",
            "entity_id",
            "director_name",
            "studio_name",
            "label_name",
            "series_name",
            "genre_names",
            "idol_names",
        ])
        .await?;
    repo.client.client.wait_for_task(task, None, None).await?;

    let task = index
        .set_filterable_attributes([
            "entity_type",
            "date",
            "duration",
            "permission",
            "director_name",
            "studio_name",
            "label_name",
            "series_name",
            "genre_names",
            "idol_names",
        ])
        .await?;
    repo.client.client.wait_for_task(task, None, None).await?;

    // Configure userProvided embedder for vector search via raw HTTP.
    // The SDK (0.28) does not expose embedder settings, so we PATCH the
    // index settings directly through MeiliSearch's REST API.
    let settings_url = format!(
        "{}/indexes/{}/settings",
        repo.client.client.get_host(),
        repo.client.index_name
    );
    let embedder_body = json!({
        "embedders": {
            "default": {
                "source": "userProvided"
            }
        }
    });
    let http_client = &repo.http;
    let resp = http_client
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
            // Wait for the task to complete (simple polling)
            let task_url = format!("{}/tasks/{}", repo.client.client.get_host(), task_uid);
            for _ in 0..60 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let task_resp = http_client
                    .get(&task_url)
                    .header(
                        "Authorization",
                        format!(
                            "Bearer {}",
                            repo.client.client.get_api_key().unwrap_or_default()
                        ),
                    )
                    .send()
                    .await?;
                let task_status: JsonValue = task_resp.json().await?;
                if let Some(status) = task_status.get("status").and_then(|v| v.as_str()) {
                    match status {
                        "succeeded" => break,
                        "failed" => {
                            tracing::warn!("Embedder setup task failed: {:?}", task_status);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    tracing::info!("MeiliSearch index '{}' initialized", repo.client.index_name);
    Ok(())
}
