mod tests;

use crate::error::Error;
use crate::graph::{Vertex, Edge};
use serde::Deserialize;
use crate::util::{naive_now, make_client, parse_body};
use async_trait::async_trait;
use crate::upstream::{Fetcher, Platform, DataSource, Connection};
use crate::graph::{vertex::Identity, edge::Proof, new_db_connection};

use uuid::Uuid;
use std::str::FromStr;


#[derive(Deserialize, Debug)]
pub struct KeybaseResponse {
    pub status: Status,
    pub them: Vec<PersonInfo>,
}

#[derive(Deserialize, Debug)]
pub struct PersonInfo {
    pub id: String,
    pub basics: Basics,
    pub proofs_summary: ProofsSummary,
}

#[derive(Deserialize, Debug)]
pub struct  Status {
    pub code: i32,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ProofsSummary {
    pub all: Vec<ProofItem>,
}

#[derive(Deserialize, Debug)]
pub struct Basics {
    pub username: String,
    pub ctime: i64,
    pub mtime: i64,
    pub id_version: i32,
    pub track_version: i32,
    pub last_id_change: i64,
    pub username_cased: String,
    pub status: i32,
    pub salt: String,
    pub eldest_seqno: i32,
}

#[derive(Deserialize, Debug)]
pub struct ProofItem {
    pub proof_type: String,
    pub nametag: String,
    pub state: i32,
    pub service_url: String,
    pub proof_url: String,
    pub sig_id: String,
    pub proof_id: String,
    pub human_url: String,
    pub presentation_group: String,
    pub presentation_tag: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
}

pub struct Keybase {
    pub platform: String,
    pub identity: String,
}

#[async_trait]
impl Fetcher for Keybase {
    async fn fetch(&self, _url: Option<String>) -> Result<Vec<Connection>, Error> { 
        let client = make_client();
        let uri: http::Uri = match format!("https://keybase.io/_/api/1.0/user/lookup.json?{}={}&fields=proofs_summary", self.platform, self.identity).parse() {
            Ok(n) => n,
            Err(err) => return Err(Error::ParamError(
                format!("Uri format Error: {}", err.to_string()))),
        };
  
        let mut resp = client.get(uri).await?;

        if !resp.status().is_success() {
            let body: ErrorResponse = parse_body(&mut resp).await?;
            return Err(Error::General(
                format!("Keybase Result Get Error: {}", body.message),
                resp.status(),
            ));
        }

        let mut body: KeybaseResponse = parse_body(&mut resp).await?;  
        if body.status.code != 0 {
            return Err(Error::General(
                format!("Keybase Result Get Error: {}", body.status.name),
                resp.status(),
            ));   
        }

        let person_info = match body.them.pop() {
            Some(i) => i,
            None => {
                return Err(Error::NoResult); 
            }
        };
        let user_id = person_info.id; 
        let user_name = person_info.basics.username;

        let db = new_db_connection().await?;

        let mut res = Vec::new();
        for p in person_info.proofs_summary.all.into_iter() {
            let from: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::Keybase,
                identity: user_id.clone(),
                created_at: None,
                display_name: user_name.clone(),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };
            let from_record = from.create_or_update(&db).await?;

            if Platform::from_str(p.proof_type.as_str()).is_err() {
                continue;
            }
            let to: Identity = Identity {
                uuid: Some(Uuid::new_v4()),
                platform: Platform::from_str(p.proof_type.as_str()).unwrap(),
                identity: p.nametag.clone(),
                created_at: None,
                display_name: p.nametag.clone(),
                added_at: naive_now(),
                avatar_url: None,
                profile_url: None,
                updated_at: naive_now(),
            };
            let to_record = to.create_or_update(&db).await?;

            let pf: Proof = Proof {
                uuid: Uuid::new_v4(),
                source: DataSource::Keybase,
                record_id: Some(p.proof_id.clone()),
                created_at: Some(naive_now()), 
                last_fetched_at: naive_now(),
            };
            pf.connect(&db, &from_record, &to_record).await?;
            let cnn: Connection = Connection {
                from: from,
                to: to,
                proof: pf,
            };
            res.push(cnn);    
        }

        Ok(res)
    }
}