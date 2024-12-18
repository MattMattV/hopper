use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

use crate::model::AtUri;

pub const REL_LINK: &str = "http://hopper.at/rel/link";
pub const NS_COLLECTION: &str = "http://hopper.at/ns/collection";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Link {
    pub(crate) rel: String,
    pub(crate) template: Option<String>,

    #[serde(default)]
    pub(crate) properties: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct WebHostMeta {
    #[serde(default)]
    pub(crate) properties: HashMap<String, String>,

    #[serde(default)]
    pub(crate) links: Vec<Link>,
}

pub(crate) async fn query(http_client: &reqwest::Client, hostname: &str) -> Result<WebHostMeta> {
    let url = format!("https://{}/.well-known/host-meta.json", hostname,);

    let web_host_meta: WebHostMeta = http_client.get(url).send().await?.json().await?;

    Ok(web_host_meta)
}

impl Link {
    pub fn new(template: &str, collection: Option<&str>) -> Self {
        let properties = collection
            .map(|collection| HashMap::from([(NS_COLLECTION.to_string(), collection.to_string())]))
            .unwrap_or_default();
        Self {
            rel: REL_LINK.to_string(),
            template: Some(template.to_string()),
            properties,
        }
    }
}

impl WebHostMeta {
    pub fn new(links: Vec<Link>) -> Self {
        Self {
            properties: Default::default(),
            links,
        }
    }

    pub(crate) fn match_uri(&self, server: &str, aturi: &AtUri) -> Option<String> {
        tracing::debug!("matching uri: {:?}", aturi);
        let prefix = format!("https://{}", server);
        for link in &self.links {
            if link.rel != REL_LINK {
                continue;
            }

            if link.template.is_none() {
                tracing::debug!("template is empty");
                continue;
            }

            let template = link.template.as_ref().unwrap();

            if !template.starts_with(prefix.as_str()) {
                tracing::debug!("template does not match prefix {}", prefix);
                continue;
            }

            let matching_collection = aturi.collection.clone().unwrap_or("identity".to_string());
            let compare_collection = link
                .properties
                .get(NS_COLLECTION)
                .map(|value| value.to_string())
                .unwrap_or("identity".to_string());

            if compare_collection != matching_collection {
                continue;
            }

            let mut result = template.replace("{identity}", &aturi.identity);
            if let Some(collection) = &aturi.collection {
                result = result.replace("{collection}", collection);
            }
            if let Some(nsid) = &aturi.rkey {
                result = result.replace("{rkey}", nsid);
            }

            return Some(result);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Link, WebHostMeta};

    #[test]
    fn test_deserialize() {
        let webfinger = serde_json::from_str::<WebHostMeta>(
            r##"{
  "links": [
    {
      "rel": "http://hopper.at/rel/link",
      "template": "https://smokesignal.events/{identity}/{rkey}",
      "properties": {
         "https://hopper.at/spec/schema/1.0/link#collection": "events.smokesignal.calendar.event"
      }
    }
  ]
}"##,
        );
        println!("{:?}", webfinger);
        assert!(webfinger.is_ok());

        let webfinger = webfinger.unwrap();
        assert_eq!(webfinger.links.len(), 1);
    }

    #[test]
    fn test_match_uri() {
        let hostname = "smokesignal.events".to_string();
        let web_finger1 = WebHostMeta {
            links: vec![Link {
                rel: "http://hopper.at/rel/link".to_string(),
                template: Some("https://smokesignal.events/{identity}".to_string()),
                properties: HashMap::from([(
                    "https://hopper.at/spec/schema/1.0/link#collection".into(),
                    "identity".into(),
                )]),
            }],
            properties: Default::default(),
        };

        assert_eq!(
            web_finger1.match_uri(
                &hostname,
                &crate::model::AtUri {
                    identity: "ngerakines.me".to_string(),
                    collection: None,
                    rkey: None,
                }
            ),
            Some("https://smokesignal.events/ngerakines.me".into())
        );

        assert_eq!(
            web_finger1.match_uri(
                &hostname,
                &crate::model::AtUri {
                    identity: "smokesignal.events".to_string(),
                    collection: Some("event".into()),
                    rkey: Some("s0xnr5kqnp".into()),
                }
            ),
            None,
        );
    }
}
