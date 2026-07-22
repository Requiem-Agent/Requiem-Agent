// openapi.rs — S9-03: OpenAPI/Swagger Documentation
// توثيق تلقائي لجميع API endpoints باستخدام utoipa
//
// يُولّد:
//   GET /api/docs → Swagger UI HTML
//   GET /api/openapi.json → OpenAPI 3.0 JSON spec

use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// OpenAPI spec (يدوي — بدون utoipa macro لتجنب dependency issues)
// ─────────────────────────────────────────────────────────────────────────────

/// يُولّد OpenAPI 3.0 spec كـ JSON
pub fn generate_openapi_spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Requiem Agent API",
            "description": "واجهة برمجية لـ Requiem Agent — نظام AI agent مستقل مبني بـ Rust + React",
            "version": "1.0.0",
            "contact": {
                "name": "Requiem Agent Team",
                "url": "https://github.com/Requiem-Agent/Requiem-Agent"
            },
            "license": {
                "name": "MIT",
                "url": "https://opensource.org/licenses/MIT"
            }
        },
        "servers": [
            {
                "url": "http://localhost:7860",
                "description": "Local development"
            },
            {
                "url": "https://rayig-dev.hf.space",
                "description": "HuggingFace Spaces (production)"
            }
        ],
        "security": [
            { "BearerAuth": [] }
        ],
        "components": {
            "securitySchemes": {
                "BearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            },
            "schemas": {
                "UserPreferences": {
                    "type": "object",
                    "properties": {
                        "theme": { "type": "string", "enum": ["dark", "light", "system"] },
                        "language": { "type": "string", "example": "ar" },
                        "compact_mode": { "type": "boolean" },
                        "default_model": { "type": "string", "example": "claude-sonnet-4-5" },
                        "default_mode": { "type": "string", "enum": ["chat", "orchestrator", "code"] },
                        "temperature": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
                        "max_tokens": { "type": "integer", "minimum": 1, "maximum": 200000 },
                        "stream_responses": { "type": "boolean" }
                    }
                },
                "AgentChatRequest": {
                    "type": "object",
                    "required": ["message"],
                    "properties": {
                        "message": { "type": "string", "example": "ما هو الذكاء الاصطناعي؟" },
                        "mode": { "type": "string", "enum": ["chat", "orchestrator", "code"], "default": "chat" },
                        "model": { "type": "string", "example": "claude-sonnet-4-5" },
                        "session_id": { "type": "string", "format": "uuid" },
                        "max_steps": { "type": "integer", "default": 10 }
                    }
                },
                "AgentChatResponse": {
                    "type": "object",
                    "properties": {
                        "success": { "type": "boolean" },
                        "data": {
                            "type": "object",
                            "properties": {
                                "reply": { "type": "string" },
                                "session_id": { "type": "string" },
                                "steps": { "type": "integer" },
                                "tokens_used": { "type": "integer" }
                            }
                        }
                    }
                },
                "StoredApiKey": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "provider": { "type": "string", "enum": ["anthropic", "openai", "gemini", "mistral"] },
                        "key_hint": { "type": "string", "example": "sk-ant-...xxxx" },
                        "created_at": { "type": "string", "format": "date-time" }
                    }
                },
                "Error": {
                    "type": "object",
                    "properties": {
                        "success": { "type": "boolean", "example": false },
                        "error": { "type": "string" },
                        "code": { "type": "integer" }
                    }
                }
            }
        },
        "paths": {
            "/healthz": {
                "get": {
                    "tags": ["System"],
                    "summary": "Health check",
                    "description": "يتحقق من أن الـ server يعمل",
                    "security": [],
                    "responses": {
                        "200": {
                            "description": "Server is healthy",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "status": { "type": "string", "example": "ok" },
                                            "version": { "type": "string" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/metrics": {
                "get": {
                    "tags": ["System"],
                    "summary": "Prometheus metrics",
                    "description": "يُرجع metrics بصيغة Prometheus text format",
                    "security": [],
                    "responses": {
                        "200": {
                            "description": "Prometheus metrics",
                            "content": {
                                "text/plain": {
                                    "schema": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/agent/chat": {
                "post": {
                    "tags": ["Agent"],
                    "summary": "Send message to agent",
                    "description": "يُرسل رسالة للـ agent ويستقبل الرد. يدعم chat و orchestrator و code modes.",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/AgentChatRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Agent response",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/AgentChatResponse" }
                                }
                            }
                        },
                        "400": { "description": "Invalid request", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } },
                        "401": { "description": "Unauthorized" },
                        "429": { "description": "Rate limit exceeded" }
                    }
                }
            },
            "/ws/agent": {
                "get": {
                    "tags": ["Agent"],
                    "summary": "WebSocket streaming endpoint",
                    "description": "WebSocket connection للـ real-time agent streaming. Protocol: start/cancel/ping messages.",
                    "responses": {
                        "101": { "description": "WebSocket upgrade successful" },
                        "401": { "description": "Unauthorized" }
                    }
                }
            },
            "/api/preferences": {
                "get": {
                    "tags": ["Preferences"],
                    "summary": "Get user preferences",
                    "responses": {
                        "200": {
                            "description": "User preferences",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "success": { "type": "boolean" },
                                            "data": { "$ref": "#/components/schemas/UserPreferences" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "put": {
                    "tags": ["Preferences"],
                    "summary": "Update user preferences (full update)",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/UserPreferences" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Preferences updated" }
                    }
                },
                "patch": {
                    "tags": ["Preferences"],
                    "summary": "Partial update user preferences",
                    "description": "يُحدّث فقط الحقول المُرسَلة",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/UserPreferences" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Preferences partially updated" }
                    }
                }
            },
            "/api/user-api-keys": {
                "get": {
                    "tags": ["API Keys"],
                    "summary": "List stored API keys",
                    "description": "يُرجع قائمة المفاتيح المخزَّنة (بدون plaintext)",
                    "responses": {
                        "200": {
                            "description": "List of stored keys",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": { "$ref": "#/components/schemas/StoredApiKey" }
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "tags": ["API Keys"],
                    "summary": "Save encrypted API key",
                    "description": "يحفظ مفتاح LLM provider مشفَّراً بـ AES-256-GCM",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["provider", "api_key"],
                                    "properties": {
                                        "provider": { "type": "string", "enum": ["anthropic", "openai", "gemini", "mistral"] },
                                        "api_key": { "type": "string" }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": { "description": "Key saved" },
                        "400": { "description": "Invalid provider or key" }
                    }
                }
            },
            "/api/user-api-keys/{id}": {
                "delete": {
                    "tags": ["API Keys"],
                    "summary": "Delete API key",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string", "format": "uuid" }
                        }
                    ],
                    "responses": {
                        "200": { "description": "Key deleted" },
                        "404": { "description": "Key not found" }
                    }
                }
            }
        },
        "tags": [
            { "name": "System", "description": "Health, metrics, and system endpoints" },
            { "name": "Agent", "description": "AI agent chat and streaming" },
            { "name": "Preferences", "description": "User preferences management" },
            { "name": "API Keys", "description": "LLM provider API key management" }
        ]
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/openapi.json — يُرجع الـ spec كـ JSON
pub async fn openapi_json() -> impl IntoResponse {
    Json(generate_openapi_spec())
}

/// GET /api/docs — Swagger UI
pub async fn swagger_ui() -> impl IntoResponse {
    Html(r#"<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head>
  <meta charset="UTF-8">
  <title>Requiem Agent API Docs</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
  <style>
    body { margin: 0; background: #0f172a; }
    .swagger-ui .topbar { background: #1e293b; }
    .swagger-ui .topbar .download-url-wrapper { display: none; }
    .swagger-ui .info .title { color: #e2e8f0; }
  </style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({
      url: '/api/openapi.json',
      dom_id: '#swagger-ui',
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: 'BaseLayout',
      deepLinking: true,
      tryItOutEnabled: true,
    });
  </script>
</body>
</html>"#)
}

/// يُضيف routes الـ docs للـ router
pub fn docs_router() -> Router {
    Router::new()
        .route("/api/docs", get(swagger_ui))
        .route("/api/openapi.json", get(openapi_json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_valid_structure() {
        let spec = generate_openapi_spec();
        assert_eq!(spec["openapi"].as_str(), Some("3.0.3"));
        assert!(spec["info"]["title"].as_str().is_some());
        assert!(spec["paths"].is_object());
        assert!(spec["components"]["schemas"].is_object());
    }

    #[test]
    fn test_all_required_paths_present() {
        let spec = generate_openapi_spec();
        let paths = &spec["paths"];
        assert!(paths["/healthz"].is_object());
        assert!(paths["/metrics"].is_object());
        assert!(paths["/api/agent/chat"].is_object());
        assert!(paths["/ws/agent"].is_object());
        assert!(paths["/api/preferences"].is_object());
        assert!(paths["/api/user-api-keys"].is_object());
    }

    #[test]
    fn test_patch_preferences_documented() {
        let spec = generate_openapi_spec();
        assert!(spec["paths"]["/api/preferences"]["patch"].is_object());
    }
}
