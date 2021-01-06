use select::document::Document;
use serde::{Deserialize, Serialize};
use url::Url;

// Read more about The Open Graph protocol at https://ogp.me
#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    title: Option<String>,
    description: Option<String>,
    image: Option<String>,
}

// Some websites like Twitter or Facebook has their own metatags format (Not OGP)
// Read more at: https://css-tricks.com/essential-meta-tags-social-media/
pub async fn get_website_ogp_metadata(url: &Url) -> Result<Metadata, reqwest::Error> {
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

    let image: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:image")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let metadata = Metadata {
        title,
        description,
        image,
    };

    Ok(metadata)
}
