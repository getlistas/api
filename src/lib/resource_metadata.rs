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

    let tag_title = document
        .find(select::predicate::Name("title"))
        .next()
        .map(|node| node.text());

    let metas = document
        .find(select::predicate::Name("meta"))
        .filter(|node| node.attr("property").is_some())
        .collect::<Vec<select::node::Node>>();

    let meta_title: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:title")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let meta_description: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:description")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let meta_thumbnail: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:image")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let metadata = WebsiteMetadata {
        title: meta_title.or(tag_title),
        description: meta_description,
        thumbnail: meta_thumbnail,
    };

    Ok(metadata)
}
