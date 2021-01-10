use select::document::Document;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebsiteMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
}

// Some websites like Twitter or Facebook has their own metatags format (Not OGP)
// Read more about The Open Graph protocol https://ogp.me
// Read more about meta tags https://css-tricks.com/essential-meta-tags-social-media/
pub async fn get_website_metadata(url: &Url) -> Result<WebsiteMetadata, reqwest::Error> {
    let res = reqwest::get(url.as_ref()).await?.text().await?;

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

    let metadata = WebsiteMetadata {
        title,
        description,
        thumbnail,
    };

    Ok(metadata)
}
