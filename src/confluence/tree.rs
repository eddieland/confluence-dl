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
/// * `max_depth` - Optional maximum depth; `None` fetches until pages are
///   exhausted.
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
