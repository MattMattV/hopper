use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

use crate::model::AtUri;

pub(crate) const ERROR_SUBJECT_MISMATCH: &str = "error-webfinger-subject-mismatch The subject of the webfinger response does not match the requested acct";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Link {
    pub rel: String,
    pub href: String,

    #[serde(default)]
    pub properties: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Webfinger {
    pub subject: String,

    #[serde(default)]
    pub properties: HashMap<String, String>,

    #[serde(default)]
    pub links: Vec<Link>,
}

pub(crate) type QueryParam<'a> = (&'a str, &'a str);
pub(crate) type QueryParams<'a> = Vec<QueryParam<'a>>;

pub(crate) fn stringify(query: QueryParams) -> String {
    query.iter().fold(String::new(), |acc, &tuple| {
        acc + tuple.0 + "=" + tuple.1 + "&"
    })
}

pub(crate) async fn query(http_client: &reqwest::Client, hostname: &str) -> Result<Webfinger> {
    let acct = format!("acct:{}", hostname);
    let args = [(
        "resource".to_string(),
        urlencoding::encode(&acct).to_string(),
    )];

    let url = format!(
        "https://{}/.well-known/webfinger?{}",
        hostname,
        stringify(args.iter().map(|(k, v)| (&**k, &**v)).collect())
    );

    let webfinger: Webfinger = http_client.get(url).send().await?.json().await?;

    if webfinger.subject != acct {
        return Err(anyhow::anyhow!(ERROR_SUBJECT_MISMATCH));
    }

    Ok(webfinger)
}

impl Webfinger {
    pub(crate) fn match_uri(&self, server: &str, aturi: &AtUri) -> Option<String> {
        let prefix = format!("https://{}", server);
        for link in &self.links {
            if !link.href.starts_with(prefix.as_str()) {
                continue;
            }

            if link.rel != "https://hopper.at/spec/schema/1.0/link" {
                continue;
            }

            let matching_collection = aturi.collection.clone().unwrap_or("identity".to_string());
            if let Some(collection_value) = link
                .properties
                .get("https://hopper.at/spec/schema/1.0/link#collection")
            {
                if *collection_value != matching_collection {
                    continue;
                }
            }

            let template = link.href.clone();
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

    use super::{Link, Webfinger};

    #[test]
    fn test_deserialize() {
        let webfinger = serde_json::from_str::<Webfinger>(
            r##"{
  "subject": "acct:smokesignal.events",
  "aliases": [
    "https://smokesignal.events"
  ],
  "links": [
    {
      "rel": "https://hopper.at/spec/schema/1.0/link",
      "href": "https://smokesignal.events/{identity}/{rkey}",
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
        assert_eq!(webfinger.subject, "acct:smokesignal.events");
    }

    #[test]
    fn test_match_uri() {
        let hostname = "smokesignal.events".to_string();
        let web_finger1 = Webfinger {
            subject: "acct:smokesignal.events".to_string(),
            links: vec![Link {
                rel: "https://hopper.at/spec/schema/1.0/link".to_string(),
                href: "https://smokesignal.events/{identity}".to_string(),
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
