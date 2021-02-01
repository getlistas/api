use select::document::Document;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WebsiteMetadata {
    pub can_resolve: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
}

// Some websites like Twitter or Facebook has their own metatags format (Not OGP)
// Read more about The Open Graph protocol https://ogp.me
// Read more about meta tags https://css-tricks.com/essential-meta-tags-social-media/
pub async fn get_website_metadata(url: &Url) -> WebsiteMetadata {
    let res = match reqwest::get(url.as_ref()).await {
        Ok(res) => res,
        Err(_) => return WebsiteMetadata::default(),
    };

    let website = match res.text().await {
        Ok(website) => website,
        Err(_) => return WebsiteMetadata::default(),
    };

    let document = Document::from(website.as_str());

    let tag_title = document
        .find(select::predicate::Name("title"))
        .next()
        .map(|node| node.text());

    let metas = document
        .find(select::predicate::Name("meta"))
        .filter(|node| node.attr("property").is_some())
        .collect::<Vec<select::node::Node>>();

    let mut meta_title: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:title")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let mut meta_description: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:description")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    let mut meta_thumbnail: Option<String> = metas
        .iter()
        .find(|meta| meta.attr("property").unwrap() == "og:image")
        .and_then(|meta| meta.attr("content"))
        .map(|value| value.to_owned());

    // Fallback 1 if our basic scrapping was not successful.
    if meta_title.is_none() || meta_description.is_none() || meta_thumbnail.is_none() {
        let linkish_metadata = get_metadata_from_linkish(url).await;
        if linkish_metadata.is_ok() {
            let linkish_metadata = linkish_metadata.unwrap();
            meta_title = meta_title.or(linkish_metadata.title);
            meta_description = meta_description.or(linkish_metadata.description);
            meta_thumbnail = meta_thumbnail.or(linkish_metadata.image);
        }
    }

    WebsiteMetadata {
        can_resolve: true,
        title: meta_title.or(tag_title),
        description: meta_description,
        thumbnail: meta_thumbnail,
    }
}

#[derive(Deserialize, Debug)]
struct LinkishResponse {
    title: Option<String>,
    description: Option<String>,
    image: Option<String>,
}

async fn get_metadata_from_linkish(url: &Url) -> Result<LinkishResponse, reqwest::Error> {
    let mut map = HashMap::new();
    map.insert("link", url.as_ref());

    let client = reqwest::Client::new();
    client
        .post("https://api.linkish.io/scrape-link")
        .json(&map)
        .send()
        .await?
        .json::<LinkishResponse>()
        .await
}
