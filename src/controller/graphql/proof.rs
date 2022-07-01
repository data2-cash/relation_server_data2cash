use aragog::DatabaseConnection;
use async_graphql::{Context, Object};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::graph::edge::Proof;
use crate::graph::vertex::IdentityRecord;
use crate::graph::Edge;
use crate::graph::{edge::proof::ProofRecord, vertex::Identity};

#[Object]
impl ProofRecord {
    async fn uuid(&self) -> String {
        self.uuid.to_string()
    }

    async fn source(&self) -> String {
        self.source.to_string()
    }

    async fn record_id(&self) -> Option<String> {
        self.record_id.clone()
    }

    async fn created_at(&self) -> Option<i64> {
        self.created_at.map(|ca| ca.timestamp())
    }

    async fn last_fetched_at(&self) -> i64 {
        self.last_fetched_at.timestamp()
    }

    async fn from(&self, ctx: &Context<'_>) -> Result<IdentityRecord> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let from_record: aragog::DatabaseRecord<Identity> = self.from_record(db).await?;

        Ok(from_record.into())
    }

    async fn to(&self, ctx: &Context<'_>) -> Result<IdentityRecord> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        let to_record: aragog::DatabaseRecord<Identity> = self.to_record(db).await?;

        Ok(to_record.into())
    }
}

/// Query entrypoint for `Proof{,Record}`
#[derive(Default)]
pub struct ProofQuery;

#[Object]
impl ProofQuery {
    async fn proof(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "UUID of this proof")] uuid: Option<String>,
    ) -> Result<Option<ProofRecord>> {
        let db: &DatabaseConnection = ctx.data().map_err(|err| Error::GraphQLError(err.message))?;
        if uuid.is_none() {
            return Ok(None);
        }
        let uuid = Uuid::parse_str(&uuid.unwrap())?;
        let found = Proof::find_by_uuid(db, &uuid).await?;

        Ok(found)
    }
}
