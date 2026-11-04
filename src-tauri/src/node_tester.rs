use std::time::{Duration, Instant};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::json;
use crate::models::{RelayNode, NodeTestResult};

pub async fn test_node_connectivity(node: &RelayNode, test_model: bool) -> NodeTestResult {
    let base_url = node.base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        return NodeTestResult {
            node_id: node.id.clone(),
            connectivity_ok: false,
            latency_ms: None,
            status_code: None,
            model_ok: None,
            message: "节点 Base URL 不能为空".to_string(),
        };
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .danger_accept_invalid_certs(false)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return NodeTestResult {
                node_id: node.id.clone(),
                connectivity_ok: false,
                latency_ms: None,
                status_code: None,
                model_ok: None,
                message: format!("创建网络客户端失败: {}", e),
            };
        }
    };

    // 1. Base Connectivity Check (Ping /models or root)
    let start_time = Instant::now();
    let ping_url = if base_url.ends_with("/v1") {
        format!("{}/models", base_url)
    } else {
        format!("{}/v1/models", base_url)
    };

    let mut headers = HeaderMap::new();
    if !node.auth_token.trim().is_empty() {
        if node.auth_mode == "apiKey" {
            if let Ok(val) = HeaderValue::from_str(node.auth_token.trim()) {
                headers.insert("x-api-key", val);
            }
        } else {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", node.auth_token.trim())) {
                headers.insert("Authorization", val);
            }
            if let Ok(val) = HeaderValue::from_str(node.auth_token.trim()) {
                headers.insert("x-api-key", val);
            }
        }
    }
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

    let resp_res = client.get(&ping_url)
        .headers(headers.clone())
        .send()
        .await;

    let latency = start_time.elapsed().as_millis() as u64;

    match resp_res {
        Ok(resp) => {
            let status = resp.status().as_u16();

            // If only connectivity test was requested
            if !test_model {
                let ok = status < 500; // 200, 401, 404 all indicate host is reachable
                let msg = if status == 200 {
                    format!("连接正常 ({}ms) · HTTP {}", latency, status)
                } else if status == 401 || status == 403 {
                    format!("主机可达 ({}ms) · 认证未通过 (HTTP {})", latency, status)
                } else {
                    format!("主机可达 ({}ms) · HTTP {}", latency, status)
                };

                return NodeTestResult {
                    node_id: node.id.clone(),
                    connectivity_ok: ok,
                    latency_ms: Some(latency),
                    status_code: Some(status),
                    model_ok: None,
                    message: msg,
                };
            }

            // 2. Model Test Requested
            let model_name = node.models.default_model.as_ref()
                .or(node.models.sonnet.as_ref())
                .or(node.models.opus.as_ref())
                .cloned()
                .unwrap_or_else(|| "claude-3-7-sonnet-20250219".to_string());

            let messages_url = if base_url.ends_with("/v1") {
                format!("{}/messages", base_url)
            } else {
                format!("{}/v1/messages", base_url)
            };

            let test_body = json!({
                "model": model_name,
                "max_tokens": 1,
                "messages": [
                    {"role": "user", "content": "ping"}
                ]
            });

            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

            let model_start = Instant::now();
            let model_res = client.post(&messages_url)
                .headers(headers)
                .json(&test_body)
                .send()
                .await;

            let total_latency = model_start.elapsed().as_millis() as u64;

            match model_res {
                Ok(m_resp) => {
                    let m_status = m_resp.status().as_u16();
                    if m_status == 200 {
                        NodeTestResult {
                            node_id: node.id.clone(),
                            connectivity_ok: true,
                            latency_ms: Some(total_latency),
                            status_code: Some(m_status),
                            model_ok: Some(true),
                            message: format!("模型调用成功 ({}ms) · 状态正常", total_latency),
                        }
                    } else if m_status == 401 || m_status == 403 {
                        NodeTestResult {
                            node_id: node.id.clone(),
                            connectivity_ok: true,
                            latency_ms: Some(total_latency),
                            status_code: Some(m_status),
                            model_ok: Some(false),
                            message: format!("认证失败 (HTTP {}) · 请检查密钥/Token", m_status),
                        }
                    } else if m_status == 404 {
                        NodeTestResult {
                            node_id: node.id.clone(),
                            connectivity_ok: true,
                            latency_ms: Some(total_latency),
                            status_code: Some(m_status),
                            model_ok: Some(false),
                            message: format!("模型不存在或路径不支持 (HTTP 404) · 模型: {}", model_name),
                        }
                    } else if m_status == 429 {
                        NodeTestResult {
                            node_id: node.id.clone(),
                            connectivity_ok: true,
                            latency_ms: Some(total_latency),
                            status_code: Some(m_status),
                            model_ok: Some(false),
                            message: "速率限制或账户余额不足 (HTTP 429)".to_string(),
                        }
                    } else {
                        NodeTestResult {
                            node_id: node.id.clone(),
                            connectivity_ok: true,
                            latency_ms: Some(total_latency),
                            status_code: Some(m_status),
                            model_ok: Some(false),
                            message: format!("服务返回错误 (HTTP {})", m_status),
                        }
                    }
                }
                Err(e) => {
                    NodeTestResult {
                        node_id: node.id.clone(),
                        connectivity_ok: true,
                        latency_ms: Some(total_latency),
                        status_code: None,
                        model_ok: Some(false),
                        message: format!("模型请求异常: {}", e),
                    }
                }
            }
        }
        Err(e) => {
            let err_msg = if e.is_timeout() {
                "连接超时 (超过 6 秒未响应)".to_string()
            } else if e.is_connect() {
                "无法连接到主机 (DNS解析失败或端口无法连通)".to_string()
            } else {
                format!("网络连接失败: {}", e)
            };

            NodeTestResult {
                node_id: node.id.clone(),
                connectivity_ok: false,
                latency_ms: None,
                status_code: None,
                model_ok: None,
                message: err_msg,
            }
        }
    }
}
