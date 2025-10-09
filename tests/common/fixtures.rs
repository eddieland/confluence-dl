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
      "key": "~example-user",
      "name": "Example User",
      "type": "personal"
    },
    "_links": {
      "webui": "/wiki/spaces/~example-user/pages/229483/Getting+started+in+Confluence+from+Jira",
      "self": "https://example.atlassian.net/wiki/rest/api/content/229483"
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

// Sample response for a page with a file attachment link
pub fn sample_page_with_attachment_response() -> serde_json::Value {
  json!({
    "id": "654321",
    "type": "page",
    "status": "current",
    "title": "Project Resources",
    "body": {
      "storage": {
        "value": r#"<p>Download the latest resources:</p>
<ac:link>
  <ri:attachment ri:filename="project-plan.pdf" />
  <ac:plain-text-link-body>Project Plan</ac:plain-text-link-body>
</ac:link>
"#,
        "representation": "storage"
      },
      "view": {
        "value": "<p>Download the latest resources.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "PROJ",
      "name": "Project Space",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/PROJ/pages/654321/Project+Resources",
      "self": "https://example.atlassian.net/wiki/rest/api/content/654321"
    }
  })
}

pub fn sample_page_with_jira_macro_response() -> serde_json::Value {
  json!({
    "id": "112233",
    "type": "page",
    "status": "current",
    "title": "Jira Integration Overview",
    "body": {
      "storage": {
        "value": r#"<h1>Jira Integration</h1>
<p>Tracked issue:</p>
<ac:structured-macro ac:name="jira">
  <ac:parameter ac:name="key">ABC-123</ac:parameter>
  <ac:parameter ac:name="baseurl">https://jira.example.com/</ac:parameter>
  <ac:parameter ac:name="summary">Investigate login regression</ac:parameter>
</ac:structured-macro>
<p>Recent issues:</p>
<ac:structured-macro ac:name="jira">
  <ac:parameter ac:name="jql">project = ABC ORDER BY created DESC</ac:parameter>
</ac:structured-macro>
"#,
        "representation": "storage"
      },
      "view": {
        "value": "<h1>Jira Integration</h1><p>Tracked issue:</p><p>Recent issues:</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "ENG",
      "name": "Engineering",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/ENG/pages/112233/Jira+Integration+Overview",
      "self": "https://example.atlassian.net/wiki/rest/api/content/112233"
    }
  })
}

pub fn sample_page_with_column_layout_response() -> serde_json::Value {
  json!({
    "id": "223344",
    "type": "page",
    "status": "current",
    "title": "Team Responsibilities",
    "body": {
      "storage": {
        "value": r#"<h1>Team Responsibilities</h1>
<ac:layout>
  <ac:layout-section>
    <ac:layout-cell>
      <p><strong>Frontend</strong>: Owns the web experience.</p>
    </ac:layout-cell>
    <ac:layout-cell>
      <p><strong>Backend</strong>: Maintains APIs and services.</p>
    </ac:layout-cell>
  </ac:layout-section>
  <ac:layout-section>
    <ac:layout-cell>
      <p><strong>DevOps</strong>: Ensures reliable deployments.</p>
    </ac:layout-cell>
    <ac:layout-cell>
      <p><strong>QA</strong>: Guards product quality.</p>
    </ac:layout-cell>
  </ac:layout-section>
</ac:layout>
"#,
        "representation": "storage"
      },
      "view": {
        "value": "<h1>Team Responsibilities</h1>",
        "representation": "view"
      }
    },
    "space": {
      "key": "OPS",
      "name": "Operations",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/OPS/pages/223344/Team+Responsibilities",
      "self": "https://example.atlassian.net/wiki/rest/api/content/223344"
    }
  })
}

// Sample response for child pages
pub fn sample_child_page_1_response() -> serde_json::Value {
  json!({
    "id": "111111",
    "type": "page",
    "status": "current",
    "title": "Child Page 1",
    "body": {
      "storage": {
        "value": "<h1>Child Page 1</h1><p>This is the first child page.</p>",
        "representation": "storage"
      },
      "view": {
        "value": "<h1>Child Page 1</h1><p>This is the first child page.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "DOCS",
      "name": "Documentation",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/DOCS/pages/111111/Child+Page+1",
      "self": "https://example.atlassian.net/wiki/rest/api/content/111111"
    }
  })
}

pub fn sample_child_page_2_response() -> serde_json::Value {
  json!({
    "id": "222222",
    "type": "page",
    "status": "current",
    "title": "Child Page 2",
    "body": {
      "storage": {
        "value": "<h1>Child Page 2</h1><p>This is the second child page.</p>",
        "representation": "storage"
      },
      "view": {
        "value": "<h1>Child Page 2</h1><p>This is the second child page.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "DOCS",
      "name": "Documentation",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/DOCS/pages/222222/Child+Page+2",
      "self": "https://example.atlassian.net/wiki/rest/api/content/222222"
    }
  })
}

pub fn sample_grandchild_page_response() -> serde_json::Value {
  json!({
    "id": "333333",
    "type": "page",
    "status": "current",
    "title": "Grandchild Page",
    "body": {
      "storage": {
        "value": "<h1>Grandchild Page</h1><p>This is a grandchild page (child of Child Page 1).</p>",
        "representation": "storage"
      },
      "view": {
        "value": "<h1>Grandchild Page</h1><p>This is a grandchild page.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "DOCS",
      "name": "Documentation",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/DOCS/pages/333333/Grandchild+Page",
      "self": "https://example.atlassian.net/wiki/rest/api/content/333333"
    }
  })
}

// Sample response for a meeting notes overview page with macros
pub fn sample_meeting_notes_overview_response() -> serde_json::Value {
  let storage_value = concat!(
    "<p style=\"text-align: right;\"><ac:macro ac:name=\"create-from-template\">",
    "<ac:parameter ac:name=\"contentBlueprintId\">484f8c9d-447d-43cb-b290-33a51cb87d67</ac:parameter>",
    "<ac:parameter ac:name=\"blueprintModuleCompleteKey\">com.atlassian.confluence.plugins.confluence-business-blueprints:meeting-notes-blueprint</ac:parameter>",
    "<ac:parameter ac:name=\"createButtonLabel\">Create meeting note</ac:parameter>",
    "</ac:macro></p>",
    "<h2>Incomplete tasks from meetings</h2>",
    "<p><ac:macro ac:name=\"tasks-report-macro\">",
    "<ac:parameter ac:name=\"spaces\">~example-user</ac:parameter>",
    "<ac:parameter ac:name=\"pageSize\">10</ac:parameter>",
    "<ac:parameter ac:name=\"status\">incomplete</ac:parameter>",
    "<ac:parameter ac:name=\"labels\">meeting-notes</ac:parameter>",
    "</ac:macro></p>",
    "<h2>Decisions from meetings</h2>",
    "<p><ac:macro ac:name=\"decisionreport\">",
    "<ac:parameter ac:name=\"cql\">space = \"~example-user\" and label = \"meeting-notes\"</ac:parameter>",
    "</ac:macro></p>",
    "<h2>All meeting notes</h2>",
    "<p><ac:macro ac:name=\"content-report-table\">",
    "<ac:parameter ac:name=\"contentBlueprintId\">484f8c9d-447d-43cb-b290-33a51cb87d67</ac:parameter>",
    "<ac:parameter ac:name=\"analyticsKey\">meeting-notes</ac:parameter>",
    "<ac:parameter ac:name=\"spaces\">~example-user</ac:parameter>",
    "<ac:parameter ac:name=\"createButtonLabel\">Create meeting note</ac:parameter>",
    "<ac:parameter ac:name=\"labels\">meeting-notes</ac:parameter>",
    "</ac:macro></p>"
  );

  json!({
    "id": "998877",
    "type": "page",
    "status": "current",
    "title": "Meeting notes in space",
    "body": {
      "storage": {
        "value": storage_value,
        "representation": "storage"
      },
      "view": {
        "value": "<h2>Incomplete tasks from meetings</h2><h2>Decisions from meetings</h2><h2>All meeting notes</h2>",
        "representation": "view"
      }
    },
    "space": {
      "key": "~example-user",
      "name": "Example User",
      "type": "personal"
    },
    "_links": {
      "webui": "/wiki/spaces/~example-user/pages/998877/Meeting+notes+in+space",
      "self": "https://example.atlassian.net/wiki/rest/api/content/998877"
    }
  })
}

// Sample response for a meeting notes page with tasks, tables, and emoticons
pub fn sample_meeting_notes_with_tasks_response() -> serde_json::Value {
  let storage_value = concat!(
    "<h2><ac:emoticon ac:name=\"blue-star\" ac:emoji-id=\"1f5d3\" />&nbsp;Date</h2>",
    "<p><time datetime=\"2025-10-07\" /></p>",
    "<h2><ac:emoticon ac:name=\"blue-star\" ac:emoji-id=\"1f465\" />&nbsp;Participants</h2>",
    "<ul><li><p><ac:link><ri:user ri:account-id=\"example-account-id\" /></ac:link></p></li></ul>",
    "<h2><ac:emoticon ac:name=\"blue-star\" ac:emoji-id=\"1f945\" />&nbsp;Goals</h2>",
    "<p><ac:placeholder>List goals for this meeting</ac:placeholder></p>",
    "<h2><ac:emoticon ac:name=\"blue-star\" ac:emoji-id=\"1f5e3\" />&nbsp;Discussion topics</h2>",
    "<table data-table-width=\"760\"><tbody>",
    "<tr><th><p><strong>Time</strong></p></th><th><p><strong>Topic</strong></p></th></tr>",
    "<tr><td><p>10:00</p></td><td><p>Project update</p></td></tr>",
    "</tbody></table>",
    "<h2><ac:emoticon ac:name=\"blue-star\" ac:emoji-id=\"2705\" />&nbsp;Action items</h2>",
    "<ac:task-list>",
    "<ac:task><ac:task-id>3</ac:task-id>",
    "<ac:task-status>incomplete</ac:task-status>",
    "<ac:task-body>Review architecture proposal</ac:task-body></ac:task>",
    "<ac:task><ac:task-id>4</ac:task-id>",
    "<ac:task-status>complete</ac:task-status>",
    "<ac:task-body>Update documentation</ac:task-body></ac:task>",
    "</ac:task-list>"
  );

  json!({
    "id": "887766",
    "type": "page",
    "status": "current",
    "title": "2025-10-07 Meeting notes",
    "body": {
      "storage": {
        "value": storage_value,
        "representation": "storage"
      },
      "view": {
        "value": "<h2>Date</h2><p>2025-10-07</p><h2>Participants</h2><h2>Goals</h2><h2>Discussion topics</h2><h2>Action items</h2>",
        "representation": "view"
      }
    },
    "space": {
      "key": "~example-user",
      "name": "Example User",
      "type": "personal"
    },
    "_links": {
      "webui": "/wiki/spaces/~example-user/pages/887766/2025-10-07+Meeting+notes",
      "self": "https://example.atlassian.net/wiki/rest/api/content/887766"
    }
  })
}

// Sample response for a comprehensive test page with all XML features
pub fn sample_comprehensive_features_response() -> serde_json::Value {
  let storage_value = concat!(
    "<h1>Comprehensive Test Page</h1>",
    "<h2>Basic Formatting</h2>",
    "<p>This page contains <strong>bold</strong>, <em>italic</em>, and <u>underlined</u> text.</p>",
    "<p>It also has <code>inline code</code> and <a href=\"https://example.com\">external links</a>.</p>",
    "<h2>Lists</h2>",
    "<ul><li>Unordered item 1</li><li>Unordered item 2<ul><li>Nested item</li></ul></li></ul>",
    "<ol><li>Ordered item 1</li><li>Ordered item 2</li></ol>",
    "<h2>Emoticons and Icons</h2>",
    "<p><ac:emoticon ac:name=\"tick\" ac:emoji-id=\"2705\" /> Success indicator</p>",
    "<p><ac:emoticon ac:name=\"cross\" ac:emoji-id=\"274c\" /> Failure indicator</p>",
    "<h2>Internal Links</h2>",
    "<p>See <ac:link><ri:page ri:content-title=\"Getting Started Guide\" /></ac:link> for more information.</p>",
    "<h2>User Mentions</h2>",
    "<p>Assigned to <ac:link><ri:user ri:account-id=\"example-account-id\" /></ac:link> for review.</p>",
    "<h2>Task Lists</h2>",
    "<ac:task-list>",
    "<ac:task><ac:task-id>1</ac:task-id><ac:task-status>complete</ac:task-status><ac:task-body>Complete this task</ac:task-body></ac:task>",
    "<ac:task><ac:task-id>2</ac:task-id><ac:task-status>incomplete</ac:task-status><ac:task-body>Pending task</ac:task-body></ac:task>",
    "</ac:task-list>",
    "<h2>Tables</h2>",
    "<table data-table-width=\"760\"><tbody>",
    "<tr><th><p><strong>Header 1</strong></p></th><th><p><strong>Header 2</strong></p></th></tr>",
    "<tr><td><p>Cell 1</p></td><td><p>Cell 2</p></td></tr>",
    "</tbody></table>",
    "<h2>Code Blocks</h2>",
    "<ac:structured-macro ac:name=\"code\"><ac:parameter ac:name=\"language\">javascript</ac:parameter>",
    "<ac:plain-text-body><![CDATA[function greet(name) { console.log(name); }]]></ac:plain-text-body>",
    "</ac:structured-macro>",
    "<h2>Images</h2>",
    "<ac:image ac:height=\"300\"><ri:attachment ri:filename=\"diagram.png\" /></ac:image>",
    "<h2>Placeholders</h2>",
    "<p><ac:placeholder>This is a placeholder for future content</ac:placeholder></p>",
    "<h2>Time Elements</h2>",
    "<p>Meeting date: <time datetime=\"2025-10-07\" /></p>"
  );

  json!({
    "id": "776655",
    "type": "page",
    "status": "current",
    "title": "Comprehensive Feature Test Page",
    "body": {
      "storage": {
        "value": storage_value,
        "representation": "storage"
      },
      "view": {
        "value": "<h1>Comprehensive Test Page</h1><h2>Basic Formatting</h2><p>This page contains bold, italic, and underlined text.</p>",
        "representation": "view"
      }
    },
    "space": {
      "key": "TEST",
      "name": "Test Space",
      "type": "global"
    },
    "_links": {
      "webui": "/wiki/spaces/TEST/pages/776655/Comprehensive+Feature+Test+Page",
      "self": "https://example.atlassian.net/wiki/rest/api/content/776655"
    }
  })
}
