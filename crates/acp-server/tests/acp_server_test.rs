//! HTTP integration tests for the ACP server.
//!
//! Each test spins up a fresh server on a random port, hits real HTTP endpoints,
//! and asserts on status codes and response bodies.

use serde_json::{json, Value};

/// Start the ACP server on a random port and return the base URL.
async fn start_test_server() -> String {
    let app = ante_acp_server::server::build_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random port");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve failed");
    });

    // Give the server a moment to start accepting connections.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    format!("http://127.0.0.1:{}", addr.port())
}

// ---------------------------------------------------------------------------
// Ping
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ping_returns_200() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/ping"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_object(), "ping should return an object");
}

// ---------------------------------------------------------------------------
// Agents list
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_agents_list_returns_single_ante_agent() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/agents"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["agents"].is_array());

    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 1, "should list exactly one agent");

    let agent = &agents[0];
    assert_eq!(agent["name"], "ante");
    assert!(agent["description"].is_string());
    assert!(agent["input_content_types"].is_array());
    assert!(agent["output_content_types"].is_array());
}

#[tokio::test]
async fn test_agents_list_content_types_match() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/agents"))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let agent = &body["agents"][0];

    let in_types = agent["input_content_types"].as_array().unwrap();
    assert!(in_types.contains(&json!("text/plain")));

    let out_types = agent["output_content_types"].as_array().unwrap();
    assert!(out_types.contains(&json!("text/plain")));
}

#[tokio::test]
async fn test_agents_list_has_metadata() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/agents"))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let meta = &body["agents"][0]["metadata"];

    assert_eq!(meta["programming_language"], "Rust");
    assert!(meta["capabilities"].is_array());
    assert!(meta["tags"].is_array());
    assert!(meta["recommended_models"].is_array());
}

// ---------------------------------------------------------------------------
// Get single agent
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_agents_get_ante() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/agents/ante"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "ante");
    assert!(body["description"].is_string());
}

#[tokio::test]
async fn test_agents_get_unknown_returns_404() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/agents/nonexistent"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "not_found");
}

// ---------------------------------------------------------------------------
// Create run — invalid inputs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_create_invalid_agent_returns_404() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .json(&json!({
            "agent_name": "nonexistent",
            "input": [{
                "role": "user",
                "parts": [{"content_type": "text/plain", "content": "hello"}]
            }]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn test_run_create_empty_input_returns_400() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .json(&json!({
            "agent_name": "ante",
            "input": []
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

#[tokio::test]
async fn test_run_create_message_with_empty_parts_returns_400() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .json(&json!({
            "agent_name": "ante",
            "input": [{
                "role": "user",
                "parts": []
            }]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "invalid_input");
}

// ---------------------------------------------------------------------------
// Create run — async mode (202 Accepted)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_create_async_returns_202() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .json(&json!({
            "agent_name": "ante",
            "input": [{
                "role": "user",
                "parts": [{"content_type": "text/plain", "content": "echo test"}]
            }],
            "mode": "async"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 202);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["agent_name"], "ante");
    assert_eq!(body["status"], "created");
    assert!(body["run_id"].is_string(), "run_id should be a string UUID");
}

#[tokio::test]
async fn test_run_create_async_has_correct_fields() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .json(&json!({
            "agent_name": "ante",
            "input": [{
                "role": "user",
                "parts": [{"content_type": "text/plain", "content": "hello"}]
            }],
            "mode": "async"
        }))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();

    // Should have the standard Run fields
    assert!(body["run_id"].is_string());
    assert!(body["created_at"].is_string());
    assert!(body["output"].is_array());
    assert!(body["output"].as_array().unwrap().is_empty());
    // No error on a freshly created run
    assert!(body["error"].is_null());
}

#[tokio::test]
async fn test_run_create_async_with_session_id() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .json(&json!({
            "agent_name": "ante",
            "session_id": "sess-123",
            "input": [{
                "role": "user",
                "parts": [{"content_type": "text/plain", "content": "hello"}]
            }],
            "mode": "async"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 202);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["session_id"], "sess-123");
}

// ---------------------------------------------------------------------------
// Get run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_get_nonexistent_returns_404() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/runs/00000000-0000-0000-0000-000000000000"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn test_run_get_after_async_create_returns_created() {
    let base = start_test_server().await;

    // Create a run
    let create_resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .json(&json!({
            "agent_name": "ante",
            "input": [{
                "role": "user",
                "parts": [{"content_type": "text/plain", "content": "hello"}]
            }],
            "mode": "async"
        }))
        .send()
        .await
        .unwrap();

    let run_id = create_resp.json::<Value>().await.unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get the run
    let resp = reqwest::Client::new()
        .get(format!("{base}/runs/{run_id}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["run_id"], run_id);
    assert_eq!(body["agent_name"], "ante");
    // Status should be "created" or "in-progress" — either is valid after async create
    let status = body["status"].as_str().unwrap();
    assert!(
        status == "created" || status == "in-progress",
        "expected created or in-progress, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Cancel run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_cancel_nonexistent_returns_404() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs/00000000-0000-0000-0000-000000000000/cancel"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn test_run_cancel_after_async_create() {
    let base = start_test_server().await;

    // Create a run in async mode
    let create_resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .json(&json!({
            "agent_name": "ante",
            "input": [{
                "role": "user",
                "parts": [{"content_type": "text/plain", "content": "hello"}]
            }],
            "mode": "async"
        }))
        .send()
        .await
        .unwrap();

    let run_id = create_resp.json::<Value>().await.unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Small delay so the background task may or may not have started
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Cancel the run — might fail if it already completed, but should at least not panic
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs/{run_id}/cancel"))
        .send()
        .await
        .unwrap();

    // 200 if successfully cancelled, 400 if already in terminal state
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "expected 200 or 400, got {}",
        resp.status()
    );
}

// ---------------------------------------------------------------------------
// Content-Type validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ping_returns_json_content_type() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/ping"))
        .send()
        .await
        .unwrap();

    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.contains("application/json"),
        "expected JSON content-type, got {ct}"
    );
}

#[tokio::test]
async fn test_error_responses_are_json() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/agents/nonexistent"))
        .send()
        .await
        .unwrap();

    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.contains("application/json"),
        "error response should be JSON, got {ct}"
    );
}

// ---------------------------------------------------------------------------
// CORS
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cors_preflight() {
    let base = start_test_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .request(reqwest::Method::OPTIONS, format!("{base}/ping"))
        .header("Origin", "https://example.com")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();

    // CORS layer should respond to preflight; 200 or 204 are both acceptable
    assert!(
        resp.status().is_success(),
        "CORS preflight should succeed, got {}",
        resp.status()
    );
}

// ---------------------------------------------------------------------------
// Missing/malformed body on POST /runs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_create_missing_body_returns_error() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .header("content-type", "application/json")
        .body("")
        .send()
        .await
        .unwrap();

    // axum returns 422 (Unprocessable Entity) or 400 for missing/malformed JSON body
    assert!(
        resp.status().is_client_error(),
        "missing body should return 4xx, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_run_create_invalid_json_body_returns_error() {
    let base = start_test_server().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/runs"))
        .header("content-type", "application/json")
        .body("not json {{{")
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "invalid JSON body should return 4xx, got {}",
        resp.status()
    );
}
