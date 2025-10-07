//! Test fixtures for Confluence API responses
//!
//! This module provides realistic sample data from the Confluence REST API
//! for use in tests.

use serde_json::json;

// Sample response for a basic Confluence page
pub fn sample_page_response() -> serde_json::Value {
  json!({
    "id": "123456",
    "type": "page",
    "status": "current",
    "title": "Getting Started Guide",
    "body": {
      "storage": {
        "value": "<h1>Getting Started</h1><p>Welcome to our documentation!</p><p>This guide will help you get started with our product.</p>",
        "representation": "storage"
      },
      "view": {
        "value": "<h1>Getting Started</h1><p>Welcome to our documentation!</p><p>This guide will help you get started with our product.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "DOCS",
      "name": "Documentation",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/DOCS/pages/123456/Getting+Started+Guide",
      "self": "https://example.atlassian.net/wiki/rest/api/content/123456"
    }
  })
}

// Sample response for a page with complex formatting
pub fn sample_complex_page_response() -> serde_json::Value {
  json!({
    "id": "789012",
    "type": "page",
    "status": "current",
    "title": "API Documentation",
    "body": {
      "storage": {
        "value": r#"<h1>API Documentation</h1>
<h2>Overview</h2>
<p>This API provides access to our services.</p>
<h2>Endpoints</h2>
<ul>
  <li><code>/api/users</code> - User management</li>
  <li><code>/api/posts</code> - Content management</li>
</ul>
<h2>Authentication</h2>
<p>Use <strong>API tokens</strong> for authentication.</p>
<ac:structured-macro ac:name="code">
  <ac:plain-text-body><![CDATA[curl -H "Authorization: Bearer TOKEN" https://api.example.com/users]]></ac:plain-text-body>
</ac:structured-macro>
<h2>Code Examples</h2>
<ac:structured-macro ac:name="code" ac:schema-version="1">
  <ac:parameter ac:name="language">python</ac:parameter>
  <ac:plain-text-body><![CDATA[import requests

def get_users():
    response = requests.get('https://api.example.com/users')
    return response.json()]]></ac:plain-text-body>
</ac:structured-macro>"#,
        "representation": "storage"
      },
      "view": {
        "value": "<h1>API Documentation</h1><h2>Overview</h2><p>This API provides access to our services.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "DEV",
      "name": "Developer Portal",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/DEV/pages/789012/API+Documentation",
      "self": "https://example.atlassian.net/wiki/rest/api/content/789012"
    }
  })
}

// Sample response for a page with internal links
pub fn sample_page_with_links_response() -> serde_json::Value {
  json!({
    "id": "345678",
    "type": "page",
    "status": "current",
    "title": "Installation Guide",
    "body": {
      "storage": {
        "value": r#"<h1>Installation</h1>
<p>See the <ac:link><ri:page ri:content-title="Getting Started Guide" /></ac:link> for prerequisites.</p>
<p>For API details, check <ac:link><ri:page ri:content-title="API Documentation" /></ac:link>.</p>
<h2>Steps</h2>
<ol>
  <li>Download the installer</li>
  <li>Run the setup wizard</li>
  <li>Configure your settings</li>
</ol>"#,
        "representation": "storage"
      },
      "view": {
        "value": "<h1>Installation</h1><p>See the Getting Started Guide for prerequisites.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "DOCS",
      "name": "Documentation",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/DOCS/pages/345678/Installation+Guide",
      "self": "https://example.atlassian.net/wiki/rest/api/content/345678"
    }
  })
}

// Sample response for current user endpoint
#[allow(dead_code)]
pub fn sample_current_user_response() -> serde_json::Value {
  json!({
    "type": "known",
    "username": "testuser",
    "userKey": "8a7f808f7e4c8e7f017e4c8e8f0001",
    "accountId": "5b10ac8d82e05b22cc7d4ef5",
    "displayName": "Test User",
    "email": "testuser@example.com"
  })
}

// Sample error response for authentication failure
#[allow(dead_code)]
pub fn sample_auth_error_response() -> serde_json::Value {
  json!({
    "statusCode": 401,
    "message": "Client must be authenticated to access this resource."
  })
}

// Sample error response for page not found
#[allow(dead_code)]
pub fn sample_not_found_response() -> serde_json::Value {
  json!({
    "statusCode": 404,
    "message": "No content found with id: 999999"
  })
}

// Sample response for a personal space page
pub fn sample_personal_space_page_response() -> serde_json::Value {
  json!({
    "id": "229483",
    "type": "page",
    "status": "current",
    "title": "Getting started in Confluence from Jira",
    "body": {
      "storage": {
        "value": "<p>This page was created from Jira.</p>",
        "representation": "storage"
      },
      "view": {
        "value": "<p>This page was created from Jira.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "~6320c26429083bbe8cc369b0",
      "name": "Edward Jones",
      "type": "personal"
    },
    "_links": {
      "webui": "/wiki/spaces/~6320c26429083bbe8cc369b0/pages/229483/Getting+started+in+Confluence+from+Jira",
      "self": "https://eddieland.atlassian.net/wiki/rest/api/content/229483"
    }
  })
}

// Sample response for a page with images
pub fn sample_page_with_images_response() -> serde_json::Value {
  json!({
    "id": "456789",
    "type": "page",
    "status": "current",
    "title": "Architecture Diagram",
    "body": {
      "storage": {
        "value": r#"<h1>System Architecture</h1>
<p>Here's our high-level architecture:</p>
<ac:image ac:height="400">
  <ri:attachment ri:filename="architecture.png" />
</ac:image>
<p>The diagram shows three main components:</p>
<ul>
  <li>Frontend (React)</li>
  <li>Backend (Node.js)</li>
  <li>Database (PostgreSQL)</li>
</ul>"#,
        "representation": "storage"
      },
      "view": {
        "value": "<h1>System Architecture</h1><p>Here's our high-level architecture:</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "ARCH",
      "name": "Architecture",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/ARCH/pages/456789/Architecture+Diagram",
      "self": "https://example.atlassian.net/wiki/rest/api/content/456789"
    }
  })
}
