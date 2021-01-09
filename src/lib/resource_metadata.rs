use select::document::Document;
use serde::{Deserialize, Serialize};
use url::Url;

// Read more about The Open Graph protocol at https://ogp.me
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceMetadata {
    can_resolve: bool,
    title: Option<String>,
    description: Option<String>,
    thumbnail: Option<String>,
}

impl ResourceMetadata {
    fn can_not_resolve() -> Self {
        Self {
            can_resolve: false,
            title: None,
            description: None,
            thumbnail: None,
        }
    }
}

// Some websites like Twitter or Facebook has their own metatags format (Not OGP)
// Read more at: https://css-tricks.com/essential-meta-tags-social-media/
pub async fn get_website_ogp_metadata(url: &Url) -> Result<ResourceMetadata, reqwest::Error> {
    let res = match request(url).await {
        Ok(res) => res,
        // TODO: Handle Listas.io send request errors.
        Err(_) => return Ok(ResourceMetadata::can_not_resolve()),
    };

    let document = Document::from(res.as_str());
    let metas = document
        .find(select::predicate::Name("meta"))
        .filter(|node| node.attr("property").is_some())
        .collect::<Vec<select::node::Node>>();

    let title: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:title")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let description: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:description")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let thumbnail: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:image")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let metadata = ResourceMetadata {
        can_resolve: true,
        title,
        description,
        thumbnail,
    };

    Ok(metadata)
}

async fn request(url: &Url) -> Result<String, reqwest::Error> {
    reqwest::get(url.as_ref()).await?.text().await
}
