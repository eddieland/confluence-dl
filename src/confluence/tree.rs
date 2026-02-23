//! Utilities for traversing Confluence page hierarchies.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use anyhow::{Result, anyhow};

use super::api::ConfluenceApi;
use super::models::Page;

/// Represents a page tree with hierarchical children.
#[derive(Debug, Clone)]
pub struct PageTree {
  /// Metadata and storage content for the page at this node.
  pub page: Page,
  /// Descendant pages nested under this node.
  pub children: Vec<PageTree>,
  /// Zero-based depth where `0` is the original root.
  pub depth: usize,
}

/// Build a page tree recursively from a root page.
///
/// This function traverses the page hierarchy starting from a root page,
/// downloading child pages up to the specified maximum depth.
///
/// # Arguments
/// * `client` - API implementation used for fetching page and child metadata.
/// * `page_id` - Identifier of the root page to use as the tree entry point.
/// * `max_depth` - Optional maximum depth; `None` fetches the entire hierarchy.
///
/// # Returns
/// A [`PageTree`] containing the root page and any fetched children.
///
/// # Errors
/// Returns an error if fetching the page tree encounters a failure, or if a
/// circular reference is detected.
pub async fn get_page_tree(client: &dyn ConfluenceApi, page_id: &str, max_depth: Option<usize>) -> Result<PageTree> {
  get_page_tree_recursive(client, page_id, 0, max_depth, &mut HashSet::new()).await
}

