use super::schema::oauth;
use crate::{
    db::{models::oauth::OAuthMeta, repository::oauth::OAuthRepository, RepoAdapterError},
    services::oauth::{OAuthProvider, TokenResponse},
};
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::{ExpressionMethods, Insertable, QueryDsl, RunQueryDsl};
use hextacy::driver::db::postgres::diesel::DieselConnection;
use std::fmt::Debug;

#[derive(Debug, Insertable)]
#[diesel(table_name = oauth)]
struct NewOAuthEntry<'a> {
    user_id: &'a str,
    access_token: &'a str,
    refresh_token: Option<&'a str>,
    provider: OAuthProvider,
    account_id: &'a str,
    scope: &'a str,
    expires_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct PgOAuthAdapter;

#[async_trait]
impl OAuthRepository<DieselConnection> for PgOAuthAdapter {
    async fn create<T>(
        conn: &mut DieselConnection,
        user_id: &str,
        account_id: &str,
        tokens: &T,
        provider: OAuthProvider,
    ) -> Result<OAuthMeta, RepoAdapterError>
    where
        T: TokenResponse + Send + Sync,
    {
        use super::schema::oauth::dsl;

        let entry = NewOAuthEntry {
            user_id,
            access_token: tokens.access_token(),
            refresh_token: tokens.refresh_token(),
            provider,
            account_id,
            scope: tokens.scope(),
            expires_at: tokens
                .expires_in()
                .map(|val| (Utc::now() + Duration::seconds(val)).naive_utc()),
        };

        diesel::insert_into(dsl::oauth)
            .values(entry)
            .get_result::<OAuthMeta>(conn)
            .map_err(RepoAdapterError::from)
    }

    async fn get_by_id(
        conn: &mut DieselConnection,
        id: &str,
    ) -> Result<OAuthMeta, RepoAdapterError> {
        use super::schema::oauth::dsl;

        dsl::oauth
            .filter(dsl::id.eq(id))
            .filter(dsl::revoked.eq(false))
            .filter(dsl::expires_at.gt(Utc::now()))
            .first::<OAuthMeta>(conn)
            .map_err(RepoAdapterError::from)
    }

    async fn get_by_account_id(
        conn: &mut DieselConnection,
        account_id: &str,
    ) -> Result<OAuthMeta, RepoAdapterError> {
        use super::schema::oauth::dsl;

        dsl::oauth
            .filter(dsl::account_id.eq(account_id))
            .filter(dsl::revoked.eq(false))
            .filter(dsl::expires_at.gt(Utc::now()))
            .first::<OAuthMeta>(conn)
            .map_err(RepoAdapterError::from)
    }

    async fn get_by_user_id(
        conn: &mut DieselConnection,
        user_id: &str,
    ) -> Result<Vec<OAuthMeta>, RepoAdapterError> {
        use super::schema::oauth::dsl;

        dsl::oauth
            .filter(dsl::user_id.eq(user_id))
            .filter(dsl::revoked.eq(false))
            .filter(dsl::expires_at.gt(Utc::now()))
            .load::<OAuthMeta>(conn)
            .map_err(RepoAdapterError::from)
    }

    async fn get_by_provider(
        conn: &mut DieselConnection,
        user_id: &str,
        provider: OAuthProvider,
    ) -> Result<OAuthMeta, RepoAdapterError> {
        use super::schema::oauth::dsl;

        dsl::oauth
            .filter(dsl::provider.eq(provider))
            .filter(dsl::user_id.eq(user_id))
            .filter(dsl::revoked.eq(false))
            .filter(dsl::expires_at.gt(Utc::now()))
            .first::<OAuthMeta>(conn)
            .map_err(RepoAdapterError::from)
    }

    async fn revoke(
        conn: &mut DieselConnection,
        access_token: &str,
    ) -> Result<OAuthMeta, RepoAdapterError> {
        use super::schema::oauth::dsl;

        diesel::update(dsl::oauth)
            .filter(dsl::access_token.eq(access_token))
            .set(dsl::revoked.eq(true))
            .load::<OAuthMeta>(conn)?
            .pop()
            .ok_or_else(|| RepoAdapterError::DoesNotExist)
    }

    async fn revoke_all(
        conn: &mut DieselConnection,
        user_id: &str,
    ) -> Result<Vec<OAuthMeta>, RepoAdapterError> {
        use super::schema::oauth::dsl;

        diesel::update(dsl::oauth)
            .filter(dsl::user_id.eq(user_id))
            .set(dsl::revoked.eq(true))
            .load::<OAuthMeta>(conn)
            .map_err(RepoAdapterError::from)
    }

    async fn update<T>(
        conn: &mut DieselConnection,
        user_id: &str,
        tokens: &T,
        provider: OAuthProvider,
    ) -> Result<OAuthMeta, RepoAdapterError>
    where
        T: TokenResponse,
    {
        use super::schema::oauth::dsl;

        diesel::update(dsl::oauth)
            .filter(dsl::user_id.eq(user_id))
            .filter(dsl::provider.eq(provider))
            .set((
                dsl::access_token.eq(tokens.access_token()),
                dsl::refresh_token.eq(tokens.refresh_token()),
                dsl::expires_at.eq(tokens
                    .expires_in()
                    .map(|val| (Utc::now() + Duration::seconds(val)).naive_utc())),
                dsl::scope.eq(tokens.scope()),
            ))
            .load::<OAuthMeta>(conn)?
            .pop()
            .ok_or_else(|| RepoAdapterError::DoesNotExist)
    }
}