/// Recursive helper that builds the page tree while tracking visited nodes.
///
/// # Arguments
/// * `client` - API implementation used for fetching page data.
/// * `page_id` - Current page being processed.
/// * `current_depth` - Depth of the current page in the traversal.
/// * `max_depth` - Optional maximum depth; `None` fetches until pages are exhausted.
/// * `visited` - Set of page IDs already seen, used to detect cycles.
///
/// # Returns
/// A future that resolves to the [`PageTree`] for the provided page.
///
/// # Errors
/// Returns an error if a cycle is detected or if API calls fail.
fn get_page_tree_recursive<'a>(
  client: &'a dyn ConfluenceApi,
  page_id: &'a str,
  current_depth: usize,
  max_depth: Option<usize>,
  visited: &'a mut HashSet<String>,
) -> Pin<Box<dyn Future<Output = Result<PageTree>> + Send + 'a>> {
  Box::pin(async move {
    if visited.contains(page_id) {
      return Err(anyhow!("Circular reference detected: page {page_id} already visited"));
    }
    visited.insert(page_id.to_string());

    let page = client.get_page(page_id).await?;

    let children = if max_depth.is_none() || current_depth < max_depth.unwrap() {
      let child_pages = client.get_child_pages(page_id).await?;
      let mut child_trees = Vec::new();

      for child_page in child_pages {
        match get_page_tree_recursive(client, &child_page.id, current_depth + 1, max_depth, visited).await {
          Ok(child_tree) => child_trees.push(child_tree),
          Err(e) => eprintln!("Warning: Failed to fetch child page {}: {}", child_page.id, e),
        }
      }

      child_trees
    } else {
      Vec::new()
    };

    Ok(PageTree {
      page,
      children,
      depth: current_depth,
    })
  })
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::Path;

  use async_trait::async_trait;

  use super::*;
  use crate::confluence::models::{Attachment, PageBody, StorageFormat, UserInfo};

  /// A fake client with a configurable number of children per page,
  /// used to verify that `get_page_tree` works when the underlying
  /// `get_child_pages` returns many children (as would happen after
  /// pagination is resolved by the real client).
  struct ManyChildrenClient {
    pages: HashMap<String, Page>,
    children: HashMap<String, Vec<String>>,
  }

  impl ManyChildrenClient {
    fn new() -> Self {
      Self {
        pages: HashMap::new(),
        children: HashMap::new(),
      }
    }

    fn add_page(&mut self, id: &str, title: &str) {
      self.pages.insert(
        id.to_string(),
        Page {
          id: id.to_string(),
          title: title.to_string(),
          page_type: "page".to_string(),
          status: "current".to_string(),
          body: Some(PageBody {
            storage: Some(StorageFormat {
              value: "<p>content</p>".to_string(),
              representation: "storage".to_string(),
            }),
            view: None,
          }),
          space: None,
          links: None,
        },
      );
    }

    fn set_children(&mut self, parent_id: &str, child_ids: Vec<String>) {
      self.children.insert(parent_id.to_string(), child_ids);
    }
  }

  #[async_trait]
  impl ConfluenceApi for ManyChildrenClient {
    async fn get_page(&self, page_id: &str) -> Result<Page> {
      self
        .pages
        .get(page_id)
        .cloned()
        .ok_or_else(|| anyhow!("page not found: {page_id}"))
    }

    async fn get_child_pages(&self, page_id: &str) -> Result<Vec<Page>> {
      let ids = self.children.get(page_id).cloned().unwrap_or_default();
      let mut pages = Vec::new();
      for id in ids {
        if let Some(page) = self.pages.get(&id) {
          pages.push(page.clone());
        }
      }
      Ok(pages)
    }

    async fn get_attachments(&self, _page_id: &str) -> Result<Vec<Attachment>> {
      Ok(Vec::new())
    }

    async fn download_attachment(&self, _url: &str, _output_path: &Path) -> Result<()> {
      Ok(())
    }

    async fn fetch_attachment(&self, _url: &str) -> Result<Vec<u8>> {
      Ok(Vec::new())
    }

    async fn test_auth(&self) -> Result<UserInfo> {
      Ok(UserInfo {
        account_id: "test".to_string(),
        email: None,
        display_name: "Test".to_string(),
        public_name: None,
      })
    }
  }

  #[tokio::test]
  async fn get_page_tree_collects_many_children() {
    let mut client = ManyChildrenClient::new();
    client.add_page("root", "Root");

    // Simulate 30 children (more than the default Confluence page size of 25)
    let child_ids: Vec<String> = (0..30).map(|i| format!("child-{i}")).collect();
    for id in &child_ids {
      client.add_page(id, &format!("Child {id}"));
    }
    client.set_children("root", child_ids);

    let tree = get_page_tree(&client, "root", None).await.unwrap();
    assert_eq!(tree.children.len(), 30);
    assert_eq!(tree.page.title, "Root");
    assert_eq!(tree.depth, 0);

    // Verify all children are at depth 1
    for child in &tree.children {
      assert_eq!(child.depth, 1);
    }
  }

  #[tokio::test]
  async fn get_page_tree_respects_max_depth() {
    let mut client = ManyChildrenClient::new();
    client.add_page("root", "Root");
    client.add_page("child", "Child");
    client.add_page("grandchild", "Grandchild");
    client.set_children("root", vec!["child".to_string()]);
    client.set_children("child", vec!["grandchild".to_string()]);

    // Depth 0 should only return root, no children
    let tree = get_page_tree(&client, "root", Some(0)).await.unwrap();
    assert_eq!(tree.children.len(), 0);

    // Depth 1 should return root + child, but not grandchild
    let tree = get_page_tree(&client, "root", Some(1)).await.unwrap();
    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].children.len(), 0);

    // No limit should return all
    let tree = get_page_tree(&client, "root", None).await.unwrap();
    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].children.len(), 1);
  }

  #[tokio::test]
  async fn get_page_tree_detects_circular_reference() {
    let mut client = ManyChildrenClient::new();
    client.add_page("a", "Page A");
    client.add_page("b", "Page B");
    // A -> B -> A (cycle)
    client.set_children("a", vec!["b".to_string()]);
    client.set_children("b", vec!["a".to_string()]);

    // The tree builder should handle the cycle gracefully via the warning
    // (child page "a" will be skipped with a warning printed to stderr)
    let tree = get_page_tree(&client, "a", None).await.unwrap();
    assert_eq!(tree.children.len(), 1);
    // The grandchild "a" should not appear because it was already visited
    assert_eq!(tree.children[0].children.len(), 0);
  }
}
